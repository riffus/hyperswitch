use api_models::admin as admin_types;
use common_utils::{
    consts::{
        DEFAULT_BACKGROUND_COLOR, DEFAULT_MERCHANT_LOGO, DEFAULT_PAYMENT_LINK_EXPIRY,
        DEFAULT_PRODUCT_IMG, DEFAULT_SDK_THEME,
    },
    ext_traits::{OptionExt, ValueExt},
};
use error_stack::{IntoReport, ResultExt};
use futures::future;
use masking::{PeekInterface, Secret};
use time::PrimitiveDateTime;

use super::errors::{self, RouterResult, StorageErrorExt};
use crate::{
    core::payments::helpers,
    errors::RouterResponse,
    routes::AppState,
    services,
    types::{
        api::payment_link::PaymentLinkResponseExt, domain, storage::enums as storage_enums,
        transformers::ForeignFrom,
    },
};

pub async fn retrieve_payment_link(
    state: AppState,
    payment_link_id: String,
) -> RouterResponse<api_models::payments::RetrievePaymentLinkResponse> {
    let db = &*state.store;
    let payment_link_config = db
        .find_payment_link_by_payment_link_id(&payment_link_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentLinkNotFound)?;

    let status = check_payment_link_status(payment_link_config.max_age);

    let response = api_models::payments::RetrievePaymentLinkResponse::foreign_from((
        payment_link_config,
        status,
    ));
    Ok(services::ApplicationResponse::Json(response))
}

pub async fn intiate_payment_link_flow(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    merchant_id: String,
    payment_id: String,
) -> RouterResponse<services::PaymentLinkFormData> {
    let db = &*state.store;
    let payment_intent = db
        .find_payment_intent_by_payment_id_merchant_id(
            &payment_id,
            &merchant_id,
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

    let payment_link_id = payment_intent
        .payment_link_id
        .get_required_value("payment_link_id")
        .change_context(errors::ApiErrorResponse::PaymentLinkNotFound)?;

    helpers::validate_payment_status_against_not_allowed_statuses(
        &payment_intent.status,
        &[
            storage_enums::IntentStatus::Cancelled,
            storage_enums::IntentStatus::Succeeded,
            storage_enums::IntentStatus::Processing,
            storage_enums::IntentStatus::RequiresCapture,
            storage_enums::IntentStatus::RequiresMerchantAction,
        ],
        "use payment link for",
    )?;

    let payment_link = db
        .find_payment_link_by_payment_link_id(&payment_link_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentLinkNotFound)?;

    let payment_link_config = extract_payment_link_config(payment_link.payment_link_config)?;

    let order_details = validate_order_details(payment_intent.order_details)?;
    let return_url = if let Some(payment_create_return_url) = payment_intent.return_url {
        payment_create_return_url
    } else {
        merchant_account
            .return_url
            .ok_or(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "return_url",
            })?
    };

    let (pub_key, currency, client_secret) = validate_sdk_requirements(
        merchant_account.publishable_key,
        payment_intent.currency,
        payment_intent.client_secret,
    )?;

    let (default_sdk_theme, default_background_color) =
        (DEFAULT_SDK_THEME, DEFAULT_BACKGROUND_COLOR);

    let payment_details = api_models::payments::PaymentLinkDetails {
        amount: payment_intent.amount,
        currency,
        payment_id: payment_intent.payment_id,
        merchant_name: payment_link_config.clone().config.seller_name.unwrap_or(
            merchant_account
                .merchant_name
                .map(|merchant_name| merchant_name.into_inner().peek().to_owned())
                .unwrap_or_default(),
        ),
        order_details,
        return_url,
        expiry: payment_link.max_age,
        pub_key,
        client_secret,
        merchant_logo: payment_link_config
            .config
            .clone()
            .logo
            .unwrap_or(DEFAULT_MERCHANT_LOGO.to_string()),
        max_items_visible_after_collapse: 3,
        theme: payment_link_config
            .config
            .clone()
            .theme
            .unwrap_or(default_sdk_theme.to_string()),
    };

    let js_script = get_js_script(payment_details)?;
    let css_script = get_color_scheme_css(
        payment_link_config.clone(),
        default_background_color.to_string(),
    );
    let payment_link_data = services::PaymentLinkFormData {
        js_script,
        sdk_url: state.conf.payment_link.sdk_url.clone(),
        css_script,
    };
    Ok(services::ApplicationResponse::PaymenkLinkForm(Box::new(
        payment_link_data,
    )))
}

/*
The get_js_script function is used to inject dynamic value to payment_link sdk, which is unique to every payment.
*/

fn get_js_script(
    payment_details: api_models::payments::PaymentLinkDetails,
) -> RouterResult<String> {
    let payment_details_str = serde_json::to_string(&payment_details)
        .into_report()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to serialize PaymentLinkDetails")?;
    Ok(format!("window.__PAYMENT_DETAILS = {payment_details_str};"))
}

fn get_color_scheme_css(
    payment_link_config: api_models::admin::PaymentCreatePaymentLinkConfig,
    default_primary_color: String,
) -> String {
    let background_primary_color = payment_link_config
        .config
        .theme
        .unwrap_or(default_primary_color.clone());
    format!(
        ":root {{
      --primary-color: {background_primary_color};
    }}"
    )
}

fn validate_sdk_requirements(
    pub_key: Option<String>,
    currency: Option<api_models::enums::Currency>,
    client_secret: Option<String>,
) -> Result<(String, api_models::enums::Currency, String), errors::ApiErrorResponse> {
    let pub_key = pub_key.ok_or(errors::ApiErrorResponse::MissingRequiredField {
        field_name: "pub_key",
    })?;

    let currency = currency.ok_or(errors::ApiErrorResponse::MissingRequiredField {
        field_name: "currency",
    })?;

    let client_secret = client_secret.ok_or(errors::ApiErrorResponse::MissingRequiredField {
        field_name: "client_secret",
    })?;
    Ok((pub_key, currency, client_secret))
}

pub async fn list_payment_link(
    state: AppState,
    merchant: domain::MerchantAccount,
    constraints: api_models::payments::PaymentLinkListConstraints,
) -> RouterResponse<Vec<api_models::payments::RetrievePaymentLinkResponse>> {
    let db = state.store.as_ref();
    let payment_link = db
        .list_payment_link_by_merchant_id(&merchant.merchant_id, constraints)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to retrieve payment link")?;
    let payment_link_list = future::try_join_all(payment_link.into_iter().map(|payment_link| {
        api_models::payments::RetrievePaymentLinkResponse::from_db_payment_link(payment_link)
    }))
    .await?;
    Ok(services::ApplicationResponse::Json(payment_link_list))
}

pub fn check_payment_link_status(max_age: PrimitiveDateTime) -> String {
    let curr_time = common_utils::date_time::now();

    if curr_time > max_age {
        "expired".to_string()
    } else {
        "active".to_string()
    }
}

fn validate_order_details(
    order_details: Option<Vec<Secret<serde_json::Value>>>,
) -> Result<
    Option<Vec<api_models::payments::OrderDetailsWithAmount>>,
    error_stack::Report<errors::ApiErrorResponse>,
> {
    let order_details = order_details
        .map(|order_details| {
            order_details
                .iter()
                .map(|data| {
                    data.to_owned()
                        .parse_value("OrderDetailsWithAmount")
                        .change_context(errors::ApiErrorResponse::InvalidDataValue {
                            field_name: "OrderDetailsWithAmount",
                        })
                        .attach_printable("Unable to parse OrderDetailsWithAmount")
                })
                .collect::<Result<Vec<api_models::payments::OrderDetailsWithAmount>, _>>()
        })
        .transpose()?;

    let updated_order_details = order_details.map(|mut order_details| {
        for order in order_details.iter_mut() {
            if order.product_img_link.is_none() {
                order.product_img_link = Some(DEFAULT_PRODUCT_IMG.to_string());
            }
        }
        order_details
    });
    Ok(updated_order_details)
}

pub fn extract_business_payment_link_config(
    pl_config: serde_json::Value,
) -> Result<admin_types::BusinessPaymentLinkConfig, error_stack::Report<errors::ApiErrorResponse>> {
    serde_json::from_value::<admin_types::BusinessPaymentLinkConfig>(pl_config.clone())
        .into_report()
        .change_context(errors::ApiErrorResponse::InvalidDataValue {
            field_name: "payment_link_config",
        })
}

pub fn extract_payment_link_config(
    pl_config: serde_json::Value,
) -> Result<
    admin_types::PaymentCreatePaymentLinkConfig,
    error_stack::Report<errors::ApiErrorResponse>,
> {
    serde_json::from_value::<admin_types::PaymentCreatePaymentLinkConfig>(pl_config.clone())
        .into_report()
        .change_context(errors::ApiErrorResponse::InvalidDataValue {
            field_name: "payment_link_config",
        })
}

pub fn get_payment_link_config_based_on_priority(
    payment_create_link_config: Option<admin_types::PaymentCreatePaymentLinkConfig>,
    business_link_config: Option<serde_json::Value>,
    merchant_name: String,
    default_domain_name: String,
) -> Result<
    (admin_types::PaymentCreatePaymentLinkConfig, String),
    error_stack::Report<errors::ApiErrorResponse>,
> {
    match (payment_create_link_config, business_link_config) {
        (Some(payment_create), Some(business)) => {
            let business_link_config = extract_business_payment_link_config(business)?;
            let domain_name = business_link_config
                .domain_name
                .map(|domain_name| format!("https://{}", domain_name))
                .unwrap_or(default_domain_name);

            let theme = payment_create.config.theme.unwrap_or_else(|| {
                business_link_config
                    .config
                    .theme
                    .unwrap_or(DEFAULT_BACKGROUND_COLOR.to_string())
            });
            let logo = payment_create.config.logo.unwrap_or_else(|| {
                business_link_config
                    .config
                    .logo
                    .unwrap_or(DEFAULT_MERCHANT_LOGO.to_string())
            });
            let seller_name = payment_create.config.seller_name.unwrap_or_else(|| {
                business_link_config
                    .config
                    .seller_name
                    .unwrap_or(merchant_name)
            });

            let max_age = payment_create.config.max_age.unwrap_or(
                business_link_config
                    .config
                    .max_age
                    .unwrap_or(DEFAULT_PAYMENT_LINK_EXPIRY),
            );

            Ok((
                admin_types::PaymentCreatePaymentLinkConfig {
                    config: admin_types::PaymentLinkConfig {
                        max_age: Some(max_age),
                        theme: Some(theme),
                        logo: Some(logo),
                        seller_name: Some(seller_name),
                    },
                },
                domain_name,
            ))
        }
        (Some(payment_create), None) => {
            let theme = payment_create
                .config
                .theme
                .unwrap_or(DEFAULT_BACKGROUND_COLOR.to_string());
            let logo = payment_create
                .config
                .logo
                .unwrap_or(DEFAULT_MERCHANT_LOGO.to_string());
            let seller_name = payment_create.config.seller_name.unwrap_or(merchant_name);
            let max_age = payment_create
                .config
                .max_age
                .unwrap_or(DEFAULT_PAYMENT_LINK_EXPIRY);

            Ok((
                admin_types::PaymentCreatePaymentLinkConfig {
                    config: admin_types::PaymentLinkConfig {
                        max_age: Some(max_age),
                        theme: Some(theme),
                        logo: Some(logo),
                        seller_name: Some(seller_name),
                    },
                },
                default_domain_name,
            ))
        }
        (None, Some(business)) => {
            let business_link_config = extract_business_payment_link_config(business)?;
            let domain_name = business_link_config
                .domain_name
                .map(|domain_name| format!("https://{}", domain_name))
                .unwrap_or(default_domain_name);
            let theme = business_link_config
                .config
                .theme
                .unwrap_or(DEFAULT_BACKGROUND_COLOR.to_string());
            let logo = business_link_config
                .config
                .logo
                .unwrap_or(DEFAULT_MERCHANT_LOGO.to_string());
            let seller_name = business_link_config
                .config
                .seller_name
                .unwrap_or(merchant_name);
            let max_age = business_link_config
                .config
                .max_age
                .unwrap_or(DEFAULT_PAYMENT_LINK_EXPIRY);
            Ok((
                admin_types::PaymentCreatePaymentLinkConfig {
                    config: admin_types::PaymentLinkConfig {
                        max_age: Some(max_age),
                        theme: Some(theme),
                        logo: Some(logo),
                        seller_name: Some(seller_name),
                    },
                },
                domain_name,
            ))
        }
        (None, None) => {
            let default_payment_config = admin_types::PaymentCreatePaymentLinkConfig {
                config: admin_types::PaymentLinkConfig {
                    max_age: Some(DEFAULT_PAYMENT_LINK_EXPIRY),
                    theme: Some(DEFAULT_BACKGROUND_COLOR.to_string()),
                    logo: Some(DEFAULT_MERCHANT_LOGO.to_string()),
                    seller_name: Some(merchant_name),
                },
            };
            Ok((default_payment_config, default_domain_name))
        }
    }
}

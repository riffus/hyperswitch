use cards::CardNumber;
use common_utils::pii;
use masking::Secret;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{enums as api_enums, payments};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum PayoutRequest {
    PayoutCreateRequest(PayoutCreateRequest),
    PayoutRetrieveRequest(PayoutRetrieveRequest),
}

// #[cfg(feature = "payouts")]
#[derive(Default, Debug, Deserialize, Serialize, Clone, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PayoutCreateRequest {
    /// Unique identifier for the payout. This ensures idempotency for multiple payouts
    /// that have been done by a single merchant. This field is auto generated and is returned in the API response.
    #[schema(
        value_type = Option<String>,
        min_length = 30,
        max_length = 30,
        example = "payout_mbabizu24mvu3mela5njyhpit4"
    )]
    pub payout_id: Option<String>, // TODO: Update this to PayoutIdType similar to PaymentIdType

    /// This is an identifier for the merchant account. This is inferred from the API key
    /// provided during the request
    #[schema(max_length = 255, example = "merchant_1668273825")]
    pub merchant_id: Option<String>,

    /// The payout amount. Amount for the payout in lowest denomination of the currency. (i.e) in cents for USD denomination, in paisa for INR denomination etc.,
    #[schema(value_type = u64, example = 6540)]
    #[serde(default, deserialize_with = "payments::amount::deserialize_option")]
    pub amount: Option<payments::Amount>,

    /// The currency of the payout request can be specified here
    #[schema(value_type = Option<Currency>, example = "USD")]
    pub currency: Option<api_enums::Currency>,

    /// This allows the merchant to manually select a connector with which the payout can go through
    #[schema(value_type = Option<Vec<Connector>>, max_length = 255, example = json!(["stripe", "adyen"]))]
    pub connector: Option<Vec<api_enums::Connector>>,

    /// The payout method that is to be used
    #[schema(value_type = PayoutType, example = "card")]
    pub payout_type: api_enums::PayoutType,

    /// The boolean value to create payout with connector
    #[schema(value_type = bool, example = true, default = false)]
    pub create_payout: Option<bool>,

    /// The payout method information required for carrying out a payout
    #[schema(value_type = Option<PayoutMethodData>)]
    pub payout_method_data: Option<PayoutMethodData>,

    /// The billing address for the payout
    pub billing: Option<payments::Address>,

    /// The identifier for the customer object. If not provided the customer ID will be autogenerated.
    #[schema(max_length = 255, example = "cus_y3oqhf46pyzuxjbcn2giaqnb44")]
    pub customer_id: Option<String>,

    /// Set to true to confirm the payout without review, no further action required
    #[schema(value_type = bool, example = true, default = false)]
    pub auto_fulfill: Option<bool>,

    /// description: The customer's email address
    #[schema(max_length = 255, value_type = Option<String>, example = "johntest@test.com")]
    pub email: Option<Secret<String, pii::EmailStrategy>>,

    /// description: The customer's name
    #[schema(value_type = Option<String>, max_length = 255, example = "John Test")]
    pub name: Option<Secret<String>>,

    /// The customer's phone number
    #[schema(value_type = Option<String>, max_length = 255, example = "3141592653")]
    pub phone: Option<Secret<String>>,

    /// The country code for the customer phone number
    #[schema(max_length = 255, example = "+1")]
    pub phone_country_code: Option<String>,

    /// It's a token used for client side verification.
    #[schema(example = "pay_U42c409qyHwOkWo3vK60_secret_el9ksDkiB8hi6j9N78yo")]
    pub client_secret: Option<String>,

    /// The URL to redirect after the completion of the operation
    #[schema(example = "https://hyperswitch.io")]
    pub return_url: Option<String>,

    /// Business country of the merchant for this payout
    #[schema(example = "US")]
    pub business_country: Option<api_enums::CountryAlpha2>,

    /// Business label of the merchant for this payout
    #[schema(example = "food")]
    pub business_label: Option<String>,

    /// A description of the payout
    #[schema(example = "It's my first payout request")]
    pub description: Option<String>,

    /// Type of entity to whom the payout is being carried out to
    #[schema(value_type = Option<api_enums::EntityType>)]
    pub entity_type: Option<api_enums::EntityType>,

    /// Specifies whether or not the payout request is recurring
    #[schema(value_type = Option<bool>)]
    pub recurring: Option<bool>,

    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    #[schema(value_type = Option<pii::SecretSerdeValue>)]
    pub metadata: Option<pii::SecretSerdeValue>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PayoutMethodData {
    Card(Card),
    Bank(Bank),
}

impl Default for PayoutMethodData {
    fn default() -> Self {
        Self::Card(Card::default())
    }
}

#[derive(Default, Eq, PartialEq, Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct Card {
    /// The card number
    #[schema(value_type = String, example = "4242424242424242")]
    pub card_number: CardNumber,

    /// The card's expiry month
    #[schema(value_type = String)]
    pub expiry_month: Secret<String>,

    /// The card's expiry year
    #[schema(value_type = String)]
    pub expiry_year: Secret<String>,

    /// The card holder's name
    #[schema(value_type = String, example = "John Doe")]
    pub card_holder_name: Secret<String>,
}

#[derive(Eq, PartialEq, Clone, Debug, Serialize, ToSchema)]
/// TODO: Implement standard format display for Bank
pub struct Bank {
    /// Bank account number is an unique identifier assigned by a bank to a customer.
    #[schema(value_type = String, example = "000123456")]
    pub bank_account_number: Option<String>,

    /// [9 digits] Routing number - used in USA for identifying a specific bank.
    #[schema(value_type = String, example = "110000000")]
    pub bank_routing_number: Option<String>,

    /// International Bank Account Number (iban) - used in many countries for identifying a bank along with it's customer.
    #[schema(value_type = String, example = "DE89370400440532013000")]
    pub iban: Option<String>,

    /// [8 / 11 digits] Bank Identifier Code (bic) / Swift Code - used in many countries for identifying a bank and it's branches
    #[schema(value_type = String, example = "HSBCGB2LXXX")]
    pub bic: Option<String>,

    /// [6 digits] Sort Code - used in UK and Ireland for identifying a bank and it's branches.
    #[schema(value_type = String, example = "98-76-54")]
    pub bank_sort_code: Option<String>,

    /// Bankleitzahl [blz] - used in Germany and Austria for identifying a bank and it's branches.
    #[schema(value_type = String, example = "10070024")]
    pub blz: Option<String>,

    /// [5 digits] Transit Number - used in Canada for identifying a bank and it's branches.
    #[schema(value_type = String, example = "12345-123")]
    pub bank_transit_number: Option<String>,

    /// Bank name
    #[schema(value_type = String, example = "Deutsche Bank")]
    pub bank_name: String,
}

impl Default for Bank {
    fn default() -> Self {
        Self {
            bank_account_number: Some("000123456".to_string()),
            bank_routing_number: Some("110000000".to_string()),
            iban: None,
            bic: None,
            bank_sort_code: None,
            blz: None,
            bank_transit_number: None,
            bank_name: "Deutsche Bank".to_string(),
        }
    }
}

impl<'de> Deserialize<'de> for Bank {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de;
        #[derive(Deserialize)]
        struct BankParams {
            bank_account_number: Option<String>,
            bank_routing_number: Option<String>,
            iban: Option<String>,
            bic: Option<String>,
            bank_sort_code: Option<String>,
            blz: Option<String>,
            bank_transit_number: Option<String>,
            bank_name: String,
        }

        let p = BankParams::deserialize(deserializer)?;
        match
            (
                p.bank_account_number.as_ref(),
                p.bank_routing_number.as_ref(),
                p.iban.as_ref(),
                p.bic.as_ref(),
                p.bank_sort_code.as_ref(),
                p.blz.as_ref(),
                p.bank_transit_number.as_ref(),
            )
        {
            (None, None, None, None, None, None, None) =>
                Err(
                    de::Error::custom(
                        "Invalid bank details, atleast one of bank_account_number + bank_routing_number / bic / bank_sort_code / blz / bank_transit_number OR iban must be provided."
                    )
                ),
            (None, Some(_), _, _, _, _, _) =>
                Err(
                    de::Error::custom(
                        "Invalid bank details, bank_account_number is required when passing bank_routing_number"
                    )
                ),
            (None, _, _, Some(_), _, _, _) =>
                Err(
                    de::Error::custom(
                        "Invalid bank details, bank_account_number is required when passing bic"
                    )
                ),
            (None, _, _, _, Some(_), _, _) =>
                Err(
                    de::Error::custom(
                        "Invalid bank details, bank_account_number is required when passing bank_sort_code"
                    )
                ),
            (None, _, _, _, _, Some(_), _) =>
                Err(
                    de::Error::custom(
                        "Invalid bank details, bank_account_number is required when passing blz"
                    )
                ),
            (None, _, _, _, _, _, Some(_)) =>
                Err(
                    de::Error::custom(
                        "Invalid bank details, bank_account_number is required when passing bank_transit_number"
                    )
                ),
            (Some(_), None, _, None, None, None, None) =>
                Err(
                    de::Error::custom(
                        "Invalid bank details, bank_account_number should be passed along with atleast one of bank_routing_number, bic, bank_sort_code, blz or bank_transit_number"
                    )
                ),
            _ =>
                Ok(Self {
                    bank_account_number: p.bank_account_number,
                    bank_routing_number: p.bank_routing_number,
                    iban: p.iban,
                    bic: p.bic,
                    bank_sort_code: p.bank_sort_code,
                    blz: p.blz,
                    bank_transit_number: p.bank_transit_number,
                    bank_name: p.bank_name,
                }),
        }
    }
}

#[derive(Debug, ToSchema, Clone, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PayoutCreateResponse {
    /// Unique identifier for the payout. This ensures idempotency for multiple payouts
    /// that have been done by a single merchant. This field is auto generated and is returned in the API response.
    #[schema(
        value_type = String,
        min_length = 30,
        max_length = 30,
        example = "payout_mbabizu24mvu3mela5njyhpit4"
    )]
    pub payout_id: String, // TODO: Update this to PayoutIdType similar to PaymentIdType

    /// This is an identifier for the merchant account. This is inferred from the API key
    /// provided during the request
    #[schema(max_length = 255, example = "merchant_1668273825")]
    pub merchant_id: String,

    /// The payout amount. Amount for the payout in lowest denomination of the currency. (i.e) in cents for USD denomination, in paisa for INR denomination etc.,
    #[schema(example = 100)]
    pub amount: i64,

    /// Recipient's currency for the payout request
    #[schema(value_type = Currency, example = "USD")]
    pub currency: api_enums::Currency,

    /// The connector used for the payout
    #[schema(example = "stripe")]
    pub connector: Option<String>,

    /// The payout method that is to be used
    #[schema(value_type = PayoutType, example = "card")]
    pub payout_type: api_enums::PayoutType,

    /// The billing address for the payout
    pub billing: Option<payments::Address>,

    /// The identifier for the customer object. If not provided the customer ID will be autogenerated.
    #[schema(value_type = String, max_length = 255, example = "cus_y3oqhf46pyzuxjbcn2giaqnb44")]
    pub customer_id: String,

    /// Set to true to confirm the payout without review, no further action required
    #[schema(value_type = bool, example = true, default = false)]
    pub auto_fulfill: bool,

    /// description: The customer's email address
    #[schema(max_length = 255, value_type = Option<String>, example = "johntest@test.com")]
    pub email: Option<Secret<String, pii::EmailStrategy>>,

    /// description: The customer's name
    #[schema(value_type = Option<String>, max_length = 255, example = "John Test")]
    pub name: Option<Secret<String>>,

    /// The customer's phone number
    #[schema(value_type = Option<String>, max_length = 255, example = "3141592653")]
    pub phone: Option<Secret<String>>,

    /// The country code for the customer phone number
    #[schema(max_length = 255, example = "+1")]
    pub phone_country_code: Option<String>,

    /// It's a token used for client side verification.
    #[schema(example = "pay_U42c409qyHwOkWo3vK60_secret_el9ksDkiB8hi6j9N78yo")]
    pub client_secret: Option<String>,

    /// The URL to redirect after the completion of the operation
    #[schema(example = "https://hyperswitch.io")]
    pub return_url: Option<String>,

    /// Business country of the merchant for this payout
    #[schema(example = "US")]
    pub business_country: Option<api_enums::CountryAlpha2>,

    /// Business label of the merchant for this payout
    #[schema(example = "food")]
    pub business_label: Option<String>,

    /// A description of the payout
    #[schema(example = "It's my first payout request")]
    pub description: Option<String>,

    /// Type of entity to whom the payout is being carried out to
    #[schema(value_type = api_enums::EntityType)]
    pub entity_type: api_enums::EntityType,

    /// Specifies whether or not the payout request is recurring
    #[schema(value_type = bool)]
    pub recurring: bool,

    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    #[schema(value_type = Option<pii::SecretSerdeValue>)]
    pub metadata: Option<pii::SecretSerdeValue>,

    /// Current status of the Payout
    pub status: api_enums::PayoutStatus,

    /// If there was an error while calling the connector the error message is received here
    #[schema(example = "Failed while verifying the card")]
    pub error_message: Option<String>,

    /// If there was an error while calling the connectors the code is received here
    #[schema(example = "E0001")]
    pub error_code: Option<String>,
}

#[derive(Default, Debug, Clone, Deserialize)]
pub struct PayoutRetrieveBody {
    pub force_sync: Option<bool>,
}

#[derive(Default, Debug, Serialize, ToSchema, Clone, Deserialize)]
pub struct PayoutRetrieveRequest {
    /// Unique identifier for the payout. This ensures idempotency for multiple payouts
    /// that have been done by a single merchant. This field is auto generated and is returned in the API response.
    #[schema(
        value_type = String,
        min_length = 30,
        max_length = 30,
        example = "payout_mbabizu24mvu3mela5njyhpit4"
    )]
    pub payout_id: String,

    /// `force_sync` with the connector to get payout details
    /// (defaults to false)
    #[schema(value_type = Option<bool>, default = false, example = true)]
    pub force_sync: Option<bool>,
}

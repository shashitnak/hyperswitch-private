use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct NmiPaymentsResponse {
  pub response: String,
  pub response_text: String,
  pub authcode: String,
  pub transaction_id: String,
  pub avsresponse: String,
  pub cvvresponse: Option<String>,
  pub order_id: String,
  #[serde(rename = "type")]
  pub transaction_type: String,
  pub response_code: String,
  pub amount_authorized: String,
  pub subscription_id: String,
  pub recurring: String,
  pub customer_vault_id: String,
  pub three_ds_version: String,
  pub eci: String,
  pub directory_server_id: String,
  pub cc_number: String,
  pub cc_exp: String
}
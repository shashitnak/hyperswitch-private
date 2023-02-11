use serde::{Deserialize, Serialize};

use crate::{
  types::{self, api, storage::enums as storage_enums, ErrorResponse},
};

#[derive(Debug, Serialize)]
pub struct NmiPaymentsRequest {
  #[serde(rename = "type")]
  pub transaction_type        : TransactionType,
  pub security_key            : String,
  pub ccnumber                : String,
  pub ccexp                   : String,
  pub cvv                     : String,
  pub account_holder_type     : Option<AccountHolderType>,
  pub account_type            : Option<AccountType>,
  pub sec_code                : Option<SecCode>,
  pub amount                  : String,
  pub surcharge               : Option<String>,
  pub currency                : storage_enums::Currency,
  pub payment                 : PaymentType,
  pub processor_id            : Option<String>,
  pub billing_method          : Option<BillingMethod>,
  pub billing_number          : Option<u8>,
  pub order_description       : Option<String>,
  pub orderid                 : Option<String>,
  pub first_name              : String,
  pub last_name               : String,
  pub address1                : String,
  pub address2                : String,
  pub city                    : String,
  pub state                   : Option<String>,
  pub zip                     : String,
  pub country                 : String,
  pub phone                   : String
}


#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BillingMethod {
  Recurring,
  Installment
}


#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TransactionType {
  Sale,
  Auth,
  Credit,
  Validate,
  Offline
}


#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AccountHolderType {
  Business,
  Personal
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AccountType {
  Checking,
  Savings
}


#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SecCode {
    /// Cash Concentration or Disbursement - Can be either a credit or debit application
    /// where funds are wither distributed or consolidated between corporate entities.
    #[serde(rename = "CCD")]
    CashConcentrationOrDisbursement,

    /// Point of Sale Entry - Point of sale debit applications non-shared (POS)
    /// environment. These transactions are most often initiated by the consumer via a plastic
    /// access card. This is only support for normal ACH transactions
    #[serde(rename = "POP")]
    PointOfSaleEntry,
    /// Prearranged Payment and Deposits - used to credit or debit a consumer account.
    /// Popularity used for payroll direct deposits and pre-authorized bill payments.
    #[serde(rename = "PPD")]
    PrearrangedPaymentAndDeposits,
    /// Telephone-Initiated Entry - Used for the origination of a single entry debit
    /// transaction to a consumer's account pursuant to a verbal authorization obtained from the
    /// consumer via the telephone.
    #[serde(rename = "TEL")]
    TelephoneInitiatedEntry,
    /// Internet (Web)-Initiated Entry - Used for the origination of debit entries
    /// (either Single or Recurring Entry) to a consumer's account pursuant to a to an
    /// authorization that is obtained from the Receiver via the Internet.
    #[serde(rename = "WEB")]
    WebInitiatedEntry,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaymentType {
  CreditCard,
  Check,
  Cash
}
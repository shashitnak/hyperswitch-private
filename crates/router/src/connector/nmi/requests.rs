use serde::{Deserialize, Serialize};

use crate::{
  types::{self, api, storage::enums as storage_enums, ErrorResponse},
};

#[derive(Debug, Serialize)]
pub struct NmiPaymentsRequest {
  #[serde(rename = "type")]
  transaction_type        : TransactionType,
  security_key            : String,
  ccnumber                : String,
  ccexp                   : String,
  cvv                     : String,
  account_holder_type     : Option<AccountHolderType>,
  account_type            : Option<AccountType>,
  sec_code                : Option<SecCode>,
  amount                  : String,
  surcharge               : Option<String>,
  currency                : storage_enums::Currency,
  payment                 : PaymentType,
  processor_id            : Option<String>,
  billing_method          : Option<BillingMethod>,
  billing_number          : Option<u8>,
  order_description       : Option<String>,
  orderid                 : Option<String>,
  first_name              : String,
  last_name               : String,
  address1                : String,
  address2                : String,
  city                    : String,
  state                   : Option<String>,
  zip                     : String,
  country                 : String,
  phone                   : String
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
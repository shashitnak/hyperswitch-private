use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{self, CardData, PaymentsRequestData},
    consts,
    logger,
    core::errors,
    types::{
        self, api,
        storage::{enums, enums as storage_enums},
        ErrorResponse, ConnectorAuthType,
    },
};
use error_stack::IntoReport;
use masking::PeekInterface;
use crate::connector::utils::AddressDetailsData;

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct NmiPaymentsRequest {
    #[serde(rename = "type")]
    pub transaction_type: TransactionType,
    pub security_key: String,
    pub ccnumber: String,
    pub ccexp: String,
    pub cvv: String,
    pub account_holder_type: Option<AccountHolderType>,
    pub account_type: Option<AccountType>,
    pub sec_code: Option<SecCode>,
    pub amount: String,
    pub surcharge: Option<String>,
    pub currency: storage_enums::Currency,
    pub payment: PaymentType,
    pub processor_id: Option<String>,
    pub billing_method: Option<BillingMethod>,
    pub billing_number: Option<u8>,
    pub order_description: Option<String>,
    pub orderid: Option<String>,
    pub first_name: String,
    pub last_name: String,
    pub address1: String,
    pub address2: String,
    pub city: String,
    pub state: Option<String>,
    pub zip: String,
    pub country: String,
    pub phone: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct NmiSyncRequest {
  pub transaction_id          : String,
  pub security_key            : String
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BillingMethod {
    #[default]
    Recurring,
    Installment,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Sale,
    #[default]
    Auth,
    Credit,
    Validate,
    Offline,
    Capture,
    Void,
    Refund
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AccountHolderType {
    #[default]
    Business,
    Personal,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AccountType {
    Checking,
    #[default]
    Savings,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SecCode {
    /// Cash Concentration or Disbursement - Can be either a credit or debit application
    /// where funds are wither distributed or consolidated between corporate entities.
    #[serde(rename = "CCD")]
    #[default]
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

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
// #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaymentType {
    #[default]
    creditcard,
    check,
    cash,
}


fn error<T>() -> Result<T, error_stack::Report<errors::ConnectorError>> {
    Err(errors::ConnectorError::FailedToObtainIntegrationUrl).into_report()
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for NmiPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        use api::payments::PaymentMethod::*;
        use storage_enums::CaptureMethod::*;
        use PaymentType::*;
        let transaction_type = match item.request.capture_method {
            Some(Automatic) => TransactionType::Sale,
            Some(Manual) => TransactionType::Auth,
            _ => error()?,
        };
        let security_key: NmiAuthType = (&item.connector_auth_type).try_into()?;
        let security_key = security_key.api_key;
        logger::debug!(security_key=?security_key);

        let card = match &item.request.payment_method_data {
            Card(card) => card,
            _ => error()?,
        };

        //     pub card_number: Secret<String, pii::CardNumber>,
        // /// The card's expiry month
        // #[schema(value_type = String, example = "24")]
        // pub card_exp_month: Secret<String>,
        // /// The card's expiry year
        // #[schema(value_type = String, example = "24")]
        // pub card_exp_year: Secret<String>,
        // /// The card holder's name
        // #[schema(value_type = String, example = "John Test")]
        // pub card_holder_name: Secret<String>,
        // /// The CVC number for the card
        // #[schema(value_type = String, example = "242")]
        // pub card_cvc: Secret<String>,

        let address = item.address.billing.as_ref().unwrap();

        let phone = address.phone.as_ref().unwrap();
        let address = address.address.as_ref().unwrap();

        Ok(NmiPaymentsRequest {
            transaction_type,
            security_key,
            ccnumber: card.card_number.peek().to_string(),
            ccexp: card.card_exp_month.peek().to_string() + &card.card_exp_year.peek().to_string(),
            cvv: card.card_cvc.peek().to_string(),
            account_holder_type: None,
            account_type: None,
            sec_code: None,
            amount: item.request.amount.to_string() + ".00",
            surcharge: None,
            currency: item.request.currency,
            payment: creditcard,
            processor_id: None,
            billing_method: None,
            billing_number: None,
            order_description: item.description.clone(),
            orderid: None,
            first_name: address.get_first_name().unwrap().peek().to_string(),
            last_name: address.get_last_name().unwrap().peek().to_string(),
            address1: address.get_line1().unwrap().peek().to_string(),
            address2: address.get_line2().unwrap().peek().to_string(),
            city: address.get_city().unwrap().to_string(),
            state: None,
            zip: address.get_zip().unwrap().peek().to_string(),
            country: address.get_country().unwrap().to_string(),
            phone: phone.number.as_ref().map(|x| x.peek().to_string()),
        })
    }
}

impl TryFrom<(&types::PaymentsSyncData, ConnectorAuthType)> for NmiSyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: (&types::PaymentsSyncData,ConnectorAuthType)) -> Result<Self, Self::Error> {
        let security_key: NmiAuthType = (&item.1).try_into()?;
        let security_key : String = security_key.api_key;
    //     pub card_number: Secret<String, pii::CardNumber>,
    // /// The card's expiry month
    // #[schema(value_type = String, example = "24")]
    // pub card_exp_month: Secret<String>,
    // /// The card's expiry year
    // #[schema(value_type = String, example = "24")]
    // pub card_exp_year: Secret<String>,
    // /// The card holder's name
    // #[schema(value_type = String, example = "John Test")]
    // pub card_holder_name: Secret<String>,
    // /// The CVC number for the card
    // #[schema(value_type = String, example = "242")]
    // pub card_cvc: Secret<String>,


      Ok(NmiSyncRequest {
        security_key,
        transaction_id : item.0.connector_transaction_id.get_connector_transaction_id().unwrap()
      })
    }
}



impl TryFrom<(&types::PaymentsCaptureData,ConnectorAuthType)> for NmiCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: (&types::PaymentsCaptureData,ConnectorAuthType)) -> Result<Self, Self::Error> {
        use storage_enums::CaptureMethod::*;
        use api::payments::PaymentMethod::*;
        use PaymentType::*;

        let security_key: NmiAuthType = (&item.1).try_into()?;
        let item = item.0;
        let security_key = security_key.api_key;
    //     pub card_number: Secret<String, pii::CardNumber>,
    // /// The card's expiry month
    // #[schema(value_type = String, example = "24")]
    // pub card_exp_month: Secret<String>,
    // /// The card's expiry year
    // #[schema(value_type = String, example = "24")]
    // pub card_exp_year: Secret<String>,
    // /// The card holder's name
    // #[schema(value_type = String, example = "John Test")]
    // pub card_holder_name: Secret<String>,
    // /// The CVC number for the card
    // #[schema(value_type = String, example = "242")]
    // pub card_cvc: Secret<String>,


      Ok(NmiCaptureRequest {
        transaction_type : TransactionType::Capture,
        security_key,
        transactionid : item.connector_transaction_id.clone(),
        amount : Some(item.amount.to_string() + ".00")
      })
    }
}


impl TryFrom<(&types::PaymentsCancelData,ConnectorAuthType)> for NmiCancelRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: (&types::PaymentsCancelData,ConnectorAuthType)) -> Result<Self, Self::Error> {
        use storage_enums::CaptureMethod::*;
        use api::payments::PaymentMethod::*;
        use PaymentType::*;

        let security_key: NmiAuthType = (&item.1).try_into()?;
        let item = item.0;
        let security_key = security_key.api_key;
    //     pub card_number: Secret<String, pii::CardNumber>,
    // /// The card's expiry month
    // #[schema(value_type = String, example = "24")]
    // pub card_exp_month: Secret<String>,
    // /// The card's expiry year
    // #[schema(value_type = String, example = "24")]
    // pub card_exp_year: Secret<String>,
    // /// The card holder's name
    // #[schema(value_type = String, example = "John Test")]
    // pub card_holder_name: Secret<String>,
    // /// The CVC number for the card
    // #[schema(value_type = String, example = "242")]
    // pub card_cvc: Secret<String>,


      Ok(NmiCancelRequest {
        transaction_type : TransactionType::Capture,
        security_key,
        transactionid : item.connector_transaction_id.clone(),
        void_reason: item.cancellation_reason.clone()
      })
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct NmiAuthType {
    pub(super) api_key: String,
}

impl TryFrom<&types::ConnectorAuthType> for NmiAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::HeaderKey { api_key } = auth_type {
            Ok(Self {
                api_key: api_key.to_string(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType.into())
        }
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum NmiPaymentStatus {
    Authorised,
    Captured,
    Failed,
    #[default]
    Processing,
    Settled,
    Canceled,
    VoidFailed
}

impl From<NmiPaymentStatus> for enums::AttemptStatus {
    fn from(item: NmiPaymentStatus) -> Self {
        match item {
            NmiPaymentStatus::Authorised => Self::Authorized,
            NmiPaymentStatus::Failed => Self::Failure,
            NmiPaymentStatus::Captured => Self::CaptureInitiated,
            NmiPaymentStatus::Processing => Self::Pending,
            NmiPaymentStatus::Settled => Self::Charged,
            NmiPaymentStatus::Canceled => Self::Voided,
            NmiPaymentStatus::VoidFailed => Self::VoidFailed,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NmiPaymentsResponse {
    pub status: NmiPaymentStatus,
    pub id: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum NmiSyncResponseStatus {
    pendingSettlement,
    pending,
    #[default]
    failed,
    canceled,
    complete,
    unknown
}


#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct NmiCaptureRequest {
    #[serde(rename = "type")]
    pub transaction_type: TransactionType,
    pub security_key: String,
    pub transactionid: String,
    pub amount : Option<String>
}


#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct NmiCancelRequest {
    #[serde(rename = "type")]
    pub transaction_type: TransactionType,
    pub security_key: String,
    pub transactionid: String,
    pub void_reason: Option<String>
}



impl From<NmiSyncResponseStatus> for enums::AttemptStatus {
    fn from(item: NmiSyncResponseStatus) -> Self {
        match item {
            NmiSyncResponseStatus::pendingSettlement => Self::CaptureInitiated,
            NmiSyncResponseStatus::failed => Self::Failure,
            NmiSyncResponseStatus::pending => Self::Authorized,
            NmiSyncResponseStatus::canceled => Self::Voided,
            NmiSyncResponseStatus::complete => Self::Charged,
            NmiSyncResponseStatus::unknown => Self::Failure,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NmiSyncResponse {
    pub condition : NmiSyncResponseStatus,
    pub transaction_id : String
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, NmiPaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        item: types::ResponseRouterData<F, NmiPaymentsResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: None,
                redirect: false,
                mandate_reference: None,
                connector_metadata: None,
            }),
            ..item.data
        })
    }
}

impl<F, Req>
    TryFrom<types::ResponseRouterData<F, NmiSyncResponse, Req, types::PaymentsResponseData>>
    for types::RouterData<F, Req, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        items: types::ResponseRouterData<F, NmiSyncResponse, Req, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        
        let response = items.response;

        Ok(Self {
            status: enums::AttemptStatus::from(response.condition.clone()),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(response.transaction_id),
                redirect: false,
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None
            }),
            ..items.data
        })
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NmiCaptureResponse {
    response: usize,
    responseText: Option<String>,
    #[serde(rename = "type")]
    transaction_type: TransactionType,
    transactionid : Option<String>
}


// impl<F, T>
//     TryFrom<
//         types::ResponseRouterData<F, NmiPaymentsResponse, T, types::PaymentsResponseData>,
//     > for types::RouterData<F, T, types::PaymentsResponseData>
// {
//     type Error = error_stack::Report<errors::ConnectorError>;
//     fn try_from(
//         item: types::ResponseRouterData<
//             F,
//             NmiPaymentsResponse,
//             T,
//             types::PaymentsResponseData,
//         >,
//     ) -> Result<Self, Self::Error> {
//         let response = item.response;
//         Ok(Self {
//             status: match response.status {
//                 NmiPaymentStatus::Captured => enums::AttemptStatus::CaptureInitiated,
//                 _ => enums::AttemptStatus::CaptureFailed
//             },
//             response: Ok(types::PaymentsResponseData::TransactionResponse {
//                 resource_id: types::ResponseId::ConnectorTransactionId(response.transactionid.unwrap()),
//                 redirect: false,
//                 redirection_data: None,
//                 mandate_reference: None,
//                 connector_metadata: None,
//             }),
//             amount_captured: None,
//             ..item.data
//         })
//     }
// }

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct NmiRefundRequest {}

impl<F> TryFrom<&types::RefundsRouterData<F>> for NmiRefundRequest {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(_item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        todo!()
    }
}

// Type definition for Refund Response

#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub enum RefundStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Succeeded => Self::Success,
            RefundStatus::Failed => Self::Failure,
            RefundStatus::Processing => Self::Pending,
            //TODO: Review mapping
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        _item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        todo!()
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        _item: types::RefundsResponseRouterData<api::RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        todo!()
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct NmiErrorResponse {
    pub error_code: String
}

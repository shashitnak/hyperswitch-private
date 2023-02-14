use error_stack::IntoReport;
use masking::PeekInterface;
use serde::{Deserialize, Serialize};

use crate::{
    core::errors,
    logger,
    types::{
        self, api,
        storage::{enums, enums as storage_enums},
        ConnectorAuthType,
    },
};

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct NmiPaymentsRequest {
    #[serde(rename = "type")]
    pub transaction_type: TransactionType,
    pub security_key: String,
    pub ccnumber: String,
    pub ccexp: String,
    pub cvv: String,
    pub amount: String,
}

#[derive(Debug, Serialize)]
pub struct NmiSyncRequest {
    pub transaction_id: String,
    pub security_key: String,
}

// #[derive(Default, Debug, Serialize, Eq, PartialEq)]
// #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
// pub enum BillingMethod {
//     #[default]
//     Recurring,
//     Installment,
// }

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
    Refund,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PaymentType {
    #[default]
    CreditCard,
}

fn error(msg: &'static str) -> error_stack::Report<errors::ConnectorError> {
    match Err(errors::ConnectorError::RequestEncodingFailedWithReason(
        msg.to_string(),
    ))
    .into_report()
    {
        Ok(()) => panic!("Impossible"),
        Err(err) => err,
    }
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for NmiPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        use api::payments::PaymentMethod::*;
        use storage_enums::CaptureMethod::*;
        let transaction_type = match item.request.capture_method {
            Some(Automatic) => TransactionType::Sale,
            Some(Manual) => TransactionType::Auth,
            _ => Err(error(
                "Only Transaction Type 'Automatic' and 'Manual' are allowed",
            ))?,
        };
        let security_key: NmiAuthType = (&item.connector_auth_type).try_into()?;
        let security_key = security_key.api_key;
        logger::debug!(security_key=?security_key);

        let card = match &item.request.payment_method_data {
            Card(card) => card,
            _ => Err(error("Only Card Payment supported"))?,
        };

        Ok(Self {
            transaction_type,
            security_key,
            // ccnumber: "4111111111111111".to_string(),
            // ccexp: "1212".to_string(),
            // cvv: "999".to_string(),
            ccnumber: card.card_number.peek().to_string(),
            ccexp: card.card_exp_month.peek().to_string() + &card.card_exp_year.peek().to_string(),
            cvv: card.card_cvc.peek().to_string(),
            amount: item.request.amount.to_string() + ".00",
        })
    }
}

impl TryFrom<&types::VerifyRouterData> for NmiPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::VerifyRouterData) -> Result<Self, Self::Error> {
        use api::payments::PaymentMethod::*;
        let transaction_type = TransactionType::Validate;
        let security_key: NmiAuthType = (&item.connector_auth_type).try_into()?;
        let security_key = security_key.api_key;
        logger::debug!(security_key=?security_key);

        let card = match &item.request.payment_method_data {
            Card(card) => card,
            _ => Err(error("Only Card Payment supported"))?,
        };

        Ok(Self {
            transaction_type,
            security_key,
            ccnumber: card.card_number.peek().to_string(),
            ccexp: card.card_exp_month.peek().to_string() + &card.card_exp_year.peek().to_string(),
            cvv: card.card_cvc.peek().to_string(),
            amount: "0.00".to_string(),
        })
    }
}

impl TryFrom<(&types::PaymentsSyncData, ConnectorAuthType)> for NmiSyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: (&types::PaymentsSyncData, ConnectorAuthType)) -> Result<Self, Self::Error> {
        let security_key: NmiAuthType = (&item.1).try_into()?;
        let security_key: String = security_key.api_key;

        Ok(Self {
            security_key,
            transaction_id: item
                .0
                .connector_transaction_id
                .get_connector_transaction_id()
                .map_err(|_| error("Did not find connector transaction Id"))?,
        })
    }
}

impl TryFrom<(&types::PaymentsCaptureData, ConnectorAuthType)> for NmiCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: (&types::PaymentsCaptureData, ConnectorAuthType),
    ) -> Result<Self, Self::Error> {
        let security_key: NmiAuthType = (&item.1).try_into()?;
        let item = item.0;
        let security_key = security_key.api_key;

        Ok(Self {
            transaction_type: TransactionType::Capture,
            security_key,
            transactionid: item.connector_transaction_id.clone(),
            amount: Some(item.amount.to_string() + ".00"),
        })
    }
}

impl TryFrom<(&types::PaymentsCancelData, ConnectorAuthType)> for NmiCancelRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: (&types::PaymentsCancelData, ConnectorAuthType),
    ) -> Result<Self, Self::Error> {
        let security_key: NmiAuthType = (&item.1).try_into()?;
        let item = item.0;
        let security_key = security_key.api_key;

        Ok(Self {
            transaction_type: TransactionType::Capture,
            security_key,
            transactionid: item.connector_transaction_id.clone(),
            void_reason: item.cancellation_reason.clone(),
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
    VoidFailed,
}

impl From<NmiPaymentStatus> for enums::AttemptStatus {
    fn from(item: NmiPaymentStatus) -> Self {
        match item {
            NmiPaymentStatus::Authorised => Self::Authorized,
            NmiPaymentStatus::Failed => Self::Failure,
            NmiPaymentStatus::Captured => Self::Charged,
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
#[serde(rename_all = "lowercase")]
pub enum NmiSyncResponseStatus {
    PendingSettlement,
    Pending,
    #[default]
    Failed,
    Canceled,
    Complete,
    Unknown,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct NmiCaptureRequest {
    #[serde(rename = "type")]
    pub transaction_type: TransactionType,
    pub security_key: String,
    pub transactionid: String,
    pub amount: Option<String>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct NmiCancelRequest {
    #[serde(rename = "type")]
    pub transaction_type: TransactionType,
    pub security_key: String,
    pub transactionid: String,
    pub void_reason: Option<String>,
}

impl From<NmiSyncResponseStatus> for enums::AttemptStatus {
    fn from(item: NmiSyncResponseStatus) -> Self {
        match item {
            NmiSyncResponseStatus::PendingSettlement => Self::Charged,
            NmiSyncResponseStatus::Failed => Self::Failure,
            NmiSyncResponseStatus::Pending => Self::Authorized,
            NmiSyncResponseStatus::Canceled => Self::Voided,
            NmiSyncResponseStatus::Complete => Self::Charged,
            NmiSyncResponseStatus::Unknown => Self::Failure,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NmiSyncResponse {
    pub condition: NmiSyncResponseStatus,
    pub transaction_id: String,
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
                connector_metadata: None,
            }),
            ..items.data
        })
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NmiCaptureResponse {
    response: usize,
    response_text: Option<String>,
    #[serde(rename = "type")]
    transaction_type: TransactionType,
    transactionid: Option<String>,
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct NmiRefundRequest {
    #[serde(rename = "type")]
    transaction_type: TransactionType,
    security_key: String,
    transactionid: String,
    amount: Option<String>,
}

impl TryFrom<(&types::RefundsData, ConnectorAuthType)> for NmiRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: (&types::RefundsData, ConnectorAuthType)) -> Result<Self, Self::Error> {
        let security_key: NmiAuthType = (&item.1).try_into()?;
        let item = item.0;
        let security_key = security_key.api_key;

        Ok(Self {
            transaction_type: TransactionType::Refund,
            security_key,
            transactionid: item.connector_transaction_id.clone(),
            amount: Some(item.refund_amount.to_string() + ".00"),
        })
    }
}

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
        }
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct NmiErrorResponse {
    pub error_code: String,
}

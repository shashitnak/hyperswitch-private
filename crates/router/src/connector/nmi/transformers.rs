use common_utils::pii;
use error_stack::ResultExt;
use masking::{PeekInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    core::errors,
    types::{self, api, storage::enums, ConnectorAuthType},
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Sale,
    Auth,
    Credit,
    Validate,
    Capture,
    Void,
    Refund,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PaymentType {
    CreditCard(Card),
}

#[derive(Debug, Serialize)]
pub struct Card {
    pub ccnumber: Secret<String, pii::CardNumber>,
    pub ccexp: Secret<String>,
    pub cvv: Secret<String>,
}

#[derive(Debug, Deserialize)]
pub enum Response {
    #[serde(alias = "1")]
    Approved,
    #[serde(alias = "2")]
    Declined,
    #[serde(alias = "3")]
    Error,
}

#[derive(Debug, Deserialize)]
pub struct GenericResponse {
    pub response: Response,
    pub responsetext: Option<String>,
    pub authcode: Option<String>,
    pub transactionid: String,
    pub avsresponse: Option<String>,
    pub cvvresponse: Option<String>,
    pub orderid: String,
    pub response_code: Option<String>,
}

impl<F, T> TryFrom<types::ResponseRouterData<F, GenericResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, GenericResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from((item.response.response)),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.transactionid),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
            }),
            ..item.data
        })
    }
}

impl From<Response> for enums::AttemptStatus {
    fn from(item: Response) -> Self {
        match item {
            Response::Approved => enums::AttemptStatus::Charged,
            Response::Declined | Response::Error => enums::AttemptStatus::Failure,
        }
    }
}

// Auth Struct
pub struct NmiAuthType {
    pub(super) api_key: String,
}

impl TryFrom<&ConnectorAuthType> for NmiAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::HeaderKey { api_key } = auth_type {
            Ok(Self {
                api_key: api_key.to_string(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType.into())
        }
    }
}

#[derive(Debug, Serialize)]
pub struct NmiPaymentsRequest {
    #[serde(rename = "type")]
    pub transaction_type: TransactionType,
    pub security_key: String,
    pub amount: String,
    pub currency: enums::Currency,
    #[serde(flatten)]
    pub payment_type: PaymentType,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for NmiPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let transaction_type = match item.request.capture_method {
            Some(storage_models::enums::CaptureMethod::Automatic) => TransactionType::Sale,
            Some(storage_models::enums::CaptureMethod::Manual) => TransactionType::Auth,
            _ => Err(errors::ConnectorError::NotImplemented(
                "Capture Method".to_string(),
            ))?,
        };
        let security_key: NmiAuthType = (&item.connector_auth_type).try_into()?;
        let security_key = security_key.api_key;
        let payment_type = get_payment_type(&item.request.payment_method_data)?;
        Ok(Self {
            transaction_type,
            security_key,
            payment_type,
            amount: item.request.amount.to_string(),
            currency: item.request.currency,
        })
    }
}

fn get_payment_type(
    payment_method: &api::PaymentMethod,
) -> Result<PaymentType, errors::ConnectorError> {
    match payment_method {
        api::PaymentMethod::Card(card) => {
            let expiry_year = card.card_exp_year.peek().clone();
            let secret_value = format!(
                "{}{}",
                card.card_exp_month.peek(),
                &expiry_year[expiry_year.len() - 2..]
            );
            let expiry_date: Secret<String> = Secret::new(secret_value);
            Ok(PaymentType::CreditCard(Card {
                ccnumber: card.card_number.clone(),
                ccexp: expiry_date,
                cvv: card.card_cvc.clone(),
            }))
        }
        _ => Err(errors::ConnectorError::NotImplemented(
            "Payment Method".to_string(),
        )),
    }
}

impl TryFrom<&types::VerifyRouterData> for NmiPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::VerifyRouterData) -> Result<Self, Self::Error> {
        let transaction_type = TransactionType::Validate;
        let security_key: NmiAuthType = (&item.connector_auth_type).try_into()?;
        let security_key = security_key.api_key;
        let payment_type = get_payment_type(&item.request.payment_method_data)?;

        Ok(Self {
            transaction_type,
            security_key,
            payment_type,
            amount: "0.0".to_string(),
            currency: item.request.currency,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct NmiSyncRequest {
    pub transaction_id: String,
    pub security_key: String,
}

impl TryFrom<&types::PaymentsSyncRouterData> for NmiSyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsSyncRouterData) -> Result<Self, Self::Error> {
        let auth = NmiAuthType::try_from(&item.connector_auth_type)?;
        Ok(Self {
            security_key: auth.api_key,
            transaction_id: item
                .request
                .connector_transaction_id
                .get_connector_transaction_id()
                .change_context(errors::ConnectorError::MissingConnectorTransactionID)?,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct NmiCaptureRequest {
    #[serde(rename = "type")]
    pub transaction_type: TransactionType,
    pub security_key: String,
    pub transactionid: String,
    pub amount: Option<String>,
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

#[derive(Debug, Serialize)]
pub struct NmiCancelRequest {
    #[serde(rename = "type")]
    pub transaction_type: TransactionType,
    pub security_key: String,
    pub transactionid: String,
    pub void_reason: Option<String>,
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Condition {
    Abandoned,
    Canceled,
    Pendingsettlement,
    Pending,
    Failed,
    Complete,
    InProgress,
    Unknown,
}

#[derive(Debug, Deserialize)]
pub struct Transaction {
    pub condition: Condition,
    pub transaction_id: String,
}

#[derive(Debug, Deserialize)]
pub struct QueryResponse {
    pub transaction: Transaction,
}

impl <T> TryFrom<types::ResponseRouterData<api::PSync, GenericResponse, T, types::PaymentsResponseData>> for types::ResponseRouterData<api::PSync> {
    fn try_from(value: types::ResponseRouterData<api::PSync, GenericResponse, T, types::PaymentsResponseData>) -> Result<Self, Self::Error> {
        
    }
}

impl TryFrom<types::ResponseRouterData<GenericResponse>>
    for types::PaymentsSyncResponseRouterData<>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        items: types::PaymentsResponseData<GenericResponse>,
    ) -> Result<Self, Self::Error> {
        let response = items.;

        Ok(Self {
            status: enums::AttemptStatus::from(response.condition),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(response.transaction_id),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
            }),
            ..items.data
        })
    }
}

// REFUND :
// Type definition for RefundRequest
#[derive(Debug, Serialize)]
pub struct NmiRefundRequest {
    #[serde(rename = "type")]
    transaction_type: TransactionType,
    security_key: String,
    transactionid: String,
    amount: String,
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
            amount: format!("{}.00", item.refund_amount.to_string()),
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, GenericResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, GenericResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(item.response.response);
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.transactionid,
                refund_status,
            }),
            ..item.data
        })
    }
}

impl From<Response> for enums::RefundStatus {
    fn from(item: Response) -> Self {
        match item {
            Response::Approved => enums::RefundStatus::Success,
            Response::Declined | Response::Error => enums::RefundStatus::Failure,
        }
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, QueryResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, QueryResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(item.response.transaction.condition);
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.transaction.transaction_id,
                refund_status,
            }),
            ..item.data
        })
    }
}

impl From<Condition> for enums::RefundStatus {
    fn from(item: Condition) -> Self {
        match item {
            Condition::Abandoned | Condition::Canceled | Condition::Failed | Condition::Unknown => {
                enums::RefundStatus::Failure
            }
            Condition::Pendingsettlement | Condition::Pending | Condition::InProgress => {
                enums::RefundStatus::Pending
            }
            Condition::Complete => enums::RefundStatus::Success,
        }
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct NmiErrorResponse {
    pub error_code: String,
}

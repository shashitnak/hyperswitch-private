use common_utils::crypto::{self, GenerateDigest};
use error_stack::{ResultExt, IntoReport};
use rand::distributions::DistString;
use serde::{Deserialize, Serialize};
use masking::PeekInterface;

use super::{
    requests::{self, *},
    // response::{GlobalpayPaymentStatus, GlobalpayPaymentsResponse, GlobalpayRefreshTokenResponse},
};
use crate::{
    connector::utils::{self, CardData, PaymentsRequestData, AddressDetailsData},
    consts,
    core::errors,
    types::{self, api, storage::enums as storage_enums, ErrorResponse},
};

pub struct NmiAuthType {
    pub(super) api_key: String,
}

impl TryFrom<&types::ConnectorAuthType> for NmiAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::HeaderKey { api_key } = item {
            Ok(Self {
                api_key: api_key.to_string(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType.into())
        }
    }
}

fn error<T>() -> Result<T, error_stack::Report<errors::ConnectorError>> {
    Err(errors::ConnectorError::FailedToObtainIntegrationUrl).into_report()
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for NmiPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        use storage_enums::CaptureMethod::*;
        use api::payments::PaymentMethod::*;
        use PaymentType::*;
        let transaction_type = match item.request.capture_method {
            Some(Automatic) => TransactionType::Sale,
            Some(Manual) => TransactionType::Auth,
            _ => error()?
        };
        let security_key: NmiAuthType = (&item.connector_auth_type).try_into()?;

        let card = match item.request.payment_method_data {
            Card(card) => card,
            _ => error()?
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

    let address = item
        .address
        .billing
        .unwrap();
    
    let phone = address.phone.unwrap();
    let address = address.address.unwrap();

      Ok(NmiPaymentsRequest {
        transaction_type,
        security_key,
        ccnumber                : card.card_number.peek().to_string(),
        ccexp                   : card.card_exp_month.peek().to_string() + &card.card_exp_year.peek().to_string(),
        cvv                     : card.card_cvc.peek().to_string(),
        account_holder_type     : None,
        account_type            : None,
        sec_code                : None,
        amount                  : item.request.amount.to_string() + ".00",
        surcharge               : None,
        currency                : item.request.currency,
        payment                 : CreditCard,
        processor_id            : None,
        billing_method          : None,
        billing_number          : None,
        order_description       : item.description.clone(),
        orderid                 : None,
        first_name              : address.get_first_name().unwrap().peek().to_string(),
        last_name               : address.get_last_name().unwrap().peek().to_string(),
        address1                : address.get_line1().unwrap().peek().to_string(),
        address2                : address.get_line2().unwrap().peek().to_string(),
        city                    : address.get_city().unwrap().to_string(),
        state                   : None,
        zip                     : address.get_zip().unwrap().peek().to_string(),
        country                 : address.get_country().unwrap().to_string(),
        phone                   : phone.country_code.unwrap().clone() + &phone.number.unwrap().peek().to_string()
      })
    }
}

// impl TryFrom<&types::PaymentsCaptureRouterData> for PaymentsRequest {
//     type Error = error_stack::Report<errors::ConnectorError>;
//     fn try_from(value: &types::PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
//         Ok(Self {
//             amount: value
//                 .request
//                 .amount_to_capture
//                 .map(|amount| amount.to_string()),
//             ..Default::default()
//         })
//     }
// }

// impl TryFrom<&types::PaymentsCancelRouterData> for GlobalpayPaymentsRequest {
//     type Error = error_stack::Report<errors::ConnectorError>;
//     fn try_from(_value: &types::PaymentsCancelRouterData) -> Result<Self, Self::Error> {
//         Ok(Self::default())
//     }
// }

// pub struct GlobalpayAuthType {
//     pub app_id: String,
//     pub key: String,
// }

// impl TryFrom<&types::ConnectorAuthType> for GlobalpayAuthType {
//     type Error = error_stack::Report<errors::ConnectorError>;
//     fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
//         match auth_type {
//             types::ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
//                 app_id: key1.to_string(),
//                 key: api_key.to_string(),
//             }),
//             _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
//         }
//     }
// }

// impl TryFrom<GlobalpayRefreshTokenResponse> for types::AccessToken {
//     type Error = error_stack::Report<errors::ParsingError>;

//     fn try_from(item: GlobalpayRefreshTokenResponse) -> Result<Self, Self::Error> {
//         Ok(Self {
//             token: item.token,
//             expires: item.seconds_to_expire,
//         })
//     }
// }

// impl TryFrom<&types::RefreshTokenRouterData> for GlobalpayRefreshTokenRequest {
//     type Error = error_stack::Report<errors::ConnectorError>;

//     fn try_from(item: &types::RefreshTokenRouterData) -> Result<Self, Self::Error> {
//         let globalpay_auth = GlobalpayAuthType::try_from(&item.connector_auth_type)
//             .change_context(errors::ConnectorError::FailedToObtainAuthType)
//             .attach_printable("Could not convert connector_auth to globalpay_auth")?;

//         let nonce = rand::distributions::Alphanumeric.sample_string(&mut rand::thread_rng(), 12);
//         let nonce_with_api_key = format!("{}{}", nonce, globalpay_auth.key);
//         let secret_vec = crypto::Sha512
//             .generate_digest(nonce_with_api_key.as_bytes())
//             .change_context(errors::ConnectorError::RequestEncodingFailed)
//             .attach_printable("error creating request nonce")?;

//         let secret = hex::encode(secret_vec);

//         Ok(Self {
//             app_id: globalpay_auth.app_id,
//             nonce,
//             secret,
//             grant_type: "client_credentials".to_string(),
//         })
//     }
// }

// impl From<GlobalpayPaymentStatus> for enums::AttemptStatus {
//     fn from(item: GlobalpayPaymentStatus) -> Self {
//         match item {
//             GlobalpayPaymentStatus::Captured | GlobalpayPaymentStatus::Funded => Self::Charged,
//             GlobalpayPaymentStatus::Declined | GlobalpayPaymentStatus::Rejected => Self::Failure,
//             GlobalpayPaymentStatus::Preauthorized => Self::Authorized,
//             GlobalpayPaymentStatus::Reversed => Self::Voided,
//             GlobalpayPaymentStatus::Initiated | GlobalpayPaymentStatus::Pending => Self::Pending,
//         }
//     }
// }

// impl From<GlobalpayPaymentStatus> for enums::RefundStatus {
//     fn from(item: GlobalpayPaymentStatus) -> Self {
//         match item {
//             GlobalpayPaymentStatus::Captured | GlobalpayPaymentStatus::Funded => Self::Success,
//             GlobalpayPaymentStatus::Declined | GlobalpayPaymentStatus::Rejected => Self::Failure,
//             GlobalpayPaymentStatus::Initiated | GlobalpayPaymentStatus::Pending => Self::Pending,
//             _ => Self::Pending,
//         }
//     }
// }

// fn get_payment_response(
//     status: enums::AttemptStatus,
//     response: GlobalpayPaymentsResponse,
// ) -> Result<types::PaymentsResponseData, ErrorResponse> {
//     match status {
//         enums::AttemptStatus::Failure => Err(ErrorResponse {
//             message: response
//                 .payment_method
//                 .and_then(|pm| pm.message)
//                 .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
//             ..Default::default()
//         }),
//         _ => Ok(types::PaymentsResponseData::TransactionResponse {
//             resource_id: types::ResponseId::ConnectorTransactionId(response.id),
//             redirection_data: None,
//             redirect: false,
//             mandate_reference: None,
//             connector_metadata: None,
//         }),
//     }
// }

// impl<F, T>
//     TryFrom<types::ResponseRouterData<F, GlobalpayPaymentsResponse, T, types::PaymentsResponseData>>
//     for types::RouterData<F, T, types::PaymentsResponseData>
// {
//     type Error = error_stack::Report<errors::ConnectorError>;
//     fn try_from(
//         item: types::ResponseRouterData<
//             F,
//             GlobalpayPaymentsResponse,
//             T,
//             types::PaymentsResponseData,
//         >,
//     ) -> Result<Self, Self::Error> {
//         let status = enums::AttemptStatus::from(item.response.status);
//         Ok(Self {
//             status,
//             response: get_payment_response(status, item.response),
//             ..item.data
//         })
//     }
// }

// impl<F, T>
//     TryFrom<types::ResponseRouterData<F, GlobalpayRefreshTokenResponse, T, types::AccessToken>>
//     for types::RouterData<F, T, types::AccessToken>
// {
//     type Error = error_stack::Report<errors::ParsingError>;
//     fn try_from(
//         item: types::ResponseRouterData<F, GlobalpayRefreshTokenResponse, T, types::AccessToken>,
//     ) -> Result<Self, Self::Error> {
//         Ok(Self {
//             response: Ok(types::AccessToken {
//                 token: item.response.token,
//                 expires: item.response.seconds_to_expire,
//             }),
//             ..item.data
//         })
//     }
// }

// impl<F> TryFrom<&types::RefundsRouterData<F>> for requests::GlobalpayRefundRequest {
//     type Error = error_stack::Report<errors::ConnectorError>;
//     fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
//         Ok(Self {
//             amount: item.request.refund_amount.to_string(),
//         })
//     }
// }

// impl TryFrom<types::RefundsResponseRouterData<api::Execute, GlobalpayPaymentsResponse>>
//     for types::RefundExecuteRouterData
// {
//     type Error = error_stack::Report<errors::ConnectorError>;
//     fn try_from(
//         item: types::RefundsResponseRouterData<api::Execute, GlobalpayPaymentsResponse>,
//     ) -> Result<Self, Self::Error> {
//         Ok(Self {
//             response: Ok(types::RefundsResponseData {
//                 connector_refund_id: item.response.id,
//                 refund_status: enums::RefundStatus::from(item.response.status),
//             }),
//             ..item.data
//         })
//     }
// }

// impl TryFrom<types::RefundsResponseRouterData<api::RSync, GlobalpayPaymentsResponse>>
//     for types::RefundsRouterData<api::RSync>
// {
//     type Error = error_stack::Report<errors::ConnectorError>;
//     fn try_from(
//         item: types::RefundsResponseRouterData<api::RSync, GlobalpayPaymentsResponse>,
//     ) -> Result<Self, Self::Error> {
//         Ok(Self {
//             response: Ok(types::RefundsResponseData {
//                 connector_refund_id: item.response.id,
//                 refund_status: enums::RefundStatus::from(item.response.status),
//             }),
//             ..item.data
//         })
//     }
// }

// #[derive(Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
// pub struct GlobalpayErrorResponse {
//     pub error_code: String,
//     pub detailed_error_code: String,
//     pub detailed_error_description: String,
// }
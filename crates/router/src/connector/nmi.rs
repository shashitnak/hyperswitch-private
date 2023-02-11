mod transformers;

use std::fmt::Debug;

use error_stack::{IntoReport, ResultExt};
use transformers as nmi;
use serde::{Deserialize, Serialize};
use crate::{
    configs::settings,
    core::{
        errors::{self, CustomResult},
        payments,
    },
    headers, logger,
    services::{self, ConnectorIntegration},
    types::{
        self,
        api::{self, ConnectorCommon, ConnectorCommonExt},
        ErrorResponse, Response,
    },
    utils::{self, BytesExt}, connector::nmi::transformers::{NmiPaymentsRequest, NmiCaptureRequest},
};

use self::transformers::NmiSyncRequest;

#[derive(Debug, Clone)]
pub struct Nmi;

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Nmi
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    // fn build_headers(
    //     &self,
    //     _req: &types::RouterData<Flow, Request, Response>,
    //     _connectors: &settings::Connectors,
    // ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
    //     todo!()
    // }
}

impl ConnectorCommon for Nmi {
    fn id(&self) -> &'static str {
        "nmi"
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/x-www-form-urlencoded"
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.nmi.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &types::ConnectorAuthType,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let auth: nmi::NmiAuthType = auth_type
            .try_into()
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(headers::AUTHORIZATION.to_string(), auth.api_key)])
    }
}

impl api::Payment for Nmi {}

impl api::PreVerify for Nmi {}
impl ConnectorIntegration<api::Verify, types::VerifyRequestData, types::PaymentsResponseData>
    for Nmi
{
}

impl api::PaymentVoid for Nmi {}

impl ConnectorIntegration<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
    for Nmi
{
    fn get_headers(
        &self,
        req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &types::PaymentsCancelRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}{}",
            self.base_url(_connectors),
            "api/transact.php"
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsCancelRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let auth = req.connector_auth_type.clone();
        let request = req.request.clone();
        let nmi_req = nmi::NmiCancelRequest::try_from((&request, auth))?;
        let nmi_req = utils::Encode::<nmi::NmiCancelRequest>::encode(&nmi_req)
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        logger::debug!(nmi_req=?nmi_req);
        Ok(Some(nmi_req))
    }

    fn build_request(
        &self,
        req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let mut headers = types::PaymentsVoidType::get_headers(
            self, req, connectors,
        )?;
        headers.push(("Content-Type".to_string(), "application/x-www-form-urlencoded".to_string()));
        let body = self.get_request_body(req)?;

        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsVoidType::get_url(self, req, connectors)?)
                .headers(headers)
                .body(body)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCancelRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsCancelRouterData, errors::ConnectorError> {
        logger::debug!(payment_auth_response=?res);

        #[derive(Deserialize)]
        struct Response {
            response: usize,
            #[serde(rename = "type")]
            transaction_type: nmi::TransactionType,
            transactionid: String
        }

        let raw_string: String = res.response.iter().map(|&x| x as char).collect();
        let response: Response = serde_urlencoded::from_str(&raw_string).unwrap();

        let response = match response {
            Response { response: 1, transaction_type: _, transactionid } =>
                nmi::NmiPaymentsResponse { status: nmi::NmiPaymentStatus::Canceled, id: transactionid },
            Response { response: _, transaction_type: _, transactionid } =>
                nmi::NmiPaymentsResponse { status: nmi::NmiPaymentStatus::VoidFailed, id: transactionid }
        };

        // let raw_string = res
        // .response
        // .into_iter()
        // .map(|x| x as char)
        // .collect::<String>();
        // let items = raw_string
        //     .split("&");
        // let mut status = nmi::NmiPaymentStatus::Failed;
        // let mut tid = "".to_string();

        // for item in items {
        //     let mut pair = item.split("=");
        //     let key = pair.next().unwrap();
        //     let val = pair.next().unwrap();
        //     match key {
        //         "response" => match val {
        //             "1" => status = nmi::NmiPaymentStatus::Succeeded,
        //             _ => ()
        //         },
        //         "transactionid" => tid = val.to_string(),
        //         _ => ()
        //     }
        // }

        logger::debug!(nmipayments_create_response=?response);
        types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        }
        .try_into()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl api::ConnectorAccessToken for Nmi {}

impl ConnectorIntegration<api::AccessTokenAuth, types::AccessTokenRequestData, types::AccessToken>
    for Nmi
{
}

impl api::PaymentSync for Nmi {}
impl ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Nmi
{
    fn get_headers(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::PaymentsSyncRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}{}",
            self.base_url(_connectors),
            "api/query.php"
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsSyncRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let auth = req.connector_auth_type.clone();
        let request = req.request.clone();
        let nmi_req = NmiSyncRequest::try_from((&request, auth))?;
        let nmi_req = utils::Encode::<nmi::NmiSyncRequest>::encode(&nmi_req)
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        logger::debug!(nmi_req=?nmi_req);
        Ok(Some(nmi_req))
    }

    fn build_request(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let mut headers = types::PaymentsSyncType::get_headers(
            self, req, connectors,
        )?;
        headers.push(("Content-Type".to_string(), "application/x-www-form-urlencoded".to_string()));
        let body = self.get_request_body(req)?;
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsSyncType::get_url(self, req, connectors)?)
                .headers(headers)
                .body(body)
                .build()
        ))
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }

    fn handle_response(
        &self,
        data: &types::PaymentsSyncRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsSyncRouterData, errors::ConnectorError> {
        logger::debug!(payment_sync_response=?res);
        // let sync_data = PaymentsSyncData {
        //     connector_transaction_id = 
        //     pub encoded_data: Option<String>,
        //     pub capture_method: Option<storage_enums::CaptureMethod>,
        // }

        // use xmlparser::Tokenizer;
        // #[derive(Debug, Deserialize)]
        // struct Res {
        //     nm_response: NmResponse
        // }

        // #[derive(Debug, Deserialize)]
        // struct NmResponse {
        //     transaction: Transaction
        // }
        
        // #[derive(Debug, Deserialize)]
        // struct Transaction {
        //     transaction_id: String,
        //     condition: nmi::NmiSyncResponseStatus
        // }
        use regex::Regex;

        let re1 = Regex::new("<transaction_id>(.*)</transaction_id>").unwrap();
        let re2 = Regex::new("<condition>(.*)</condition>").unwrap();

        let mut transaction_id = None;
        let mut condition = None;

        let raw_str: String = res.response.iter().map(|&x| x as char).collect();

        for tid in re1.captures_iter(&raw_str) {
            transaction_id = Some((&tid[1]).to_string());
            println!("transaction_id={transaction_id:?}");
        }
        use nmi::NmiSyncResponseStatus::*;
        for cid in re2.captures_iter(&raw_str) {
            condition = Some(match &cid[1] {
                "pendingsettlement" => pendingSettlement,
                "pending" => pending,
                "failed" => failed,
                "canceled" => canceled,
                "complete" => complete,
                _ => unknown
            });
            println!("condition={transaction_id:?}");
        }
        // println!("bolaaaaaaa {:?}",raw_str);
        //println!("tokenizer = {:?}", Tokenizer::from(raw_str.as_str()));
        // let something= Tokenizer::from(raw_str.as_str());
        // println!("something = {:?}", something);
        // let transaction = response.nm_response.transaction;
        let sync_res = nmi::NmiSyncResponse {
            condition: condition.unwrap(),
            transaction_id: transaction_id.unwrap()
        };
        logger::debug!(sync_res=?sync_res);

        // let response: nmi::NmiPaymentsResponse = res
        //     .response
        //     .parse_struct("nmi PaymentsResponse")
        //     .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
            types::RouterData::try_from(types::ResponseRouterData {
                response: sync_res,
                data: data.clone(),
                http_code: res.status_code,
            })
            .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }
}

impl api::PaymentCapture for Nmi {}
impl ConnectorIntegration<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
    for Nmi
{
    fn get_headers(
        &self,
        req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &types::PaymentsCaptureRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}{}",
            self.base_url(_connectors),
            "api/transact.php"
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsCaptureRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let auth = req.connector_auth_type.clone();
        let request = req.request.clone();
        let nmi_req = NmiCaptureRequest::try_from((&request, auth))?;
        let nmi_req = utils::Encode::<nmi::NmiCaptureRequest>::encode(&nmi_req)
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        logger::debug!(nmi_req=?nmi_req);
        Ok(Some(nmi_req))
    }

    fn build_request(
        &self,
        req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let mut headers = types::PaymentsCaptureType::get_headers(
            self, req, connectors,
        )?;
        headers.push(("Content-Type".to_string(), "application/x-www-form-urlencoded".to_string()));
        let body = self.get_request_body(req)?;

        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsCaptureType::get_url(self, req, connectors)?)
                .headers(headers)
                .body(body)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCaptureRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsCaptureRouterData, errors::ConnectorError> {
        logger::debug!(payment_auth_response=?res);

        #[derive(Deserialize)]
        struct Response {
            response: usize,
            #[serde(rename = "type")]
            transaction_type: nmi::TransactionType,
            transactionid: String
        }

        let raw_string: String = res.response.iter().map(|&x| x as char).collect();
        let response: Response = serde_urlencoded::from_str(&raw_string).unwrap();

        let response = match response {
            Response { response: 1, transaction_type: _, transactionid } =>
                nmi::NmiPaymentsResponse { status: nmi::NmiPaymentStatus::Captured, id: transactionid },
            Response { response: _, transaction_type: _, transactionid } =>
                nmi::NmiPaymentsResponse { status: nmi::NmiPaymentStatus::Failed, id: transactionid }
        };

        // let raw_string = res
        // .response
        // .into_iter()
        // .map(|x| x as char)
        // .collect::<String>();
        // let items = raw_string
        //     .split("&");
        // let mut status = nmi::NmiPaymentStatus::Failed;
        // let mut tid = "".to_string();

        // for item in items {
        //     let mut pair = item.split("=");
        //     let key = pair.next().unwrap();
        //     let val = pair.next().unwrap();
        //     match key {
        //         "response" => match val {
        //             "1" => status = nmi::NmiPaymentStatus::Succeeded,
        //             _ => ()
        //         },
        //         "transactionid" => tid = val.to_string(),
        //         _ => ()
        //     }
        // }

        logger::debug!(nmipayments_create_response=?response);
        types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        }
        .try_into()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl api::PaymentSession for Nmi {}

impl ConnectorIntegration<api::Session, types::PaymentsSessionData, types::PaymentsResponseData>
    for Nmi
{
    //TODO: implement sessions flow
}

impl api::PaymentAuthorize for Nmi {}

impl ConnectorIntegration<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
    for Nmi
{
    fn get_headers(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::PaymentsAuthorizeRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}/{}",
            self.base_url(_connectors),
            "api/transact.php"
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {

        let nmi_req = utils::Encode::<nmi::NmiPaymentsRequest>::convert_and_url_encode(req)
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(nmi_req))
    }

    fn build_request(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let mut headers = types::PaymentsAuthorizeType::get_headers(
                    self, req, connectors,
                )?;
        headers.push(("Content-Type".to_string(), "application/x-www-form-urlencoded".to_string()));
        let body = types::PaymentsAuthorizeType::get_request_body(self, req)?;
        logger::debug!(body=?body);
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsAuthorizeType::get_url(
                    self, req, connectors,
                )?)
                .headers(headers)
                .body(body)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsAuthorizeRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsAuthorizeRouterData, errors::ConnectorError> {
        logger::debug!(payment_auth_response=?res);
        let raw_string = res
        .response
        .into_iter()
        .map(|x| x as char)
        .collect::<String>();
        // let items = raw_string
        //     .split("&");
        // let mut status = nmi::NmiPaymentStatus::Failed;
        // let mut tid = "".to_string();

        // for item in items {
        //     let mut pair = item.split("=");
        //     let key = pair.next().unwrap();
        //     let val = pair.next().unwrap();
        //     match key {
        //         "response" => match val {
        //             "1" => status = nmi::NmiPaymentStatus::Succeeded,
        //             _ => ()
        //         },
        //         "transactionid" => tid = val.to_string(),
        //         _ => ()
        //     }
        // }

        use serde::Deserialize;

        #[derive(Deserialize)]
        struct Response {
            response: usize,
            #[serde(rename = "type")]
            transaction_type: nmi::TransactionType,
            transactionid: String
        }

        let response: Response = serde_urlencoded::from_str(&raw_string).unwrap();
        use nmi::TransactionType::*;
        use nmi::NmiPaymentStatus::*;

    //     Authorised,
    // Captured,
    // Failed,
    // #[default]
    // Processing,
    // Settled

        let response = match response {
            Response { response: 1, transaction_type: Sale, transactionid } => nmi::NmiPaymentsResponse { status: Captured, id: transactionid },
            Response { response: 1, transaction_type: Auth, transactionid } => nmi::NmiPaymentsResponse { status: Authorised, id: transactionid },
            Response { response: _, transaction_type: _, transactionid } => nmi::NmiPaymentsResponse { status: Failed, id: transactionid }
        };

        // let response: nmi::NmiPaymentsResponse = res
        //     .response
        //     .parse_struct("PaymentIntentResponse")
        //     .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::debug!(nmipayments_create_response=?response);
        types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        }
        .try_into()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl api::Refund for Nmi {}
impl api::RefundExecute for Nmi {}
impl api::RefundSync for Nmi {}

impl ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData> for Nmi {
    fn get_headers(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::RefundsRouterData<api::Execute>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        todo!()
    }

    fn get_request_body(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let nmi_req = utils::Encode::<nmi::NmiRefundRequest>::convert_and_encode(req)
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(nmi_req))
    }

    fn build_request(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::RefundExecuteType::get_url(self, req, connectors)?)
            .headers(types::RefundExecuteType::get_headers(
                self, req, connectors,
            )?)
            .body(types::RefundExecuteType::get_request_body(self, req)?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &types::RefundsRouterData<api::Execute>,
        res: Response,
    ) -> CustomResult<types::RefundsRouterData<api::Execute>, errors::ConnectorError> {
        logger::debug!(target: "router::connector::nmi", response=?res);
        let response: nmi::RefundResponse = res
            .response
            .parse_struct("nmi RefundResponse")
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        }
        .try_into()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData> for Nmi {
    fn get_headers(
        &self,
        req: &types::RefundSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::RefundSyncRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        todo!()
    }

    fn build_request(
        &self,
        req: &types::RefundSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Get)
                .url(&types::RefundSyncType::get_url(self, req, connectors)?)
                .headers(types::RefundSyncType::get_headers(self, req, connectors)?)
                .body(types::RefundSyncType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::RefundSyncRouterData,
        res: Response,
    ) -> CustomResult<types::RefundSyncRouterData, errors::ConnectorError> {
        logger::debug!(target: "router::connector::nmi", response=?res);
        let response: nmi::RefundResponse = res
            .response
            .parse_struct("nmi RefundResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        }
        .try_into()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Nmi {
    fn get_webhook_object_reference_id(
        &self,
        _body: &[u8],
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }

    fn get_webhook_event_type(
        &self,
        _body: &[u8],
    ) -> CustomResult<api::IncomingWebhookEvent, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }

    fn get_webhook_resource_object(
        &self,
        _body: &[u8],
    ) -> CustomResult<serde_json::Value, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }
}

impl services::ConnectorRedirectResponse for Nmi {
    fn get_flow_type(
        &self,
        _query_params: &str,
    ) -> CustomResult<payments::CallConnectorAction, errors::ConnectorError> {
        Ok(payments::CallConnectorAction::Trigger)
    }
}

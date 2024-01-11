use std::fmt;

use crate::agent::{
    provisioning_service_server::ProvisioningService, Empty, ProvisioningCodeRequest,
    ProvisioningCodeResponse, ProvisioningStatusResponse,
};
use crate::agent::{DeProvisioningStatusResponse, PingResponse};
use anyhow::Result;
use provisioning::errors::{map_provisioning_error_to_tonic, ProvisioningError};
use provisioning::handler::ProvisioningMessage;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tonic::{Request, Response, Status};

#[derive(Serialize, Deserialize)]
pub enum MathsError {
    DivByZero(i32, i32),
}

impl fmt::Display for MathsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MathsError::DivByZero(error, code) => write!(f, "DivByZero: {} {}", error, code),
        }
    }
}
#[derive(Debug)]
pub struct ProvisioningServiceHandler {
    provisioning_tx: mpsc::Sender<provisioning::handler::ProvisioningMessage>,
}

impl ProvisioningServiceHandler {
    // Add an opening brace here
    pub fn new(provisioning_tx: mpsc::Sender<ProvisioningMessage>) -> Self {
        Self { provisioning_tx }
    }
}

#[tonic::async_trait]
impl ProvisioningService for ProvisioningServiceHandler {
    async fn ping(&self, _request: Request<Empty>) -> Result<Response<PingResponse>, Status> {
        let provisioning_tx = self.provisioning_tx.clone();
        // send message
        let (tx, rx) = oneshot::channel();
        match provisioning_tx
            .send(ProvisioningMessage::Ping { reply_to: tx })
            .await
        {
            Ok(_) => {}
            Err(e) => {
                return Err(Status::unavailable("provisioning service unavailable").into());
            }
        }
        // TODO handle
        let reply =
            rx.await.unwrap_or(Err(
                Status::unavailable("provisioning service unavailable").into()
            ));
        if reply.is_ok() {
            let response = reply.unwrap();
            return Ok(Response::new(PingResponse {
                code: response.code,
                message: response.message,
            }));
        } else {
            match reply.unwrap_err().downcast::<ProvisioningError>() {
                Ok(e) => {
                    let status = map_provisioning_error_to_tonic(
                        e.code,
                        e.code.to_string() + " - " + e.message.as_str(),
                    );
                    Err(status)
                }
                Err(e) => Err(Status::internal(e.to_string()).into()),
            }
        }
    }

    async fn generate_code(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<ProvisioningCodeResponse>, Status> {
        let provisioning_tx = self.provisioning_tx.clone();

        // send message
        let (tx, rx) = oneshot::channel();
        let _ = provisioning_tx
            .send(ProvisioningMessage::GenerateCode { reply_to: tx })
            .await;

        // TODO handle
        let reply =
            rx.await.unwrap_or(Err(
                Status::unavailable("provisioning service unavailable").into()
            ));

        if reply.is_ok() {
            let code = reply.unwrap();
            Ok(Response::new(ProvisioningCodeResponse { code }))
        } else {
            Err(Status::from_error(reply.unwrap_err().into()))
        }
    }

    async fn provision_by_code(
        &self,
        request: Request<ProvisioningCodeRequest>,
    ) -> Result<Response<ProvisioningStatusResponse>, Status> {
        let provisioning_tx = self.provisioning_tx.clone();

        // send message
        let (tx, rx) = oneshot::channel();
        let _ = provisioning_tx
            .send(ProvisioningMessage::ProvisionByCode {
                code: request.into_inner().code,
                reply_to: tx,
            })
            .await;

        let reply =
            rx.await.unwrap_or(Err(
                Status::unavailable("provisioning service unavailable").into()
            ));

        if reply.is_ok() {
            let result = reply.unwrap();
            Ok(Response::new(ProvisioningStatusResponse {
                success: result,
            }))
        } else {
            Err(Status::from_error(reply.unwrap_err().into()))
        }
    }
    async fn deprovision(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<DeProvisioningStatusResponse>, Status> {
        let provisioning_tx = self.provisioning_tx.clone();

        // send message
        let (tx, rx) = oneshot::channel();
        let _ = provisioning_tx
            .send(ProvisioningMessage::Deprovision { reply_to: tx })
            .await;

        // TODO handle
        let reply =
            rx.await.unwrap_or(Err(
                Status::unavailable("provisioning service unavailable").into()
            ));

        if reply.is_ok() {
            let success = reply.unwrap();
            Ok(Response::new(DeProvisioningStatusResponse { success }))
        } else {
            Err(Status::from_error(reply.unwrap_err().into()))
        }
    }
}

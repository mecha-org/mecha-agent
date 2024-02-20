use crate::agent::{
    provisioning_service_server::ProvisioningService, Empty, ProvisioningCodeRequest,
    ProvisioningCodeResponse, ProvisioningStatusResponse,
};
use crate::agent::{DeProvisioningStatusResponse, PingResponse};
use anyhow::Result;
use channel::{recv_with_custom_timeout, recv_with_timeout};
use provisioning::errors::{map_provisioning_error_to_tonic, ProvisioningError};
use provisioning::handler::ProvisioningMessage;
use tokio::sync::mpsc::{self};
use tokio::sync::oneshot;
use tonic::{Request, Response, Status};
use tracing::error;

const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");
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
                error!(
                    func = "ping",
                    package = PACKAGE_NAME,
                    "error while pinging machine num - {} error - {}",
                    1000,
                    e
                );
                return Err(Status::unavailable("provisioning service unavailable").into());
            }
        }
        let result = match recv_with_timeout(rx).await {
            Ok(res) => res,
            Err(err) => {
                error!(
                    func = "ping",
                    package = PACKAGE_NAME,
                    "error while  receive pinging machine num - {} error - {}",
                    1000,
                    err
                );
                match err.downcast::<ProvisioningError>() {
                    Ok(e) => {
                        let status = map_provisioning_error_to_tonic(
                            e.code,
                            e.code.to_string() + " - " + e.message.as_str(),
                        );
                        return Err(status);
                    }
                    Err(e) => return Err(Status::internal(e.to_string()).into()),
                }
            }
        };
        return Ok(Response::new(PingResponse {
            code: result.code,
            message: result.message,
        }));
    }

    async fn generate_code(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<ProvisioningCodeResponse>, Status> {
        let provisioning_tx = self.provisioning_tx.clone();

        // send message
        let (tx, rx) = oneshot::channel();
        match provisioning_tx
            .send(ProvisioningMessage::GenerateCode { reply_to: tx })
            .await
        {
            Ok(_) => {}
            Err(e) => {
                return Err(Status::unavailable("provisioning service unavailable").into());
            }
        }

        let code = match recv_with_timeout(rx).await {
            Ok(code) => code,
            Err(err) => return Err(Status::unavailable("provisioning service unavailable").into()),
        };

        Ok(Response::new(ProvisioningCodeResponse { code }))
    }

    async fn provision_by_code(
        &self,
        request: Request<ProvisioningCodeRequest>,
    ) -> Result<Response<ProvisioningStatusResponse>, Status> {
        let provisioning_tx = self.provisioning_tx.clone();

        // send message
        let (tx, rx) = oneshot::channel();
        match provisioning_tx
            .send(ProvisioningMessage::ProvisionByCode {
                code: request.into_inner().code,
                reply_to: tx,
            })
            .await
        {
            Ok(_) => {}
            Err(e) => {
                error!(
                    func = "provision_by_code",
                    package = PACKAGE_NAME,
                    "error while provision machine by code num {}, error - {}",
                    1001,
                    e
                );
                return Err(Status::unavailable("provisioning service unavailable").into());
            }
        }
        let result = match recv_with_custom_timeout(30000, rx).await {
            Ok(res) => res,
            Err(err) => {
                error!(
                    func = "provision_by_code",
                    package = PACKAGE_NAME,
                    "error while provision machine by code num - {} error - {}",
                    1002,
                    err
                );
                match err.downcast::<ProvisioningError>() {
                    Ok(e) => {
                        let status = map_provisioning_error_to_tonic(
                            e.code,
                            e.code.to_string() + " - " + e.message.as_str(),
                        );
                        return Err(status);
                    }
                    Err(e) => return Err(Status::internal(e.to_string()).into()),
                }
            }
        };
        Ok(Response::new(ProvisioningStatusResponse {
            success: result,
        }))
    }
    async fn deprovision(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<DeProvisioningStatusResponse>, Status> {
        let provisioning_tx = self.provisioning_tx.clone();

        // send message
        let (tx, rx) = oneshot::channel();
        match provisioning_tx
            .send(ProvisioningMessage::Deprovision { reply_to: tx })
            .await
        {
            Ok(_) => {}
            Err(e) => {
                error!(
                    func = "deprovision",
                    package = PACKAGE_NAME,
                    "error while deprovision machine num - {} error - {}",
                    1003,
                    e
                );
                return Err(Status::unavailable("provisioning service unavailable").into());
            }
        }

        let success = match recv_with_timeout(rx).await {
            Ok(res) => res,
            Err(err) => {
                error!(
                    func = "deprovision",
                    package = PACKAGE_NAME,
                    "error while deprovision machine num - {} error - {}",
                    1004,
                    err
                );
                match err.downcast::<ProvisioningError>() {
                    Ok(e) => {
                        let status = map_provisioning_error_to_tonic(
                            e.code,
                            e.code.to_string() + " - " + e.message.as_str(),
                        );
                        return Err(status);
                    }
                    Err(e) => return Err(Status::internal(e.to_string()).into()),
                }
            }
        };
        Ok(Response::new(DeProvisioningStatusResponse { success }))
    }
}

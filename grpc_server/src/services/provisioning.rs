use crate::agent::DeProvisioningStatusResponse;
use crate::agent::{
    provisioning_service_server::ProvisioningService, Empty, ProvisioningCodeRequest,
    ProvisioningCodeResponse, ProvisioningStatusResponse,
};
use anyhow::Result;
use provisioning::handler::ProvisioningMessage;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tonic::{Request, Response, Status};

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

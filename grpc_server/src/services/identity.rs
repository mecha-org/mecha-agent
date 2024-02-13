use crate::agent::identity_service_server::IdentityService;
use crate::agent::GetMachineCertRequest;
use crate::agent::GetMachineCertResponse;
use crate::agent::GetMachineIdRequest;
use crate::agent::GetMachineIdResponse;
use crate::agent::GetProvisionStatusRequest;
use crate::agent::GetProvisionStatusResponse;
use anyhow::Result;
use identity::handler::IdentityMessage;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tonic::{Request, Response, Status};

#[derive(Debug)]
pub struct IdentityServiceHandler {
    identity_tx: mpsc::Sender<identity::handler::IdentityMessage>,
}

impl IdentityServiceHandler {
    // Add an opening brace here
    pub fn new(identity_tx: mpsc::Sender<IdentityMessage>) -> Self {
        Self { identity_tx }
    }
}

#[tonic::async_trait]
impl IdentityService for IdentityServiceHandler {
    async fn get_machine_id(
        &self,
        _request: Request<GetMachineIdRequest>,
    ) -> Result<Response<GetMachineIdResponse>, Status> {
        let identity_tx = self.identity_tx.clone();

        // send message
        let (tx, rx) = oneshot::channel();
        let _ = identity_tx
            .send(IdentityMessage::GetMachineId { reply_to: tx })
            .await;

        // TODO handle
        let reply = rx.await.unwrap_or(Err(
            Status::unavailable("identity service unavailable").into()
        ));

        if reply.is_ok() {
            let machine_id = reply.unwrap();
            Ok(Response::new(GetMachineIdResponse { machine_id }))
        } else {
            Err(Status::from_error(reply.unwrap_err().into()))
        }
    }

    async fn get_provision_status(
        &self,
        _request: Request<GetProvisionStatusRequest>,
    ) -> Result<Response<GetProvisionStatusResponse>, Status> {
        let identity_tx = self.identity_tx.clone();

        // send message
        let (tx, rx) = oneshot::channel();
        let _ = identity_tx
            .send(IdentityMessage::GetProvisionStatus { reply_to: tx })
            .await;

        // TODO handle
        let reply = rx.await.unwrap_or(Err(
            Status::unavailable("identity service unavailable").into()
        ));

        if reply.is_ok() {
            let status = reply.unwrap();
            Ok(Response::new(GetProvisionStatusResponse { status }))
        } else {
            Err(Status::from_error(reply.unwrap_err().into()))
        }
    }
    async fn get_machine_cert(
        &self,
        _request: Request<GetMachineCertRequest>,
    ) -> Result<Response<GetMachineCertResponse>, Status> {
        let identity_tx = self.identity_tx.clone();

        // send message
        let (tx, rx) = oneshot::channel();
        let _ = identity_tx
            .send(IdentityMessage::GetMachineCert { reply_to: tx })
            .await;

        // TODO handle
        let reply = rx.await.unwrap_or(Err(
            Status::unavailable("identity service unavailable").into()
        ));

        if reply.is_ok() {
            let machine_cert = reply.unwrap();
            Ok(Response::new(GetMachineCertResponse {
                common_name: machine_cert.common_name,
                expiry: machine_cert.expiry,
                fingerprint: machine_cert.fingerprint,
                public_cert: machine_cert.public_cert,
                ca_bundle: machine_cert.ca_bundle,
                root_cert: machine_cert.root_cert,
            }))
        } else {
            Err(Status::from_error(reply.unwrap_err().into()))
        }
    }
}

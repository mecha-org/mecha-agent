use crate::agent::settings_service_server::SettingsService;
use crate::agent::{
    GetSettingsRequest, GetSettingsResponse, SetSettingsRequest, SetSettingsResponse,
};
use anyhow::Result;
use provisioning::handler::ProvisioningMessage;
use settings::handler::SettingMessage;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tonic::{Request, Response, Status};

#[derive(Debug)]
pub struct SettingsServiceHandler {
    settings_tx: mpsc::Sender<SettingMessage>,
    provisioning_tx: mpsc::Sender<ProvisioningMessage>,
}

impl SettingsServiceHandler {
    // Add an opening brace here
    pub fn new(
        settings_tx: mpsc::Sender<SettingMessage>,
        provisioning_tx: mpsc::Sender<ProvisioningMessage>,
    ) -> Self {
        Self {
            settings_tx,
            provisioning_tx,
        }
    }
}

#[tonic::async_trait]
impl SettingsService for SettingsServiceHandler {
    async fn get(
        &self,
        request: Request<GetSettingsRequest>,
    ) -> Result<Response<GetSettingsResponse>, Status> {
        let key = request.into_inner().key;
        let settings_tx = self.settings_tx.clone();
        // send message
        let (tx, rx) = oneshot::channel();

        let _ = settings_tx
            .send(SettingMessage::GetSettingsByKey {
                reply_to: tx,
                key: key,
            })
            .await;

        // TODO handle
        let reply = rx.await.unwrap_or(Err(
            Status::unavailable("settings service unavailable").into()
        ));
        if reply.is_ok() {
            let response = reply.unwrap();
            Ok(Response::new(GetSettingsResponse { value: response }))
        } else {
            Err(Status::from_error(reply.unwrap_err().into()))
        }
    }
    async fn set(
        &self,
        request: Request<SetSettingsRequest>,
    ) -> Result<Response<SetSettingsResponse>, Status> {
        let settings = request.into_inner().settings;
        let settings_tx = self.settings_tx.clone();
        // send message
        let (tx, rx) = oneshot::channel();
        let _ = settings_tx
            .send(SettingMessage::SetSettings {
                reply_to: tx,
                settings: settings,
            })
            .await;

        let reply = rx.await.unwrap_or(Err(
            Status::unavailable("settings service unavailable").into()
        ));
        if reply.is_ok() {
            let response = reply.unwrap();
            Ok(Response::new(SetSettingsResponse { success: response }))
        } else {
            Err(Status::from_error(reply.unwrap_err().into()))
        }
    }
}

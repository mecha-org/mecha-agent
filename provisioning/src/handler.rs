use agent_settings::{read_settings_yml, AgentSettings};
use anyhow::{bail, Result};
use async_trait::async_trait;
use events::Event;
use services::{ServiceHandler, ServiceStatus};
use tokio::select;
use tokio::sync::{broadcast, mpsc, oneshot};

use crate::errors::{ProvisioningError, ProvisioningErrorCodes};
use crate::service::{de_provision, generate_code, provision_by_code};

pub struct ProvisioningHandler {
    event_tx: broadcast::Sender<Event>,
    status: ServiceStatus,
}

pub enum ProvisioningMessage {
    GenerateCode {
        reply_to: oneshot::Sender<Result<String>>,
    },
    ProvisionByCode {
        code: String,
        reply_to: oneshot::Sender<Result<bool>>,
    },
    ProvisionByManifest {
        manifest: String,
        reply_to: oneshot::Sender<Option<bool>>,
    },
    Deprovision {
        reply_to: oneshot::Sender<Result<bool>>,
    },
}

pub struct ProvisioningOptions {
    pub event_tx: broadcast::Sender<Event>,
}

impl ProvisioningHandler {
    pub fn new(options: ProvisioningOptions) -> Self {
        Self {
            event_tx: options.event_tx,
            status: ServiceStatus::INACTIVE,
        }
    }

    pub async fn run(&mut self, mut message_rx: mpsc::Receiver<ProvisioningMessage>) {
        // start the service
        let _ = &self.start().await;

        loop {
            select! {
                msg = message_rx.recv() => {
                    if msg.is_none() {
                        continue;
                    }

                    match msg.unwrap() {
                        ProvisioningMessage::GenerateCode { reply_to } => {
                            // let code = generate_error();
                            let code = generate_code();
                            reply_to.send(code);
                        }
                        ProvisioningMessage::ProvisionByCode { code, reply_to } => {
                            let status = provision_by_code(code, self.event_tx.clone()).await;
                            let _ = reply_to.send(status);
                        }
                        ProvisioningMessage::ProvisionByManifest { manifest, reply_to } => {
                            println!("Provisioning by manifest: {}", manifest);
                            let _ = reply_to.send(Some(true));
                        }
                        ProvisioningMessage::Deprovision { reply_to } => {
                            let status = de_provision(self.event_tx.clone());
                            let _ = reply_to.send(status);
                        }
                    };
                }
            }
        }
    }
}

#[async_trait]
impl ServiceHandler for ProvisioningHandler {
    async fn start(&mut self) -> Result<bool> {
        self.status = ServiceStatus::STARTED;
        Ok(true)
    }

    async fn stop(&mut self) -> Result<bool> {
        self.status = ServiceStatus::STOPPED;
        Ok(true)
    }

    fn get_status(&self) -> anyhow::Result<ServiceStatus> {
        Ok(self.status)
    }

    fn is_stopped(&self) -> Result<bool> {
        Ok(self.status == ServiceStatus::STOPPED)
    }

    fn is_started(&self) -> Result<bool> {
        Ok(self.status == ServiceStatus::STARTED)
    }
}

fn generate_error() -> Result<String> {
    bail!(ProvisioningError::new(
        ProvisioningErrorCodes::ManifestLookupBadRequestError,
        String::from("Dummy Error"),
        false // Not reporting bad request errors
    ))
}
fn get_settings() -> AgentSettings {
    let settings: AgentSettings = match read_settings_yml() {
        Ok(settings) => settings,
        Err(_) => AgentSettings::default(),
    };
    settings
}

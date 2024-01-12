use anyhow::{bail, Result};
use events::Event;
use identity::handler::IdentityMessage;
use messaging::handler::MessagingMessage;
use services::{ServiceHandler, ServiceStatus};
use settings::handler::SettingMessage;
use std::process::Child;
use tokio::{
    select,
    sync::{broadcast, mpsc, oneshot},
};
use tonic::async_trait;
use tracing::{debug, error, info};

use crate::{
    errors::{NetworkingError, NetworkingErrorCodes},
    service::start,
};
pub struct NetworkingHandler {
    event_tx: broadcast::Sender<Event>,
    messaging_tx: mpsc::Sender<MessagingMessage>,
    identity_tx: mpsc::Sender<IdentityMessage>,
    setting_tx: mpsc::Sender<SettingMessage>,
    status: ServiceStatus,
    nebula_process: Option<tokio::process::Child>,
}
pub struct NetworkingOptions {
    pub event_tx: broadcast::Sender<Event>,
    pub messaging_tx: mpsc::Sender<MessagingMessage>,
    pub identity_tx: mpsc::Sender<IdentityMessage>,
    pub setting_tx: mpsc::Sender<SettingMessage>,
}

pub enum NetworkingMessage {
    Start {
        reply_to: oneshot::Sender<Result<bool>>,
    },
}

impl NetworkingHandler {
    pub fn new(options: NetworkingOptions) -> Self {
        Self {
            event_tx: options.event_tx,
            messaging_tx: options.messaging_tx,
            identity_tx: options.identity_tx,
            setting_tx: options.setting_tx,
            status: ServiceStatus::INACTIVE,
            nebula_process: None,
        }
    }
    pub async fn run(&mut self, mut message_rx: mpsc::Receiver<NetworkingMessage>) -> Result<()> {
        info!(func = "run", package = env!("CARGO_PKG_NAME"), "init");
        // Start the service
        let _ = &self.start().await;
        let mut event_rx = self.event_tx.subscribe();
        loop {
            select! {
                msg = message_rx.recv() => {
                    if msg.is_none() {
                        continue;
                    }

                    match msg.unwrap() {
                        NetworkingMessage::Start { reply_to } => {
                            let res = self.start().await;
                            let _ = reply_to.send(res);
                        }
                    };
                }
                // Receive events from other services
                event = event_rx.recv() => {
                    if event.is_err() {
                        continue;
                    }
                    match event.unwrap() {
                        Event::Provisioning(events::ProvisioningEvent::Provisioned) => {
                        },
                        Event::Provisioning(events::ProvisioningEvent::Deprovisioned) => {
                            let _ = &self.stop().await;
                        },
                        Event::Messaging(_) => {},
                        Event::Settings(events::SettingEvent::Synced) => {
                            info!(
                                func = "run",
                                package = env!("CARGO_PKG_NAME"),
                                "settings sync event received"
                            );
                            let _ = &self.start().await;
                        },
                        Event::Settings(events::SettingEvent::Updated { settings }) => {
                            info!(
                                func = "run",
                                package = env!("CARGO_PKG_NAME"),
                                "settings updated event received"
                            );
                            match settings.get("networking.enabled") {
                                Some(value) => {
                                    if value == "true" {
                                        let _ = &self.stop().await;
                                        let _ = &self.start().await;
                                    } else if value == "false" {
                                        let _ = &self.stop().await;
                                    } else {
                                        // Can be add other function to perform
                                    }
                                },
                                None => (),
                            }
                        },
                        Event::Nats(_) => {},

                    }
                }
            }
        }
    }

    async fn cleanup(&mut self) -> Result<bool> {
        info!(
            func = "cleanup",
            package = env!("CARGO_PKG_NAME"),
            "performing cleanup before stopping networking",
        );

        match self.nebula_process.as_mut() {
            Some(nebula_process) => {
                match nebula_process.kill().await {
                    Ok(_) => {
                        info!(
                            func = "cleanup",
                            package = env!("CARGO_PKG_NAME"),
                            "nebula process killed",
                        );
                    }
                    Err(e) => {
                        error!(
                            func = "cleanup",
                            package = env!("CARGO_PKG_NAME"),
                            "error while stopping nebula process {}",
                            e
                        );
                        bail!(NetworkingError::new(
                            NetworkingErrorCodes::CleanupNebulaProcessError,
                            format!("error while stopping nebula process {}", e),
                            false
                        ))
                    }
                };
            }
            None => {
                info!(
                    func = "cleanup",
                    package = env!("CARGO_PKG_NAME"),
                    "nebula process was not running",
                );
            }
        };
        info!(
            func = "cleanup",
            package = env!("CARGO_PKG_NAME"),
            "cleanup done",
        );
        Ok(true)
    }
}

#[async_trait]
impl ServiceHandler for NetworkingHandler {
    async fn start(&mut self) -> Result<bool> {
        let start_response_res = start(
            self.setting_tx.clone(),
            self.identity_tx.clone(),
            self.messaging_tx.clone(),
        )
        .await;

        match start_response_res {
            Ok(start_response) => {
                self.nebula_process = Some(start_response.nebula_process);
                self.status = ServiceStatus::STARTED;
                Ok(true)
            }
            Err(e) => Err(e),
        }
    }

    async fn stop(&mut self) -> Result<bool> {
        let _ = self.cleanup().await;
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

use anyhow::Result;
use events::Event;
use identity::handler::IdentityMessage;
use messaging::handler::MessagingMessage;
use services::{ServiceHandler, ServiceStatus};
use settings::handler::SettingMessage;
use tokio::{
    select,
    sync::{broadcast, mpsc, oneshot},
};
use tonic::async_trait;
use tracing::info;

use crate::service::start;

pub struct NetworkingHandler {
    event_tx: broadcast::Sender<Event>,
    messaging_tx: mpsc::Sender<MessagingMessage>,
    identity_tx: mpsc::Sender<IdentityMessage>,
    setting_tx: mpsc::Sender<SettingMessage>,
    status: ServiceStatus,
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
        }
    }
    pub async fn run(&mut self, mut message_rx: mpsc::Receiver<NetworkingMessage>) -> Result<()> {
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
                        let start_result = start(self.setting_tx.clone(),
                            self.identity_tx.clone(),
                            self.messaging_tx.clone()
                        ).await;
                            let _ = reply_to.send(start_result);
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
                            let _ = &self.start().await;
                        },
                        Event::Provisioning(events::ProvisioningEvent::Deprovisioned) => {
                            let _ = &self.stop().await;
                        },
                        Event::Messaging(_) => {},
                        Event::Settings(events::SettingEvent::Synced) => {
                            info!("networking handler: Settings synced");
                            let _ = start(self.setting_tx.clone(),
                                self.identity_tx.clone(),
                                self.messaging_tx.clone()
                            ).await;
                        },
                        Event::Settings(events::SettingEvent::Updated { settings }) => {
                            info!("networking handler: Settings updated");
                        },
                    }
                }
            }
        }
    }
}

#[async_trait]
impl ServiceHandler for NetworkingHandler {
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

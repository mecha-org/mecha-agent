use anyhow::Result;
use events::Event;
use identity::handler::IdentityMessage;
use messaging::handler::MessagingMessage;
use services::{ServiceHandler, ServiceStatus};
use std::collections::HashMap;
use tokio::select;
use tokio::sync::{broadcast, mpsc, oneshot};
use tonic::async_trait;
use tracing::info;

use crate::services::{get_settings_by_key, set_settings, start_consumer, sync_settings};

pub struct SettingHandler {
    event_tx: broadcast::Sender<Event>,
    messaging_tx: mpsc::Sender<MessagingMessage>,
    identity_tx: mpsc::Sender<IdentityMessage>,
    status: ServiceStatus,
}

pub enum SettingMessage {
    StartConsumer {
        reply_to: oneshot::Sender<Result<bool>>,
    },
    SyncSettings {
        reply_to: oneshot::Sender<Result<bool>>,
    },
    GetSettingsByKey {
        reply_to: oneshot::Sender<Result<String>>,
        key: String,
    },
    SetSettings {
        reply_to: oneshot::Sender<Result<bool>>,
        settings: HashMap<String, String>,
    },
}

pub struct SettingOptions {
    pub event_tx: broadcast::Sender<Event>,
    pub messaging_tx: mpsc::Sender<MessagingMessage>,
    pub identity_tx: mpsc::Sender<IdentityMessage>,
}

impl SettingHandler {
    pub fn new(options: SettingOptions) -> Self {
        Self {
            event_tx: options.event_tx,
            messaging_tx: options.messaging_tx,
            identity_tx: options.identity_tx,
            status: ServiceStatus::INACTIVE,
        }
    }

    pub async fn run(&mut self, mut message_rx: mpsc::Receiver<SettingMessage>) -> Result<()> {
        info!(func = "run", package = env!("CARGO_PKG_NAME"), "init");
        // start the service
        let _ = &self.start().await;
        let mut event_rx = self.event_tx.subscribe();
        loop {
            select! {
                msg = message_rx.recv() => {
                    if msg.is_none() {
                        continue;
                    }

                    match msg.unwrap() {
                        SettingMessage::StartConsumer { reply_to } => {
                            let status = start_consumer(self.messaging_tx.clone(), self.identity_tx.clone()).await;
                            reply_to.send(status);
                        }
                        SettingMessage::SyncSettings { reply_to } => {
                            let status = sync_settings(self.event_tx.clone(), self.messaging_tx.clone(), self.identity_tx.clone()).await;
                            reply_to.send(status);
                        }
                        SettingMessage::GetSettingsByKey { reply_to, key } => {
                            let value = get_settings_by_key(key).await;
                            let _ = reply_to.send(value);
                        }
                        SettingMessage::SetSettings { reply_to, settings } => {
                            let result = set_settings(self.event_tx.clone(), settings).await;
                            let _ = reply_to.send(Ok(false));
                        }
                    };
                }

                // Receive events from other services
                event = event_rx.recv() => {
                    if event.is_err() {
                        continue;
                    }
                    match event.unwrap() {
                        Event::Messaging(events::MessagingEvent::Connected) => {
                            info!(
                                func = "run",
                                package = env!("CARGO_PKG_NAME"),
                                "event received messaging service connected"
                            );
                            let _ = sync_settings(self.event_tx.clone(), self.messaging_tx.clone(), self.identity_tx.clone()).await;
                            let _ = start_consumer(self.messaging_tx.clone(), self.identity_tx.clone()).await;
                        }
                        Event::Provisioning(_) => {},
                        Event::Settings(_) => {},
                        Event::Nats(_) => {},
                    }
                }
            }
        }
    }
}

#[async_trait]
impl ServiceHandler for SettingHandler {
    async fn start(&mut self) -> Result<bool> {
        println!("start setting service");
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

fn generate_code() -> Result<String> {
    Ok("123456".to_string())
}

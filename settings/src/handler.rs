use anyhow::{bail, Result};
use events::Event;
use identity::handler::IdentityMessage;
use messaging::handler::MessagingMessage;
use services::{ServiceHandler, ServiceStatus};
use std::collections::HashMap;
use tokio::select;
use tokio::sync::{broadcast, mpsc, oneshot};
use tonic::async_trait;
use tracing::{error, info};

use crate::errors::{DeviceSettingError, DeviceSettingErrorCodes};
use crate::services::{
    create_pull_consumer, get_settings_by_key, set_settings, start_settings, sync_settings,
};

pub struct SettingHandler {
    event_tx: broadcast::Sender<Event>,
    messaging_tx: mpsc::Sender<MessagingMessage>,
    identity_tx: mpsc::Sender<IdentityMessage>,
    status: ServiceStatus,
}

pub enum SettingMessage {
    StartSettings {
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
    async fn sync_settings(&mut self) -> Result<bool> {
        let (event_tx, messaging_tx) = (self.event_tx.clone(), self.messaging_tx.clone());
        let consumer =
            match create_pull_consumer(self.messaging_tx.clone(), self.identity_tx.clone()).await {
                Ok(s) => s,
                Err(e) => {
                    error!(
                        func = "sync_settings",
                        package = env!("CARGO_PKG_NAME"),
                        "error creating pull consumer, error -  {:?}",
                        e
                    );
                    bail!(DeviceSettingError::new(
                        DeviceSettingErrorCodes::CreateConsumerError,
                        format!("create consumer error - {:?} ", e.to_string()),
                        true
                    ))
                }
            };
        let _ = tokio::task::spawn(async move {
            match sync_settings(consumer, event_tx, messaging_tx).await {
                Ok(_) => {}
                Err(e) => {
                    error!(
                        func = "sync_settings",
                        package = env!("CARGO_PKG_NAME"),
                        "error syncing settings, error -  {:?}",
                        e
                    );
                }
            }
        });
        Ok(true)
    }

    async fn start_settings(&mut self) -> Result<bool> {
        let messaging_tx = self.messaging_tx.clone();
        let consumer =
            match create_pull_consumer(self.messaging_tx.clone(), self.identity_tx.clone()).await {
                Ok(s) => s,
                Err(e) => {
                    error!(
                        func = "sync_settings",
                        package = env!("CARGO_PKG_NAME"),
                        "error creating pull consumer, error -  {:?}",
                        e
                    );
                    bail!(DeviceSettingError::new(
                        DeviceSettingErrorCodes::CreateConsumerError,
                        format!("create consumer error - {:?} ", e.to_string()),
                        true
                    ))
                }
            };
        let _ = tokio::task::spawn(async move {
            match start_settings(consumer, messaging_tx).await {
                Ok(_) => {}
                Err(e) => {
                    error!(
                        func = "sync_settings",
                        package = env!("CARGO_PKG_NAME"),
                        "error syncing settings, error -  {:?}",
                        e
                    );
                }
            }
        });
        Ok(true)
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
                        SettingMessage::StartSettings { reply_to } => {
                            let status = self.start_settings().await;
                            reply_to.send(status);
                        }
                        SettingMessage::SyncSettings { reply_to } => {
                            let status = self.sync_settings().await;
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
                            let _ = self.sync_settings().await;
                            let _ = self.start_settings().await;
                        }
                        _ => {}

                    }
                }
            }
        }
    }
}

#[async_trait]
impl ServiceHandler for SettingHandler {
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

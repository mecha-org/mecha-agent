use anyhow::{bail, Result};
use events::Event;
use identity::handler::IdentityMessage;
use messaging::handler::MessagingMessage;
use std::collections::HashMap;
use tokio::select;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::task::{JoinHandle, JoinSet};
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

use crate::errors::{DeviceSettingError, DeviceSettingErrorCodes};
use crate::service::{
    await_settings_message, create_pull_consumer, get_settings_by_key, set_settings,
};
const PACKAGE_NAME: &str = env!("CARGO_CRATE_NAME");
pub struct SettingHandler {
    event_tx: broadcast::Sender<Event>,
    messaging_tx: mpsc::Sender<MessagingMessage>,
    identity_tx: mpsc::Sender<IdentityMessage>,
    setting_consumer_token: Option<CancellationToken>,
    sync_settings_token: Option<CancellationToken>,
}

pub enum SettingMessage {
    StartSettings {
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
            setting_consumer_token: None,
            sync_settings_token: None,
        }
    }

    async fn settings_consumer(&mut self) -> Result<bool> {
        let fn_name = "settings_consumer";
        // safety: check for existing cancel token, and cancel it
        let exist_settings_token = &self.setting_consumer_token;
        if exist_settings_token.is_some() {
            let _ = exist_settings_token.as_ref().unwrap().cancel();
        }
        // create a new token
        let settings_token = CancellationToken::new();
        let settings_token_cloned = settings_token.clone();
        let messaging_tx = self.messaging_tx.clone();
        let consumer =
            match create_pull_consumer(self.messaging_tx.clone(), self.identity_tx.clone()).await {
                Ok(s) => s,
                Err(e) => {
                    error!(
                        func = fn_name,
                        package = env!("CARGO_PKG_NAME"),
                        "error creating pull consumer, error -  {:?}",
                        e
                    );
                    bail!(DeviceSettingError::new(
                        DeviceSettingErrorCodes::CreateConsumerError,
                        format!("create consumer error - {:?} ", e.to_string()),
                    ))
                }
            };
        println!("******************* consumer**************: {:?}", consumer);
        let mut futures = JoinSet::new();
        futures.spawn(await_settings_message(
            consumer.clone(),
            messaging_tx.clone(),
            self.event_tx.clone(),
        ));
        // create spawn for timer
        let _: JoinHandle<Result<()>> = tokio::task::spawn(async move {
            loop {
                select! {
                        _ = settings_token.cancelled() => {
                            info!(
                                func = fn_name,
                                package = PACKAGE_NAME,
                                result = "success",
                                "settings subscriber cancelled"
                            );
                            return Ok(());
                    },
                    result = futures.join_next() => {
                        if result.is_none() {
                            continue;
                        }
                    },
                }
            }
        });

        // Save to state
        self.setting_consumer_token = Some(settings_token_cloned);
        Ok(true)
    }
    fn clear_settings_subscription(&self) -> Result<bool> {
        let exist_subscriber_token = &self.setting_consumer_token;
        if exist_subscriber_token.is_some() {
            let _ = exist_subscriber_token.as_ref().unwrap().cancel();
        } else {
            return Ok(false);
        }
        Ok(true)
    }
    fn clear_sync_settings_subscriber(&self) -> Result<bool> {
        let exist_subscriber_token = &self.sync_settings_token;
        if exist_subscriber_token.is_some() {
            let _ = exist_subscriber_token.as_ref().unwrap().cancel();
        } else {
            return Ok(false);
        }
        Ok(true)
    }
    pub async fn run(&mut self, mut message_rx: mpsc::Receiver<SettingMessage>) -> Result<()> {
        let fn_name = "run";
        info!(
            func = fn_name,
            package = PACKAGE_NAME,
            "settings service initiated"
        );
        let mut event_rx = self.event_tx.subscribe();
        loop {
            select! {
                msg = message_rx.recv() => {
                    if msg.is_none() {
                        continue;
                    }

                    match msg.unwrap() {
                        SettingMessage::StartSettings { reply_to } => {
                            let status = self.settings_consumer().await;
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
                            info!{
                                func = fn_name,
                                package = PACKAGE_NAME,
                                "connected event in settings service"
                            }
                            match self.settings_consumer().await {
                                Ok(_res) => {},
                                Err(e) => {
                                    error!(
                                        func = fn_name,
                                        package = PACKAGE_NAME,
                                        "error starting settings consumer: {}", e
                                    );
                                }
                            }

                        }
                        Event::Messaging(events::MessagingEvent::Disconnected) => {
                            info!(
                                func = fn_name,
                                package = PACKAGE_NAME,
                                "disconnected event in settings service"
                            );
                            let _ = self.clear_sync_settings_subscriber();
                            let _ = self.clear_settings_subscription();
                        }
                        Event::Provisioning(events::ProvisioningEvent::Deprovisioned) => {
                            info!(
                                func = fn_name,
                                package = PACKAGE_NAME,
                                "deprovisioned event in settings service"
                            );
                            let _ = self.clear_sync_settings_subscriber();
                            let _ = self.clear_settings_subscription();
                        }
                        _ => {}

                    }
                }
            }
        }
    }
}

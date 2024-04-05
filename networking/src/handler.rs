use anyhow::{bail, Result};
use events::Event;
use identity::handler::IdentityMessage;
use messaging::handler::MessagingMessage;
use settings::handler::SettingMessage;
use tokio::select;
use tokio::sync::{broadcast, mpsc};
use tokio::task::{JoinHandle, JoinSet};
use tokio_util::sync::CancellationToken;
use tracing::{error, info};
use wireguard::Wireguard;

use crate::errors::{NetworkingError, NetworkingErrorCodes};
use crate::service::{await_consumer_message, configure_wireguard, create_pull_consumer};

const PACKAGE_NAME: &str = env!("CARGO_CRATE_NAME");
pub struct NetworkingHandler {
    identity_tx: mpsc::Sender<IdentityMessage>,
    messaging_tx: mpsc::Sender<MessagingMessage>,
    event_tx: broadcast::Sender<Event>,
    subscriber_token: Option<CancellationToken>,
    networking_consumer_token: Option<CancellationToken>,
    wireguard: Option<Wireguard>,
}

pub enum NetworkingMessage {}

pub struct NetworkingOptions {
    pub messaging_tx: mpsc::Sender<MessagingMessage>,
    pub identity_tx: mpsc::Sender<IdentityMessage>,
    pub event_tx: broadcast::Sender<Event>,
    pub setting_tx: mpsc::Sender<SettingMessage>,
}

impl NetworkingHandler {
    pub fn new(options: NetworkingOptions) -> Self {
        Self {
            identity_tx: options.identity_tx,
            messaging_tx: options.messaging_tx,
            event_tx: options.event_tx,
            subscriber_token: None,
            networking_consumer_token: None,
            wireguard: None,
        }
    }
    async fn networking_consumer(&mut self) -> Result<bool> {
        let fn_name = "networking_consumer";
        // safety: check for existing cancel token, and cancel it
        let exist_consumer_token = &self.networking_consumer_token;
        if exist_consumer_token.is_some() {
            let _ = exist_consumer_token.as_ref().unwrap().cancel();
        }
        // create a new token
        let consumer_token = CancellationToken::new();
        let consumer_token_cloned = consumer_token.clone();
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
                    bail!(NetworkingError::new(
                        NetworkingErrorCodes::CreateConsumerError,
                        format!("create consumer error - {:?} ", e.to_string()),
                        true
                    ))
                }
            };
        let mut futures = JoinSet::new();
        futures.spawn(await_consumer_message(
            consumer.clone(),
            messaging_tx.clone(),
            self.wireguard.as_ref().unwrap().clone(),
        ));
        // create spawn for timer
        let _: JoinHandle<Result<()>> = tokio::task::spawn(async move {
            loop {
                select! {
                        _ = consumer_token.cancelled() => {
                            info!(
                                func = fn_name,
                                package = PACKAGE_NAME,
                                result = "success",
                                "consumer subscriber cancelled"
                            );
                            return Ok(());
                    },
                    result = futures.join_next() => {
                        if result.unwrap().is_ok() {}
                    },
                }
            }
        });

        // Save to state
        self.networking_consumer_token = Some(consumer_token_cloned);
        Ok(true)
    }
    pub async fn run(&mut self, mut message_rx: mpsc::Receiver<NetworkingMessage>) -> Result<()> {
        info!(func = "run", package = PACKAGE_NAME, "init");
        let mut event_rx = self.event_tx.subscribe();
        loop {
            select! {
                msg = message_rx.recv() => {
                    if msg.is_none() {
                        continue;
                    }

                    match msg.unwrap() {
                        _ => {}
                    };
                },
                // Receive events from other services
                event = event_rx.recv() => {
                    if event.is_err() {
                        continue;
                    }
                    match event.unwrap() {
                        Event::Messaging(events::MessagingEvent::Connected) => {
                            info!(
                                func = "run",
                                package = PACKAGE_NAME,
                                "connected event in networking"
                            );
                            match configure_wireguard(self.messaging_tx.clone(), self.identity_tx.clone()).await {
                                Ok(wireguard) => {
                                    info!(
                                        func = "run",
                                        package = PACKAGE_NAME,
                                        "configure wireguard success"
                                    );
                                    self.wireguard = Some(wireguard);
                                }
                                Err(e) => {
                                    error!(
                                        func = "run",
                                        package = PACKAGE_NAME,
                                        "configure wireguard error - {:?}",
                                        e
                                    );
                                }
                            }
                            let _ = self.networking_consumer().await;
                        },
                        Event::Messaging(events::MessagingEvent::Disconnected) => {
                            info!(
                                func = "run",
                                package = PACKAGE_NAME,
                                "disconnected event in networking"
                            );
                        },
                        Event::Provisioning(events::ProvisioningEvent::Deprovisioned) => {
                            info!(
                                func = "run",
                                package = PACKAGE_NAME,
                                "deprovisioned event in networking"
                            );
                        },
                        _ => {},
                    }
                }

            }
        }
    }
}

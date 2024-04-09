use agent_settings::{read_settings_yml, AgentSettings};
use anyhow::{bail, Result};
use events::Event;
use identity::handler::IdentityMessage;
use messaging::handler::MessagingMessage;
use settings::handler::SettingMessage;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::task::{JoinHandle, JoinSet};
use tokio::{select, task};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use wireguard::Wireguard;

use crate::errors::{NetworkingError, NetworkingErrorCodes};
use crate::handshake_handler::{
    await_networking_handshake_request, HandshakeChannelHandler, HandshakeMessage, Manifest,
};
use crate::service::{
    await_consumer_message, configure_wireguard, create_pull_consumer, subscribe_to_nats,
};

const PACKAGE_NAME: &str = env!("CARGO_CRATE_NAME");
pub struct NetworkingHandler {
    identity_tx: mpsc::Sender<IdentityMessage>,
    messaging_tx: mpsc::Sender<MessagingMessage>,
    event_tx: broadcast::Sender<Event>,
    subscriber_token: Option<CancellationToken>,
    networking_consumer_token: Option<CancellationToken>,
    wireguard: Option<Wireguard>,
    handshake_handler: Option<HandshakeChannelHandler>,
}

pub enum NetworkingMessage {
    Request {
        machine_id: String,
        reply_to: oneshot::Sender<Result<bool>>,
    },
    HandshakeManifest {
        manifest: Manifest,
    },
}

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
            handshake_handler: None,
        }
    }
    pub async fn subscribe_to_nats(&mut self) -> Result<()> {
        info!(
            func = "subscribe_to_nats",
            package = env!("CARGO_PKG_NAME"),
            "init"
        );
        let settings = match read_settings_yml() {
            Ok(s) => s,
            Err(e) => {
                warn!(
                    func = "subscribe_to_nats",
                    package = PACKAGE_NAME,
                    "settings.yml not found, using default settings"
                );
                AgentSettings::default()
            }
        };

        let (handshake_t, handshake_tx, channel_id) =
            init_handshake_handler(self.messaging_tx.clone()).await;
        // safety: check for existing cancel token, and cancel it
        let exist_subscriber_token = &self.subscriber_token;
        if exist_subscriber_token.is_some() {
            let _ = exist_subscriber_token.as_ref().unwrap().cancel();
        }

        // create a new token
        let subscriber_token = CancellationToken::new();
        let subscriber_token_cloned = subscriber_token.clone();
        let messaging_tx = self.messaging_tx.clone();
        let identity_tx = self.identity_tx.clone();
        let event_tx = self.event_tx.clone();
        let mut timer = tokio::time::interval(std::time::Duration::from_secs(50));
        let subscribers = match subscribe_to_nats(
            identity_tx.clone(),
            messaging_tx.clone(),
            channel_id,
            settings.networking.peer_settings.network_id,
        )
        .await
        {
            Ok(v) => v,
            Err(e) => {
                error!(
                    func = "subscribe_to_nats",
                    package = PACKAGE_NAME,
                    "subscribe to nats error - {:?}",
                    e
                );
                bail!(e)
            }
        };
        let mut futures = JoinSet::new();
        futures.spawn(await_networking_handshake_request(
            subscribers.handshake_request.unwrap(),
            handshake_tx.clone(),
        ));
        // create spawn for timer
        let _: JoinHandle<Result<()>> = tokio::task::spawn(async move {
            loop {
                select! {
                        _ = subscriber_token.cancelled() => {
                            info!(
                                func = "subscribe_to_nats",
                                package = PACKAGE_NAME,
                                result = "success",
                                "subscribe to nats cancelled"
                            );
                            return Ok(());
                    },
                    result = futures.join_next() => {
                        if result.unwrap().is_ok() {}
                    },
                    _ = timer.tick() => {
                        info!(
                            func = "subscribe_to_nats",
                            package = PACKAGE_NAME,
                            result = "success",
                            "subscribe to nats timer tick"
                        );
                    }
                }
            }
            // return Ok(());
        });
        // Save to state
        self.subscriber_token = Some(subscriber_token_cloned);
        Ok(())
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
                            let _ = self.subscribe_to_nats().await;
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

async fn init_handshake_handler(
    messaging_tx: mpsc::Sender<MessagingMessage>,
) -> (
    task::JoinHandle<Result<()>>,
    mpsc::Sender<HandshakeMessage>,
    String,
) {
    let (handshake_tx, handshake_rx) = mpsc::channel(32);
    let (mut handler, channel_id) =
        HandshakeChannelHandler::new("networking".to_string(), messaging_tx.clone());
    let handshake_t = tokio::spawn(async move {
        match handler.run(handshake_rx).await {
            Ok(_) => (),
            Err(e) => {
                error!(
                    func = "init_status_service",
                    package = PACKAGE_NAME,
                    "error init/run status service: {:?}",
                    e
                );
                bail!(NetworkingError::new(
                    NetworkingErrorCodes::NetworkingInitError,
                    format!("error init/run status service: {:?}", e),
                    true
                ));
            }
        }
        Ok(())
    });

    (handshake_t, handshake_tx, channel_id)
}

use crate::service::{
    await_deprovision_message, await_re_issue_cert_message, de_provision, generate_code, ping,
    provision_by_code, subscribe_to_nats, PingResponse,
};
use anyhow::{bail, Result};
use events::Event;
use identity::handler::IdentityMessage;
use messaging::handler::MessagingMessage;
use tokio::select;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::task::{JoinHandle, JoinSet};
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

const PACKAGE_NAME: &str = env!("CARGO_CRATE_NAME");

pub struct ProvisioningHandler {
    identity_tx: mpsc::Sender<IdentityMessage>,
    messaging_tx: mpsc::Sender<MessagingMessage>,
    event_tx: broadcast::Sender<Event>,
    subscriber_token: Option<CancellationToken>,
}

pub enum ProvisioningMessage {
    Ping {
        reply_to: oneshot::Sender<Result<PingResponse>>,
    },
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
    pub messaging_tx: mpsc::Sender<MessagingMessage>,
    pub identity_tx: mpsc::Sender<IdentityMessage>,
    pub event_tx: broadcast::Sender<Event>,
}

impl ProvisioningHandler {
    pub fn new(options: ProvisioningOptions) -> Self {
        Self {
            identity_tx: options.identity_tx,
            messaging_tx: options.messaging_tx,
            event_tx: options.event_tx,
            subscriber_token: None,
        }
    }

    pub async fn subscribe_to_nats(&mut self) -> Result<()> {
        info!(
            func = "subscribe_to_nats",
            package = env!("CARGO_PKG_NAME"),
            "init"
        );

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
        let subscribers = match subscribe_to_nats(identity_tx.clone(), messaging_tx.clone()).await {
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
        futures.spawn(await_deprovision_message(
            identity_tx.clone(),
            event_tx.clone(),
            subscribers.de_provisioning_request.unwrap(),
        ));
        futures.spawn(await_re_issue_cert_message(
            identity_tx.clone(),
            event_tx.clone(),
            subscribers.re_issue_certificate.unwrap(),
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
    pub fn clear_nats_subscription(&self) -> Result<bool> {
        let exist_subscriber_token = &self.subscriber_token;
        if exist_subscriber_token.is_some() {
            let _ = exist_subscriber_token.as_ref().unwrap().cancel();
        } else {
            return Ok(false);
        }
        Ok(true)
    }
    pub async fn run(&mut self, mut message_rx: mpsc::Receiver<ProvisioningMessage>) -> Result<()> {
        info!(func = "run", package = env!("CARGO_PKG_NAME"), "init");
        let mut event_rx = self.event_tx.subscribe();
        loop {
            select! {
                msg = message_rx.recv() => {
                    if msg.is_none() {
                        continue;
                    }

                    match msg.unwrap() {
                        ProvisioningMessage::Ping { reply_to } => {
                            let response = ping().await;
                            let _ = reply_to.send(response);
                        }
                        ProvisioningMessage::GenerateCode { reply_to } => {
                            let code = generate_code();
                            let _ = reply_to.send(code);
                        }
                        ProvisioningMessage::ProvisionByCode { code, reply_to } => {
                            let status = provision_by_code(code, self.event_tx.clone()).await;
                            let _ = reply_to.send(status);
                        }
                        ProvisioningMessage::ProvisionByManifest { manifest, reply_to } => {
                            let _ = reply_to.send(Some(true));
                        }
                        ProvisioningMessage::Deprovision { reply_to } => {
                            let status = de_provision(self.event_tx.clone());
                            let _ = reply_to.send(status);
                        }
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
                                package = env!("CARGO_PKG_NAME"),
                                "connected event in provisioning"
                            );
                            // start
                            match &self.subscribe_to_nats().await {
                                Ok(_) => {},
                                Err(e) => {
                                    error!(
                                        func = "run",
                                        package = env!("CARGO_PKG_NAME"),
                                        "subscribe to nats error - {:?}",
                                        e
                                    );
                                }
                            }
                        },
                        Event::Messaging(events::MessagingEvent::Disconnected) => {
                            info!(
                                func = "run",
                                package = env!("CARGO_PKG_NAME"),
                                "disconnected event in provisioning"
                            );
                            let _ = &self.clear_nats_subscription();
                        },
                        Event::Provisioning(events::ProvisioningEvent::Deprovisioned) => {
                            info!(
                                func = "run",
                                package = env!("CARGO_PKG_NAME"),
                                "deprovisioned event in provisioning"
                            );
                            let _ = &self.clear_nats_subscription();
                        },
                        _ => {},
                    }
                }

            }
        }
    }
}

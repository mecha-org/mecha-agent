use std::ops::Deref;
use std::sync::Arc;

use crate::errors::{ProvisioningError, ProvisioningErrorCodes};
use crate::service::{
    de_provision, generate_code, ping, provision_by_code, subscribe_to_nats, PingResponse,
    ProvisioningSubscriber,
};
use anyhow::{bail, Result};
use async_trait::async_trait;
use events::Event;
use futures::StreamExt;
use identity::handler::IdentityMessage;
use messaging::handler::MessagingMessage;
use messaging::Subscriber as NatsSubscriber;
use messaging::{async_nats, Message as NatsMessage};
use services::{ServiceHandler, ServiceStatus};
use tokio::select;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::task::JoinSet;
use tracing::{error, info};

const PACKAGE_NAME: &str = env!("CARGO_CRATE_NAME");

pub struct ProvisioningHandler {
    identity_tx: mpsc::Sender<IdentityMessage>,
    message_tx: mpsc::Sender<MessagingMessage>,
    event_tx: broadcast::Sender<Event>,
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
            message_tx: options.messaging_tx,
            event_tx: options.event_tx,
        }
    }

    pub async fn run(&mut self, mut message_rx: mpsc::Receiver<ProvisioningMessage>) -> Result<()> {
        info!(func = "run", package = env!("CARGO_PKG_NAME"), "init");
        let mut timer = tokio::time::interval(std::time::Duration::from_secs(5));

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
                _ = timer.tick() => {
                    info!(func = "run", package = env!("CARGO_PKG_NAME"), "service is running!");
                }
            }
        }
    }
}

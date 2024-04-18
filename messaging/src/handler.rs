use crate::errors::{MessagingError, MessagingErrorCodes};
use crate::service::{get_machine_id, Messaging};
use anyhow::{bail, Result};
use events::Event;
use identity::handler::IdentityMessage;
use nats_client::{jetstream::JetStreamClient, Bytes, Subscriber};
use services::{ServiceHandler, ServiceStatus};
use tokio::{
    select,
    sync::{broadcast, mpsc, oneshot},
};
use tonic::async_trait;
use tracing::{error, info};

const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");

pub enum MessagingMessage {
    Connect {
        reply_to: oneshot::Sender<Result<bool>>,
    },
    Disconnect {
        reply_to: oneshot::Sender<Result<bool>>,
    },
    Reconnect {
        reply_to: oneshot::Sender<Result<bool>>,
    },
    Subscriber {
        reply_to: oneshot::Sender<Result<Subscriber>>,
        subject: String,
    },
    Send {
        reply_to: oneshot::Sender<Result<bool>>,
        message: String,
        subject: String,
    },
    Request {
        reply_to: oneshot::Sender<Result<Bytes>>,
        message: String,
        subject: String,
    },
    InitJetStream {
        reply_to: oneshot::Sender<Result<JetStreamClient>>,
    },
}
pub struct MessagingOptions {
    pub event_tx: broadcast::Sender<Event>,
    pub identity_tx: mpsc::Sender<IdentityMessage>,
}

pub struct MessagingHandler {
    event_tx: broadcast::Sender<Event>,
    status: ServiceStatus,
    messaging_client: Messaging,
    identity_tx: mpsc::Sender<IdentityMessage>,
}

impl MessagingHandler {
    pub fn new(options: MessagingOptions) -> Self {
        Self {
            event_tx: options.event_tx,
            status: ServiceStatus::STARTED,
            messaging_client: Messaging::new(true),
            identity_tx: options.identity_tx,
        }
    }
    pub async fn run(&mut self, mut message_rx: mpsc::Receiver<MessagingMessage>) -> Result<()> {
        info!(
            func = "run",
            package = env!("CARGO_PKG_NAME"),
            "messaging service initiated"
        );
        let _ = &self.start().await;
        let mut event_rx = self.event_tx.subscribe();
        loop {
            select! {
                msg = message_rx.recv() => {
                    if msg.is_none() {
                        continue;
                    }
                    match msg.unwrap() {
                        MessagingMessage::Send{reply_to, message, subject} => {
                            let res = self.messaging_client.publish(&subject.as_str(), Bytes::from(message)).await;
                            let _ = reply_to.send(res);
                        }
                        MessagingMessage::Request{reply_to, message, subject} => {
                            let res = self.messaging_client.request(&subject.as_str(), Bytes::from(message)).await;
                            let _ = reply_to.send(res);
                        },
                        MessagingMessage::Connect { reply_to } => {
                            let res = self.messaging_client.connect(&self.identity_tx, self.event_tx.clone()).await;
                            let _ = reply_to.send(res);
                        },
                        MessagingMessage::Reconnect { reply_to } => {
                            let res = self.messaging_client.connect(&self.identity_tx, self.event_tx.clone()).await;
                            let _ = reply_to.send(res);
                        },
                        MessagingMessage::Subscriber { reply_to, subject } => {
                            let res = self.messaging_client.subscribe(subject.as_str()).await;
                            let _ = reply_to.send(res);
                        },
                        MessagingMessage::InitJetStream { reply_to } => {
                            let res = self.messaging_client.init_jetstream().await;
                            let _ = reply_to.send(res);
                        }
                        _ => {}
                    };
                }
                // Receive events from other services
                event = event_rx.recv() => {
                    if event.is_err() {
                        continue;
                    }
                    match event.unwrap() {
                        Event::Provisioning(events::ProvisioningEvent::Provisioned) => {
                            info!(
                                func = "run",
                                package = env!("CARGO_PKG_NAME"),
                                "messaging service received provisioning event"
                            );
                            let _ = &self.start().await;
                        },
                        Event::Provisioning(events::ProvisioningEvent::Deprovisioned) => {
                            info!(
                                func = "run",
                                package = env!("CARGO_PKG_NAME"),
                                "messaging service received deprovisioning event"
                            );
                            let _ = &self.stop().await;
                        },
                        Event::Nats(nats_client::NatsEvent::Disconnected) => {
                            let _ = match self.event_tx.send(Event::Messaging(events::MessagingEvent::Disconnected)) {
                                Ok(_) => {}
                                Err(e) => {
                                    error!(
                                        func = "run",
                                        package = PACKAGE_NAME,
                                        "error sending messaging service disconnected event - {}",
                                        e
                                    );
                                    bail!(MessagingError::new(
                                        MessagingErrorCodes::EventSendError,
                                        format!("error sending messaging service disconnected - {}", e),
                                        true
                                    ));
                                }
                            };
                            let _ = self.messaging_client.connect(&self.identity_tx, self.event_tx.clone()).await;
                        },
                      _ => {}
                    }
                }
            }
        }
    }
}

#[async_trait]
impl ServiceHandler for MessagingHandler {
    async fn start(&mut self) -> Result<bool> {
        let machine_id = match get_machine_id(self.identity_tx.clone()).await {
            Ok(id) => id,
            Err(e) => {
                return Ok(false);
            }
        };
        if !machine_id.is_empty() {
            self.status = ServiceStatus::STARTED;
            match self
                .messaging_client
                .connect(&self.identity_tx, self.event_tx.clone())
                .await
            {
                Ok(_) => {}
                Err(e) => {}
            };
        }
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

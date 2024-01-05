use anyhow::Result;
use events::Event;
use identity::handler::IdentityMessage;
use nats_client::{jetstream::JetStreamClient, Bytes, Subscriber};
use services::{ServiceHandler, ServiceStatus};
use tokio::{
    select,
    sync::{broadcast, mpsc, oneshot},
};
use tonic::async_trait;
use tracing::info;

use crate::service::{get_machine_id, Messaging};

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
                                    MessagingMessage::Disconnect { reply_to } => todo!(),
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
                    Event::Messaging(_) => {},
                    Event::Settings(_) => {},
                    Event::Nats(nats_client::NatsEvent::Disconnected) => {
                        let _ = self.messaging_client.connect(&self.identity_tx, self.event_tx.clone()).await;
                    },
                    Event::Nats(_) => {
                    },
                }
            }
            }
        }
    }
}

#[async_trait]
impl ServiceHandler for MessagingHandler {
    async fn start(&mut self) -> Result<bool> {
        println!("Starting messaging service");
        let machine_id = match get_machine_id(self.identity_tx.clone()).await {
            Ok(id) => id,
            Err(e) => {
                return Ok(false);
            }
        };
        println!("Should connect: {}", machine_id);
        if !machine_id.is_empty() {
            self.status = ServiceStatus::STARTED;
            match self
                .messaging_client
                .connect(&self.identity_tx, self.event_tx.clone())
                .await
            {
                Ok(_) => {}
                Err(e) => {
                    println!("Error connecting to messaging service: {:?}", e);
                }
            };
        }
        println!("Messaging service started");
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

use anyhow::{bail, Result};
use events::Event;
use identity::handler::IdentityMessage;
use messaging::handler::MessagingMessage;
use serde_json::json;
use services::{ServiceHandler, ServiceStatus};
use tokio::{
    select,
    sync::{
        broadcast,
        mpsc::{self, Sender},
        oneshot,
    },
};
use tonic::async_trait;
use tracing::{error, info, warn};

use crate::service::{
    device_provision_status, get_time_interval, send_heartbeat, SendHeartbeatOptions,
};

pub struct HeartbeatHandler {
    event_tx: broadcast::Sender<Event>,
    messaging_tx: Sender<MessagingMessage>,
    identity_tx: Sender<IdentityMessage>,
    status: ServiceStatus,
}

pub enum HeartbeatMessage {
    Send {
        reply_to: oneshot::Sender<Result<bool>>,
    },
}
pub struct HeartbeatOptions {
    pub event_tx: broadcast::Sender<Event>,
    pub messaging_tx: Sender<MessagingMessage>,
    pub identity_tx: Sender<IdentityMessage>,
}

impl HeartbeatHandler {
    pub fn new(options: HeartbeatOptions) -> Self {
        Self {
            event_tx: options.event_tx,
            messaging_tx: options.messaging_tx,
            identity_tx: options.identity_tx,
            status: ServiceStatus::INACTIVE,
        }
    }
    pub async fn run(&mut self, mut message_rx: mpsc::Receiver<HeartbeatMessage>) -> Result<()> {
        // start the service
        let _ = &self.start().await;
        let interval_in_secs: u64 = get_time_interval();
        let mut event_rx = self.event_tx.subscribe();
        let mut timer = tokio::time::interval(std::time::Duration::from_secs(interval_in_secs));
        loop {
            select! {
                    msg = message_rx.recv() => {
                        if msg.is_none() {
                            continue;
                        }

                        match msg.unwrap() {
                            HeartbeatMessage::Send { reply_to } => {
                                let res = send_heartbeat(SendHeartbeatOptions {
                                    messaging_tx: self.messaging_tx.clone(),
                                    identity_tx: self.identity_tx.clone(),
                                }).await;
                                reply_to.send(res);
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
                                info!("Heartbeat service received provisioning event");
                                let _ = &self.start().await;
                            },
                            Event::Provisioning(events::ProvisioningEvent::Deprovisioned) => {
                                let _ = &self.stop().await;
                            },
                            Event::Messaging(events::MessagingEvent::Connected) => {
                                info!("Heartbeat service received messaging connected event");
                                if !self.is_started().unwrap() {
                                    let _ = &self.start().await;
                                }
                            },
                            Event::Settings(_) => {},
                        }
                    }

                    _ = timer.tick() => {
                        if self.is_started().unwrap() {
                           let _ = send_heartbeat(SendHeartbeatOptions {
                                messaging_tx: self.messaging_tx.clone(),
                                identity_tx: self.identity_tx.clone(),
                            }).await;
                    } else {
                        info!("heartbeat service is not started");
                    }
                }
            }
        }
    }
}

#[async_trait]
impl ServiceHandler for HeartbeatHandler {
    async fn start(&mut self) -> Result<bool> {
        // Start if device is provisioned
        let is_provisioned = device_provision_status(self.identity_tx.clone()).await;
        error!("is provisioned : {}", is_provisioned);
        if is_provisioned {
            self.status = ServiceStatus::STARTED;
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

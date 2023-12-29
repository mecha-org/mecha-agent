use anyhow::Result;
use events::Event;
use identity::handler::IdentityMessage;
use messaging::handler::MessagingMessage;
use services::{ServiceHandler, ServiceStatus};
use tokio::{
    select,
    sync::{broadcast, mpsc, oneshot},
};
use tonic::async_trait;
use tracing::info;

use crate::service::{device_provision_status, process_logs, process_metrics, telemetry_init};

pub struct TelemetryHandler {
    event_tx: broadcast::Sender<Event>,
    pub messaging_tx: mpsc::Sender<MessagingMessage>,
    pub identity_tx: mpsc::Sender<IdentityMessage>,
    status: ServiceStatus,
}
pub struct TelemetryOptions {
    pub event_tx: broadcast::Sender<Event>,
    pub messaging_tx: mpsc::Sender<MessagingMessage>,
    pub identity_tx: mpsc::Sender<IdentityMessage>,
}

pub enum TelemetryMessage {
    SendLogs {
        logs: Vec<u8>,
        logs_type: String,
        reply_to: oneshot::Sender<Result<bool>>,
    },
    SendMetrics {
        metrics: Vec<u8>,
        metrics_type: String,
        reply_to: oneshot::Sender<Result<bool>>,
    },
}

impl TelemetryHandler {
    pub fn new(options: TelemetryOptions) -> Self {
        Self {
            event_tx: options.event_tx,
            identity_tx: options.identity_tx,
            messaging_tx: options.messaging_tx,
            status: ServiceStatus::INACTIVE,
        }
    }
    pub async fn run(&mut self, mut message_rx: mpsc::Receiver<TelemetryMessage>) -> Result<()> {
        // Start the service
        let mut event_rx = self.event_tx.subscribe();
        loop {
            select! {
                msg = message_rx.recv() => {
                    if msg.is_none() {
                        continue;
                    }

                    match msg.unwrap() {
                        TelemetryMessage::SendLogs {logs, logs_type, reply_to } => {
                            match self.is_started() {
                                Ok(started) => match started {
                                    true => {
                                        let result = process_logs(logs_type, logs, self.identity_tx.clone(), self.messaging_tx.clone() ).await;
                                        let _ = reply_to.send(result);
                                    },
                                    false => {
                                        let _ = reply_to.send(Ok(false));
                                        continue;
                                    }
                                },
                                Err(_) => {
                                    let _ = reply_to.send(Ok(false));
                                    continue;
                                }
                            }
                        }
                        TelemetryMessage::SendMetrics {metrics, metrics_type, reply_to } => {
                            let result = process_metrics(metrics, metrics_type, self.identity_tx.clone(), self.messaging_tx.clone() ).await;
                            let _ = reply_to.send(result);
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
                            "device provisioned event received"
                        );
                        let _ = &self.start().await;
                    },
                    Event::Provisioning(events::ProvisioningEvent::Deprovisioned) => {
                        let _ = &self.stop().await;
                    },
                    Event::Messaging(_) => {},
                    Event::Settings(events::SettingEvent::Synced) => {
                        let _ = &self.start().await;
                    },
                    Event::Settings(events::SettingEvent::Updated { settings }) => {
                        info!(
                            func = "run",
                            package = env!("CARGO_PKG_NAME"),
                            "settings updated event received"
                        );
                        match settings.get("telemetry.enabled") {
                            Some(value) => {
                                if value == "true" {
                                    let _ = &self.start().await;
                                } else if value == "false" {
                                    let _ = &self.stop().await;
                                } else {
                                    // Can be add other function to perform
                                }
                            },
                            None => {},
                        }
                    },
                }
            }
            }
        }
    }
}

#[async_trait]
impl ServiceHandler for TelemetryHandler {
    async fn start(&mut self) -> Result<bool> {
        // match device_provision_status(self.identity_tx.clone()).await {
        //     Ok(provisioned) => {
        //         if provisioned {
        //             self.status = ServiceStatus::STARTED;
        //             let _ = telemetry_init();
        //             Ok(true)
        //         } else {
        //             Ok(false)
        //         }
        //     }
        //     Err(err) => Ok(false),
        // }
        let _ = telemetry_init();
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

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

use crate::service::{process_logs, telemetry_init};

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
    pub async fn run(&mut self, mut message_rx: mpsc::Receiver<TelemetryMessage>) {
        // Start the service
        let _ = &self.start().await;
        loop {
            select! {
                msg = message_rx.recv() => {
                    if msg.is_none() {
                        continue;
                    }

                    match msg.unwrap() {
                        TelemetryMessage::SendLogs {logs, logs_type, reply_to } => {
                            process_logs(logs_type, logs, self.identity_tx.clone(), self.messaging_tx.clone() ).await;
                            let _ = reply_to.send(Ok(true));
                        }
                    };
                }
            }
        }
    }
}

#[async_trait]
impl ServiceHandler for TelemetryHandler {
    async fn start(&mut self) -> Result<bool> {
        self.status = ServiceStatus::STARTED;
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

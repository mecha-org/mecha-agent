use crate::service::{get_machine_cert, get_machine_id, get_provision_status};
use anyhow::Result;
use crypto::MachineCert;
use events::Event;
use services::{ServiceHandler, ServiceStatus};
use tokio::{
    select,
    sync::{broadcast, mpsc, oneshot},
};
use tonic::async_trait;
use tracing::info;

pub struct IdentityHandler {
    event_tx: broadcast::Sender<Event>,
    status: ServiceStatus,
}
pub struct IdentityOptions {
    pub event_tx: broadcast::Sender<Event>,
}

pub enum IdentityMessage {
    GetMachineId {
        reply_to: oneshot::Sender<Result<String>>,
    },
    GetProvisionStatus {
        reply_to: oneshot::Sender<Result<bool>>,
    },
    GetMachineCert {
        reply_to: oneshot::Sender<Result<MachineCert>>,
    },
}

impl IdentityHandler {
    pub fn new(options: IdentityOptions) -> Self {
        Self {
            event_tx: options.event_tx,
            status: ServiceStatus::INACTIVE,
        }
    }
    pub async fn run(&mut self, mut message_rx: mpsc::Receiver<IdentityMessage>) -> Result<()> {
        // Start the service
        let _ = &self.start().await;
        loop {
            select! {
                msg = message_rx.recv() => {
                    if msg.is_none() {
                        continue;
                    }

                    match msg.unwrap() {
                        IdentityMessage::GetMachineId { reply_to } => {
                            // let code = generate_error();
                            let machine_id_result = get_machine_id();
                            let _ = reply_to.send(machine_id_result);
                        }
                        IdentityMessage::GetProvisionStatus { reply_to } => {
                            let provision_status = get_provision_status();
                            let _ = reply_to.send(provision_status);
                        }
                        IdentityMessage::GetMachineCert { reply_to } => {
                            let cert = get_machine_cert();
                            let _ = reply_to.send(cert);
                        }
                    };
                }
            }
        }
    }
}

#[async_trait]
impl ServiceHandler for IdentityHandler {
    async fn start(&mut self) -> Result<bool> {
        self.status = ServiceStatus::STARTED;
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

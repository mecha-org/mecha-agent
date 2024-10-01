use crate::service::{get_machine_cert, get_machine_id, get_provision_status};
use anyhow::Result;
use crypto::MachineCertDetails;
use events::Event;
use tokio::{
    select,
    sync::{broadcast, mpsc, oneshot},
};
use tracing::info;

#[derive()]
pub struct Settings {
    pub data_dir: String,
}
pub struct MachineCertPaths {
    pub machine_cert: String,
    pub machine_private_key: String,
    pub ca_bundle: String,
    pub root_cert: String,
}
pub struct IdentityHandler {
    _event_tx: broadcast::Sender<Event>,
    settings: Settings,
}
pub struct IdentityOptions {
    pub event_tx: broadcast::Sender<Event>,
    pub settings: Settings,
}

pub enum IdentityMessage {
    GetMachineId {
        reply_to: oneshot::Sender<Result<String>>,
    },
    GetProvisionStatus {
        reply_to: oneshot::Sender<Result<bool>>,
    },
    GetMachineCert {
        reply_to: oneshot::Sender<Result<MachineCertDetails>>,
    },
}

impl IdentityHandler {
    pub fn new(options: IdentityOptions) -> Self {
        Self {
            _event_tx: options.event_tx,
            settings: options.settings,
        }
    }
    pub async fn run(&mut self, mut message_rx: mpsc::Receiver<IdentityMessage>) -> Result<()> {
        info!(
            func = "run",
            package = env!("CARGO_PKG_NAME"),
            "identity service initiated"
        );

        loop {
            select! {
                msg = message_rx.recv() => {
                    if msg.is_none() {
                        continue;
                    }

                    match msg.unwrap() {
                        IdentityMessage::GetMachineId { reply_to } => {
                            let machine_id_result = get_machine_id(&self.settings.data_dir);
                            let _ = reply_to.send(machine_id_result);
                        }
                        IdentityMessage::GetProvisionStatus { reply_to } => {
                            let provision_status = get_provision_status(&self.settings.data_dir);
                            let _ = reply_to.send(provision_status);
                        }
                        IdentityMessage::GetMachineCert { reply_to } => {
                            let cert = get_machine_cert(&self.settings.data_dir);
                            let _ = reply_to.send(cert);
                        }
                    };
                }
            }
        }
    }
}

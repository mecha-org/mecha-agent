use anyhow::Result;
use events::Event;
use identity::handler::IdentityMessage;
use messaging::handler::MessagingMessage;
use settings::handler::SettingMessage;
use tokio::select;
use tokio::sync::{broadcast, mpsc};
use tokio_util::sync::CancellationToken;
use tracing::info;

const PACKAGE_NAME: &str = env!("CARGO_CRATE_NAME");
pub struct NetworkingHandler {
    identity_tx: mpsc::Sender<IdentityMessage>,
    messaging_tx: mpsc::Sender<MessagingMessage>,
    event_tx: broadcast::Sender<Event>,
    subscriber_token: Option<CancellationToken>,
}

pub enum NetworkingMessage {}

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
        }
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

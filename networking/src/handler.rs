use anyhow::{bail, Result};
use channel::recv_with_timeout;
use events::Event;
use identity::handler::IdentityMessage;
use messaging::handler::MessagingMessage;
use settings::handler::SettingMessage;
use tokio::{
    select,
    sync::{broadcast, mpsc, oneshot},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info};

use crate::service::networking_init;
pub struct NetworkingHandler {
    event_tx: broadcast::Sender<Event>,
    messaging_tx: mpsc::Sender<MessagingMessage>,
    identity_tx: mpsc::Sender<IdentityMessage>,
    setting_tx: mpsc::Sender<SettingMessage>,
    networking_task_token: Option<CancellationToken>,
}
pub struct NetworkingOptions {
    pub event_tx: broadcast::Sender<Event>,
    pub messaging_tx: mpsc::Sender<MessagingMessage>,
    pub identity_tx: mpsc::Sender<IdentityMessage>,
    pub setting_tx: mpsc::Sender<SettingMessage>,
}

pub enum NetworkingMessage {
    Start {
        reply_to: oneshot::Sender<Result<bool>>,
    },
}

impl NetworkingHandler {
    pub fn new(options: NetworkingOptions) -> Self {
        Self {
            event_tx: options.event_tx,
            messaging_tx: options.messaging_tx,
            identity_tx: options.identity_tx,
            setting_tx: options.setting_tx,
            networking_task_token: None,
        }
    }
    async fn initialize_networking(&mut self) -> Result<bool> {
        let fn_name = "initialize_networking";
        // safety: check for existing cancel token, and cancel it
        let networking_task_token = &self.networking_task_token;
        if networking_task_token.is_some() {
            let _ = networking_task_token.as_ref().unwrap().cancel();
        }
        // create new token
        let networking_task_token = CancellationToken::new();
        let networking_task_token_cloned = Some(networking_task_token.clone());
        let mut nebula_process = None;
        match networking_init(
            self.setting_tx.clone(),
            self.identity_tx.clone(),
            self.messaging_tx.clone(),
        )
        .await
        {
            Ok(networking_init_res) => {
                println!("networking process started: {:?}", networking_init_res);
                nebula_process = Some(networking_init_res.nebula_process);
            }
            Err(e) => {
                error!(
                    func = fn_name,
                    package = env!("CARGO_PKG_NAME"),
                    "failed to start networking, error - {}",
                    e
                );
            }
        };
        let mut timer = tokio::time::interval(std::time::Duration::from_secs(10));
        let _: JoinHandle<Result<()>> = tokio::task::spawn(async move {
            loop {
                select! {
                    _ = networking_task_token.cancelled() => {
                        info!(
                            func = fn_name,
                            package = env!("CARGO_PKG_NAME"),
                            "networking task cancelled"
                        );
                        if nebula_process.is_some() {
                            match nebula_process.unwrap().kill().await {
                                Ok(_) => {
                                    info!(
                                        func = fn_name,
                                        package = env!("CARGO_PKG_NAME"),
                                        "networking process killed"
                                    );
                                }
                                Err(e) => {
                                    error!(
                                        func = fn_name,
                                        package = env!("CARGO_PKG_NAME"),
                                        "failed to kill networking process, error - {}",
                                        e
                                    );
                                }
                            }
                        }
                        return Ok(());
                    }
                    _ = timer.tick() => {
                        if nebula_process.is_some(){
                            println!("networking process is running");
                        } else {
                            println!("networking process is not running");
                        }
                    }
                }
            }
        });
        self.networking_task_token = networking_task_token_cloned;
        Ok(true)
    }

    pub fn kill_networking_process(&self) -> Result<bool> {
        let exist_networking_task_token = &self.networking_task_token;
        if exist_networking_task_token.is_some() {
            let _ = exist_networking_task_token.as_ref().unwrap().cancel();
        } else {
            return Ok(false);
        }
        Ok(true)
    }

    pub async fn run(&mut self, mut message_rx: mpsc::Receiver<NetworkingMessage>) -> Result<()> {
        info!(func = "run", package = env!("CARGO_PKG_NAME"), "init");
        let fn_name = "run";
        // Start the service
        let mut event_rx = self.event_tx.subscribe();

        loop {
            select! {
                msg = message_rx.recv() => {
                    if msg.is_none() {
                        continue;
                    }

                    match msg.unwrap() {
                        NetworkingMessage::Start { reply_to } => {
                            let res = self.initialize_networking().await;
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
                        Event::Provisioning(events::ProvisioningEvent::Provisioned) => {}
                        Event::Provisioning(events::ProvisioningEvent::Deprovisioned) => {
                            let _ = &self.kill_networking_process();
                        }
                        Event::Messaging(_) => {}
                        Event::Settings(events::SettingEvent::Synced) => {
                            info!(
                                func = "run",
                                package = env!("CARGO_PKG_NAME"),
                                "settings sync event received"
                            );
                            let (tx, rx) = oneshot::channel();
                            match self
                                .setting_tx
                                .clone()
                                .send(SettingMessage::GetSettingsByKey {
                                    reply_to: tx,
                                    key: String::from("networking.enabled"),
                                })
                                .await
                            {
                                Ok(_) => (),
                                Err(e) => {
                                    error!(
                                        func = fn_name,
                                        package = env!("CARGO_PKG_NAME"),
                                        "failed to send get networking enabled message, error - {}",
                                        e
                                    );
                                }
                            }
                            let networking_enabled = match recv_with_timeout(rx).await {
                                Ok(r) => r,
                                Err(e) => {
                                    error!(
                                        func = fn_name,
                                        package = env!("CARGO_PKG_NAME"),
                                        "failed to receive get networking enabled message, error - {}",
                                        e
                                    );
                                "".to_string()}
                            };
                            info!("after settings sync networking enabled is {}", networking_enabled);
                            if networking_enabled == "true" {
                                let _ = &self.initialize_networking().await;
                            } else if networking_enabled == "false" {
                                let _ = &self.kill_networking_process();
                            } else {
                                // Can be add other function to perform
                            }
                        }
                        Event::Settings(events::SettingEvent::Updated { settings }) => {
                            info!(
                                func = "run",
                                package = env!("CARGO_PKG_NAME"),
                                "settings updated event received"
                            );
                            match settings.get("networking.enabled") {
                                Some(value) => {
                                    info!("after settings update networking enabled is {}", value);
                                    if value == "true" {
                                        let _ = &self.initialize_networking().await;
                                    } else if value == "false" {
                                        let _ = &self.kill_networking_process();
                                    } else {
                                        // Can be add other function to perform
                                    }
                                }
                                None => (),
                            }
                        }
                        Event::Nats(_) => {}
                    }
                }
            }
        }
    }
}

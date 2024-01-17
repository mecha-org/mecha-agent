use std::ops::Deref;

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
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
use tonic::async_trait;
use tracing::{error, info, warn};

use crate::service::{
    device_provision_status, get_time_interval, send_heartbeat, SendHeartbeatOptions,
};

pub struct HeartbeatHandler {
    event_tx: broadcast::Sender<Event>,
    messaging_tx: Sender<MessagingMessage>,
    identity_tx: Sender<IdentityMessage>,
    timer_token: Option<CancellationToken>,
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
            timer_token: None,
        }
    }
    pub async fn set_timer(&mut self) -> Result<()> {
        info!(func = "set_timer", package = env!("CARGO_PKG_NAME"), "init");

        // safety: check for existing cancel token, and cancel it
        let exist_timer_token = &self.timer_token;
        if exist_timer_token.is_some() {
            let _ = exist_timer_token.as_ref().unwrap().cancel();
        }

        // create a new token
        let timer_token = CancellationToken::new();
        let timer_token_cloned = timer_token.clone();
        let messaging_tx = self.messaging_tx.clone();
        let identity_tx = self.identity_tx.clone();

        // create spawn for timer
        let _: JoinHandle<Result<()>> = tokio::task::spawn(async move {
            let interval_in_secs: u64 = get_time_interval();
            let mut timer = tokio::time::interval(std::time::Duration::from_secs(interval_in_secs));
            loop {
                select! {
                        _ = timer_token.cancelled() => {
                            return Ok(());
                        },
                        _ = timer.tick() => {
                            let _ = send_heartbeat(SendHeartbeatOptions {
                                messaging_tx: messaging_tx.clone(),
                                identity_tx: identity_tx.clone(),
                            }).await;
                    }
                }
            }
        });

        // Save to state
        self.timer_token = Some(timer_token_cloned);

        Ok(())
    }

    pub fn clear_timer(&self) -> Result<bool> {
        let exist_timer_token = &self.timer_token;
        if exist_timer_token.is_some() {
            let _ = exist_timer_token.as_ref().unwrap().cancel();
        } else {
            return Ok(false);
        }
        Ok(true)
    }

    pub async fn run(&mut self, mut message_rx: mpsc::Receiver<HeartbeatMessage>) -> Result<()> {
        info!(func = "run", package = env!("CARGO_PKG_NAME"), "init");
        let mut event_rx = self.event_tx.subscribe();

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
                            Event::Messaging(events::MessagingEvent::Connected) => {
                                // start
                                let _ = &self.set_timer().await;
                            },
                            Event::Messaging(events::MessagingEvent::Disconnected) => {
                                let _ = &self.clear_timer();
                            },
                            Event::Provisioning(events::ProvisioningEvent::Deprovisioned) => {
                                let _ = &self.clear_timer();
                            },
                            _ => {},
                        }
                    }
            }
        }
    }
}

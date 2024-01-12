use anyhow::bail;
use anyhow::Result;
use events::Event;
use events::ProvisioningEvent;
use heartbeat::handler::HeartbeatHandler;
use heartbeat::handler::HeartbeatMessage;
use heartbeat::handler::HeartbeatOptions;
use identity::handler::IdentityMessage;
use messaging::handler::MessagingMessage;
use provisioning::handler::ProvisioningMessage;
use provisioning::handler::ProvisioningOptions;
use std::collections::HashMap;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::task;
use tokio_util::sync::CancellationToken;
use tracing::error;
use tracing::info;

use crate::errors::AgentError;
use crate::errors::AgentErrorCodes;

const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");

#[derive(Debug, Hash, PartialEq, Eq)]
pub enum SubscriberKey {
    Provisioning = 0,
    Heartbeat = 1,
}

pub struct GlobalSubscriberOpts {
    pub event_tx: broadcast::Sender<Event>,
    pub messaging_tx: mpsc::Sender<MessagingMessage>,
    pub identity_tx: mpsc::Sender<IdentityMessage>,
}

pub struct GlobalSubscriber {
    event_tx: broadcast::Sender<Event>,
    messaging_tx: mpsc::Sender<MessagingMessage>,
    identity_tx: mpsc::Sender<IdentityMessage>,
    cancel_token_map: Option<HashMap<SubscriberKey, CancellationToken>>,
}

impl GlobalSubscriber {
    pub fn new(opts: GlobalSubscriberOpts) -> Self {
        let GlobalSubscriberOpts {
            event_tx,
            messaging_tx,
            identity_tx,
        } = opts;

        Self {
            event_tx,
            messaging_tx,
            identity_tx,
            cancel_token_map: None,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        info!("global_subscriber: init");
        let event_tx = &self.event_tx;
        let mut event_tx_cloned = event_tx.subscribe();
        loop {
            tokio::select! {
                evt = event_tx_cloned.recv() => {
                    info!("global_subscriber: event_tx_cloned recv - {:?}", evt);
                    if evt.is_err() {
                        continue;
                    }

                    let _ = match evt.unwrap() {
                        Event::Provisioning(prov_evt) => {
                            let _ = match prov_evt {
                                ProvisioningEvent::Provisioned => {
                                    let _ = &self.start_all().await;
                                },
                                ProvisioningEvent::Deprovisioned => {
                                    let _ = &self.stop_all();
                                },
                                _ => {},
                            };
                        },
                        _ => {},
                    };
                },
            }
        }
    }

    async fn start_all(&mut self) -> Result<()> {
        // heartbeat service
        let heartbeat_opt = HeartbeatOptions {
            event_tx: self.event_tx.clone(),
            messaging_tx: self.messaging_tx.clone(),
            identity_tx: self.identity_tx.clone(),
        };
        let (heartbeat_t, heartbeat_cancel_token) = init_heartbeat_service(heartbeat_opt).await;

        // create cancellation token map
        let mut cancel_token_map = HashMap::new();

        // insert all
        cancel_token_map.insert(SubscriberKey::Heartbeat, heartbeat_cancel_token);

        // set to self member
        self.cancel_token_map = Some(cancel_token_map);

        // await all handles
        // let _ = heartbeat_t.await.unwrap();

        Ok(())
    }

    fn stop_all(&self) -> Result<bool> {
        println!("stop_all subscribers");
        let cancel_token_map = match &self.cancel_token_map {
            Some(m) => m,
            None => return Ok(false),
        };

        println!(
            "iter cancel_token_map {}",
            cancel_token_map.values().count()
        );

        for (k, c_token) in cancel_token_map.into_iter() {
            println!("cancelling token {:?}", k);
            c_token.clone().cancel();
        }

        Ok(true)
    }
}

async fn init_heartbeat_service(
    opt: HeartbeatOptions,
) -> (task::JoinHandle<Result<()>>, CancellationToken) {
    let token = CancellationToken::new();
    let cloned_token = token.clone();

    let heartbeat_t = tokio::spawn(async move {
        match HeartbeatHandler::new(opt).subscribe(cloned_token).await {
            Ok(_) => (),
            Err(e) => {
                error!(
                    func = "init_heartbeat_service",
                    package = PACKAGE_NAME,
                    "error init/run heartbeat service: {:?}",
                    e
                );
                bail!(AgentError::new(
                    AgentErrorCodes::HeartbeatInitError,
                    format!("error init/run heartbeat service: {:?}", e),
                    true
                ));
            }
        }
        Ok(())
    });

    (heartbeat_t, token)
}

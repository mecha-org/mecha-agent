use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use std::str::FromStr;

use agent_settings::{read_settings_yml, AgentSettings};
use anyhow::{bail, Result};
use crypto::random::generate_random_alphanumeric;
use futures::StreamExt;
use local_ip_address::list_afinet_netifas;
use messaging::handler::MessagingMessage;
use messaging::Subscriber as NatsSubscriber;
use serde::{Deserialize, Serialize};
use std::io;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tokio::sync::mpsc::Receiver;
use tokio::task::JoinHandle;
use tokio::{select, sync::oneshot};
use tracing::{error, info, warn};

use crate::handler::NetworkingMessage;

const PACKAGE_NAME: &str = env!("CARGO_CRATE_NAME");

pub enum HandshakeMessage {
    Request {
        machine_id: String,
        reply_to: oneshot::Sender<Result<bool>>,
    },
    HandshakeManifest {
        manifest: Manifest,
    },
}

struct Candidate {
    ip: Ipv4Addr,
    port: u32,
}

pub struct Manifest {
    txn_id: u32,
    candidates: Option<Candidates>,
}

struct Candidates {
    local: Vec<Candidate>,
    stun: Option<Candidate>, //once we have function to get reflexive address, we can remove this option
}
pub enum TransactionStatus {
    TxnState { machine_id: String, state: String },
}

#[derive()]
pub struct HandshakeChannelHandler {
    pub id: String,
    messaging_tx: mpsc::Sender<MessagingMessage>,
    disco_socket: Option<tokio::net::UdpSocket>,
    txns: HashMap<String, TransactionStatus>,
}
impl HandshakeChannelHandler {
    pub fn new(disco_url: String, messaging_tx: mpsc::Sender<MessagingMessage>) -> (Self, String) {
        let id = generate_random_alphanumeric(32);
        (
            Self {
                id: id.clone(),
                messaging_tx: messaging_tx,
                disco_socket: None,
                txns: HashMap::new(),
            },
            id,
        )
    }
    pub async fn run(&mut self, mut message_rx: mpsc::Receiver<HandshakeMessage>) -> Result<()> {
        info!(func = "run", package = PACKAGE_NAME, "init");
        //Todo: Start Disco
        match start_disco().await {
            Ok(_) => {}
            Err(e) => {
                error!(
                    func = "run",
                    package = PACKAGE_NAME,
                    "Error starting disco: {}",
                    e
                );
                bail!(e)
            }
        }
        loop {
            select! {
                msg = message_rx.recv() => {
                    if msg.is_none() {
                        continue;
                    }

                    match msg.unwrap() {
                        HandshakeMessage::Request { machine_id, reply_to } => {
                            let result = self.send_handshake_manifest(machine_id).await;
                            let _ = reply_to.send(result);
                        }
                        _ => {}
                    };
                },
            }
        }
    }
    async fn send_handshake_manifest(&mut self, machine_id: String) -> Result<bool> {
        let fn_name = "send_handshake_manifest";
        info!(func = fn_name, package = PACKAGE_NAME, "init");
        let settings: AgentSettings = match read_settings_yml() {
            Ok(settings) => settings,
            Err(_) => {
                warn!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "settings.yml not found, using default settings"
                );
                AgentSettings::default()
            }
        };
        // generate a txn ID, save it with machine_id
        let txn_id = generate_random_alphanumeric(32);
        let txn = TransactionStatus::TxnState {
            machine_id: machine_id,
            state: String::from("PENDING"),
        };
        self.txns.insert(txn_id, txn);
        let endpoints = match discover_endpoints() {
            Ok(endpoints) => endpoints,
            Err(e) => {
                warn!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "Error discovering endpoints: {}",
                    e
                );
                bail!(e)
            }
        };
        //1. Ge stun candidates
        //2. Create Manifest
        let local_candidates: Vec<Candidate> = endpoints
            .iter()
            .map(|endpoint| Candidate {
                ip: *endpoint,
                port: settings.networking.disco_port,
            })
            .collect();
        let candidates = Candidates {
            local: local_candidates,
            stun: None,
        };
        let manifest = Manifest {
            txn_id: 1,
            candidates: Some(candidates),
        };
        // send reply to NATS
        // self.messaging_tx
        //     .send(MessagingMessage::Send { reply_to: (), message: (), subject: () } { manifest });
        Ok(true)
    }
}

pub fn discover_endpoints() -> Result<Vec<Ipv4Addr>> {
    let network_interfaces = match list_afinet_netifas() {
        Ok(n) => n,
        Err(e) => {
            println!("Error discovering endpoints: {}", e);
            bail!(e)
        }
    };
    let mut endpoints: Vec<&IpAddr> = vec![];
    #[cfg(target_os = "macos")]
    for (name, ip) in network_interfaces.iter() {
        if name.to_lowercase().starts_with("en") {
            println!("{}:\t{:?}", name, ip);
            if ip.is_ipv4() {
                endpoints.push(ip);
            }
        }
    }

    #[cfg(target_os = "linux")]
    for (name, ip) in network_interfaces.iter() {
        if name.to_lowercase().starts_with("wlan")
            || name.to_lowercase().starts_with("eth")
            || name.to_lowercase().starts_with("en")
        {
            if (ip.is_ipv4()) {
                endpoints.push(ip);
            }
        }
    }
    // add port to each ip address
    let ipv4_addr: Vec<Ipv4Addr> = endpoints
        .iter()
        .map(|ip| Ipv4Addr::from_str(ip.to_string().as_str()).unwrap())
        .collect();

    Ok(ipv4_addr)
}

pub async fn await_networking_handshake_request(
    mut subscriber: NatsSubscriber,
    handshake_tx: mpsc::Sender<HandshakeMessage>,
) -> Result<()> {
    let machine_id = "machine_id".to_string();
    // Don't exit loop in any case by returning a response
    while let Some(message) = subscriber.next().await {
        let (tx, rx) = oneshot::channel();
        let _ = handshake_tx
            .send(HandshakeMessage::Request {
                machine_id: machine_id.clone(),
                reply_to: tx,
            })
            .await;
    }
    Ok(())
}

pub async fn start_disco() -> Result<()> {
    info!(func = "start_disco", package = PACKAGE_NAME, "init");
    let sock = match UdpSocket::bind("0.0.0.0:8080").await {
        Ok(s) => {
            info!(
                func = "start_disco",
                package = PACKAGE_NAME,
                "bound to socket: {:?}",
                s.local_addr().unwrap()
            );
            s
        }
        Err(e) => {
            error!(
                func = "start_disco",
                package = PACKAGE_NAME,
                "Error binding to socket: {}",
                e
            );
            bail!(e)
        }
    };
    let mut buf = [0; 1024];
    let _: JoinHandle<Result<()>> = tokio::task::spawn(async move {
        loop {
            let (len, addr) = sock.recv_from(&mut buf).await?;
            println!("{:?} bytes received from {:?}", len, addr);
        }
    });
    Ok(())
}

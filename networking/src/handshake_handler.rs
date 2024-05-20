use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;

use agent_settings::{read_settings_yml, AgentSettings};
use anyhow::{bail, Result};
use chrono::format;
use crypto::random::generate_random_alphanumeric;
use futures::StreamExt;
use local_ip_address::list_afinet_netifas;
use messaging::async_nats::{HeaderMap, Message};
use messaging::handler::MessagingMessage;
use messaging::Subscriber as NatsSubscriber;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::net::UdpSocket;
use tokio::select;
use tokio::sync::{mpsc, oneshot};
use tracing::{error, info, warn};

use crate::errors::{NetworkingError, NetworkingErrorCodes};
use crate::service::ChannelDetails;

const PACKAGE_NAME: &str = env!("CARGO_CRATE_NAME");

pub enum HandshakeMessage {
    Request {
        machine_id: String,
        reply_subject: String,
    },
    HandshakeManifest {
        manifest: Manifest,
    },
}

#[derive(Serialize, Deserialize, Debug)]
struct Candidate {
    ip: Ipv4Addr,
    port: u16,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Manifest {
    txn_id: String,
    candidates: Option<Candidates>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Candidates {
    local: Vec<Candidate>,
    stun: Option<Candidate>, //once we have function to get reflexive address, we can remove this option
}
#[derive(Serialize, Deserialize, Clone)]
pub enum TransactionStatus {
    TxnState { machine_id: String, state: String },
}

#[derive(Clone)]
pub struct HandshakeChannelHandler {
    pub channel_id: String,
    messaging_tx: mpsc::Sender<MessagingMessage>,
    pub handshake_tx: mpsc::Sender<HandshakeMessage>,
    txns: HashMap<String, TransactionStatus>,
}
impl HandshakeChannelHandler {
    pub fn new(
        messaging_tx: mpsc::Sender<MessagingMessage>,
        handshake_tx: mpsc::Sender<HandshakeMessage>,
    ) -> Self {
        let channel_id: String = generate_random_alphanumeric(32);
        Self {
            channel_id,
            messaging_tx,
            handshake_tx,
            txns: HashMap::new(),
        }
    }

    pub async fn start_disco(&self, disco_addr: String) -> Result<UdpSocket> {
        let sock = match create_disco_socket(disco_addr).await {
            Ok(s) => s,
            Err(e) => {
                error!(
                    func = "new",
                    package = PACKAGE_NAME,
                    "Error starting disco server: {}",
                    e
                );
                bail!("Error starting disco: {}", e); // TODO
            }
        };
        Ok(sock)
    }

    pub async fn run(
        &mut self,
        sock: &mut UdpSocket,
        message_rx: &mut mpsc::Receiver<HandshakeMessage>,
    ) -> Result<()> {
        info!(func = "run", package = PACKAGE_NAME, "init");
        let mut buf = [0; 1024];
        loop {
            select! {
                packet = sock.recv_from(&mut buf) => {
                    let (len, addr) = packet.unwrap();
                    println!("{:?} bytes received from {:?}", len, addr);
                },
                msg = message_rx.recv() => {
                    if msg.is_none() {
                        continue;
                    }

                    match msg.unwrap() {
                        HandshakeMessage::Request { machine_id, reply_subject } => {
                            let _result = self.send_handshake_manifest(machine_id, reply_subject).await;
                        }
                        _ => {}
                    };
                },
            }
        }
    }
    async fn send_handshake_manifest(
        &mut self,
        machine_id: String,
        reply_subject: String,
    ) -> Result<bool> {
        println!("handshake request received");
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
        self.txns.insert(txn_id.clone(), txn);
        // println!("txn_id: {:?}", txn_id);
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
        let addr: SocketAddr = match settings.networking.disco_addr.parse() {
            Ok(addr) => addr,
            Err(e) => {
                warn!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "failed to parse disco address: {}",
                    e
                );
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 8080)
            }
        };
        //1. Ge stun candidates
        //2. Create Manifest
        let local_candidates: Vec<Candidate> = endpoints
            .iter()
            .map(|endpoint| Candidate {
                ip: *endpoint,
                port: addr.port(),
            })
            .collect();
        let candidates = Candidates {
            local: local_candidates,
            stun: None,
        };
        let manifest = Manifest {
            txn_id: txn_id,
            candidates: Some(candidates),
        };
        let mut header_map: HashMap<String, String> = HashMap::new();
        header_map.insert(String::from("Message-Type"), String::from("REPLY"));
        println!("manifest: {:?}", manifest);
        // send reply to NATS
        let (tx, _rx) = oneshot::channel();
        let _ = self
            .messaging_tx
            .send(MessagingMessage::Send {
                subject: reply_subject,
                message: json!(manifest).to_string(),
                reply_to: tx,
                headers: Some(header_map),
            })
            .await;
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

pub async fn await_networking_handshake_message(
    mut subscriber: NatsSubscriber,
    handshake_tx: mpsc::Sender<HandshakeMessage>,
) -> Result<()> {
    let fn_name = "await_networking_handshake_handler";
    // Don't exit loop in any case by returning a response
    while let Some(message) = subscriber.next().await {
        println!("message received on handshake channel");
        match process_handshake_request(message, handshake_tx.clone()).await {
            Ok(_) => {}
            Err(err) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "error processing message - {:?}",
                    err
                );
            }
        }
    }
    Ok(())
}

async fn process_handshake_request(
    message: Message,
    handshake_tx: mpsc::Sender<HandshakeMessage>,
) -> Result<bool> {
    let fn_name = "process_handshake_request";

    // Process mesaage
    let payload_str = match std::str::from_utf8(&message.payload) {
        Ok(s) => s,
        Err(e) => {
            bail!(NetworkingError::new(
                NetworkingErrorCodes::ExtractMessagePayloadError,
                format!("error converting payload to string - {}", e),
            ))
        }
    };
    let message_type =
        match get_header_by_key(message.headers.clone(), String::from("Message-Type")) {
            Ok(s) => s,
            Err(e) => {
                error! {
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "error getting message type from headers - {}",
                    e
                };
                bail!(e)
            }
        };
    match message_type.as_str() {
        "REQUEST" => {
            let request_payload: ChannelDetails = match serde_json::from_str(&payload_str) {
                Ok(s) => s,
                Err(e) => bail!(NetworkingError::new(
                    NetworkingErrorCodes::PayloadDeserializationError,
                    format!("error while deserializing message payload {}", e),

                )),
            };
            info!(
                func = fn_name,
                package = PACKAGE_NAME,
                "received handshake request: {:?}",
                request_payload
            );
            let reply_subject = format!(
                "network.{}.node.handshake.{}",
                sha256::digest(request_payload.network_id.clone()),
                request_payload.channel.clone()
            );
            let _ = handshake_tx
                .send(HandshakeMessage::Request {
                    machine_id: request_payload.machine_id.clone(),
                    reply_subject: reply_subject,
                })
                .await;
        }
        "REPLY" => {
            let reply_payload: Manifest = match serde_json::from_str(&payload_str) {
                Ok(s) => s,
                Err(e) => bail!(NetworkingError::new(
                    NetworkingErrorCodes::PayloadDeserializationError,
                    format!("error while deserializing message payload {}", e),

                )),
            };
            println!("manifest received: {:?}", reply_payload);
        }
        _ => {
            warn!(
                func = fn_name,
                package = PACKAGE_NAME,
                "Unknown message type: {}",
                message_type
            );
        }
    }
    Ok(true)
}

fn get_header_by_key(headers: Option<HeaderMap>, header_key: String) -> Result<String> {
    let fn_name = "get_header_by_key";
    let message_headers = match headers {
        Some(h) => h,
        None => {
            warn!(
                func = fn_name,
                package = PACKAGE_NAME,
                "No headers found in message",
            );
            bail!(NetworkingError::new(
                NetworkingErrorCodes::ExtractMessageHeadersError,
                String::from("no headers found in message"),
            ))
        }
    };
    let message_type = match message_headers.get(header_key.as_str()) {
        Some(v) => v.to_string(),
        None => {
            warn!(
                func = fn_name,
                package = PACKAGE_NAME,
                "No message type found in message headers: {:?}",
                message_headers
            );
            String::from("")
        }
    };
    Ok(message_type)
}
pub async fn create_disco_socket(addr: String) -> Result<UdpSocket> {
    info!(func = "create_disco_socket", package = PACKAGE_NAME, "init");
    let sock = match UdpSocket::bind(addr).await {
        Ok(s) => {
            info!(
                func = "create_disco_socket",
                package = PACKAGE_NAME,
                "bound to socket: {:?}",
                s.local_addr().unwrap()
            );
            s
        }
        Err(e) => {
            error!(
                func = "create_disco_socket",
                package = PACKAGE_NAME,
                "Error binding to socket: {}",
                e
            );
            bail!(e)
        }
    };
    Ok(sock)
}

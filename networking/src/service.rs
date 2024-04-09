use std::net::SocketAddr;

use agent_settings::{networking::NetworkingSettings, read_settings_yml, AgentSettings};
use anyhow::{bail, Result};
use channel::recv_with_timeout;
use crypto::random::generate_random_alphanumeric;
use futures::StreamExt;
use identity::handler::IdentityMessage;
use messaging::Subscriber as NatsSubscriber;
use messaging::{
    async_nats::jetstream::consumer::{pull::Config, Consumer},
    handler::MessagingMessage,
    Message,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha256::digest;
use tokio::sync::{
    mpsc::{self, Sender},
    oneshot,
};
use tracing::{debug, error, info, trace, warn};
use wireguard::{PeerConfiguration, Wireguard};

use crate::errors::{NetworkingError, NetworkingErrorCodes};

#[derive(Serialize, Deserialize, Debug)]
struct NetworkDetails {
    machine_id: String,
    network_id: String,
    ipv4_addr: String,
    ipv6_addr: String,
    pub_key: String,
    candidates: Candidates,
}

#[derive(Serialize, Deserialize, Debug)]
struct Candidates {
    local: Option<SocketAddr>,
    stun: Option<SocketAddr>,
    turn: Option<SocketAddr>,
}
#[derive(Debug, Default)]
pub struct NetworkingSubscriber {
    pub handshake_request: Option<NatsSubscriber>,
}
#[derive(Debug)]
pub enum NetworkingSubject {
    HandshakeRequest(String),
}
const PACKAGE_NAME: &str = env!("CARGO_CRATE_NAME");
pub async fn subscribe_to_nats(
    identity_tx: mpsc::Sender<IdentityMessage>,
    messaging_tx: mpsc::Sender<MessagingMessage>,
    channel_id: String,
    network_id: String,
) -> Result<NetworkingSubscriber> {
    let fn_name = "subscribe_to_nats";
    // Get machine id
    let machine_id = match get_machine_id(identity_tx.clone()).await {
        Ok(id) => id,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error getting machine id - {}",
                e
            );
            bail!(e)
        }
    };
    let list_of_subjects = vec![NetworkingSubject::HandshakeRequest(format!(
        "network.{}.node.handshake.{channel_id}",
        sha256::digest(network_id.clone())
    ))];
    let mut networking_subscriber = NetworkingSubscriber::default();
    // Iterate over everything.
    for subject in list_of_subjects {
        let (tx, rx) = oneshot::channel();
        let subject_string = match &subject {
            NetworkingSubject::HandshakeRequest(s) => s.to_string(),
        };
        match messaging_tx
            .send(MessagingMessage::Subscriber {
                reply_to: tx,
                subject: subject_string,
            })
            .await
        {
            Ok(_) => {}
            Err(e) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "error sending get que subscriber for issue token- {}",
                    e
                );
                bail!(NetworkingError::new(
                    NetworkingErrorCodes::ChannelSendMessageError,
                    format!("error sending subscriber message - {}", e),
                    true
                ));
            }
        }
        match recv_with_timeout(rx).await {
            Ok(subscriber) => match &subject {
                NetworkingSubject::HandshakeRequest(_) => {
                    networking_subscriber.handshake_request = Some(subscriber)
                }
            },
            Err(e) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "error while get networking subscriber - {:?}, error - {}",
                    &subject,
                    e
                );
                bail!(NetworkingError::new(
                    NetworkingErrorCodes::ChannelReceiveMessageError,
                    format!(
                        "error get networking subscriber - {:?}, error - {}",
                        &subject, e
                    ),
                    true
                ));
            }
        };
    }

    Ok(networking_subscriber)
}
pub async fn configure_wireguard(
    messaging_tx: Sender<MessagingMessage>,
    identity_tx: Sender<IdentityMessage>,
) -> Result<Wireguard> {
    let fn_name = "configure_wireguard";
    // read settings from settings.yml
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
    // The agent will pull the networking settings
    // Generate a wireguard private key + public key
    let keys = match wireguard::generate_new_key_pair() {
        Ok(keys) => keys,
        Err(e) => {
            warn!(
                func = fn_name,
                package = PACKAGE_NAME,
                error = e.to_string().as_str(),
                "Error generating wireguard key pair"
            );
            bail!(NetworkingError::new(
                NetworkingErrorCodes::GenerateKeyPairError,
                format!("error generating key pair - {}", e),
                true
            ))
        }
    };

    // Configure a wireguard interface as per settings.yml and machine settings
    let mut wireguard = Wireguard::new(settings.networking.wireguard.tun);
    let wg_config = wireguard::WgConfig {
        ip_address: settings.networking.peer_settings.ipv4_address.clone(),
        port: settings.networking.wireguard.port,
    };
    match wireguard.setup_wireguard(&wg_config, keys.secret_key.clone()) {
        Ok(_) => {
            println!("Wireguard interface setup successfully");
            ()
        }
        Err(e) => {
            warn!(
                func = fn_name,
                package = PACKAGE_NAME,
                error = e.to_string().as_str(),
                "Error setting up wireguard interface"
            );
            bail!(NetworkingError::new(
                NetworkingErrorCodes::SettingUpWireguardError,
                format!("error setting up wireguard interface - {}", e),
                true
            ))
        }
    }
    let machine_id = match get_machine_id(identity_tx.clone()).await {
        Ok(id) => id,
        Err(e) => {
            warn!(
                func = fn_name,
                package = PACKAGE_NAME,
                error = e.to_string().as_str(),
                "Error getting machine id"
            );
            bail!(e)
        }
    };
    let (tx, rx) = tokio::sync::oneshot::channel();
    let subject = format!(
        "machine.{}.networking.network.{}.node.{}",
        digest(machine_id.clone()),
        digest(settings.networking.peer_settings.network_id.clone()),
        digest(format!(
            "{}:{}",
            settings.networking.peer_settings.ipv4_address,
            settings.networking.peer_settings.ipv6_address
        ))
    );
    let payload = NetworkDetails {
        machine_id: machine_id.clone(),
        network_id: settings.networking.peer_settings.network_id.clone(),
        ipv4_addr: settings.networking.peer_settings.ipv4_address.clone(),
        ipv6_addr: settings.networking.peer_settings.ipv6_address.clone(),
        pub_key: keys.public_key.clone(),
        candidates: Candidates {
            local: None,
            stun: None,
            turn: None,
        },
    };

    // Publish its own peer information
    let _ = match messaging_tx
        .send(MessagingMessage::Send {
            reply_to: tx,
            message: json!(payload).to_string(),
            subject: subject,
        })
        .await
    {
        Ok(_) => {
            println!("networking node message sent");
            ()
        }
        Err(e) => {
            warn!(
                func = fn_name,
                package = PACKAGE_NAME,
                error = e.to_string().as_str(),
                "Error sending message to messaging"
            );
            bail!(NetworkingError::new(
                NetworkingErrorCodes::UnknownError,
                format!("error sending message to messaging - {}", e),
                true
            ))
        }
    };
    Ok(wireguard)
}

pub async fn await_consumer_message(
    consumer: Consumer<Config>,
    messaging_tx: Sender<MessagingMessage>,
    wireguard: Wireguard,
) -> Result<bool> {
    println!("awaiting consumer message");
    let fn_name = "await_consumer_message";
    let settings = match read_settings_yml() {
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
    let mut messages = match consumer.messages().await {
        Ok(s) => s,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error fetching messages, error -  {:?}",
                e
            );
            bail!(NetworkingError::new(
                NetworkingErrorCodes::PullMessagesError,
                format!("pull messages error - {:?} - {}", e.kind(), e.to_string()),
                true
            ))
        }
    };

    while let Some(Ok(message)) = messages.next().await {
        println!("message in consumer stream {:?}", message.payload);
        match process_message(
            settings.networking.clone(),
            message.clone(),
            wireguard.clone(),
        )
        .await
        {
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

        // Acknowledges a message delivery
        match message.ack().await {
            Ok(res) => println!("message Acknowledged {:?}", res),
            Err(err) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "message acknowledge failed {}",
                    err
                );
            }
        };
    }
    info!(
        func = fn_name,
        package = PACKAGE_NAME,
        "message delivery acknowledged"
    );
    Ok(true)
}

async fn process_message(
    settings: NetworkingSettings,
    message: Message,
    mut wireguard: Wireguard,
) -> Result<bool> {
    let fn_name = "process_message";
    trace!(
        func = fn_name,
        package = PACKAGE_NAME,
        "processing message - {:?}",
        message
    );
    let node_id_hash = digest(format!(
        "{}:{}",
        settings.peer_settings.ipv4_address, settings.peer_settings.ipv6_address
    ));
    // Do not process message if it is from same node
    if message.subject.contains(&node_id_hash) {
        info!(
            func = fn_name,
            package = PACKAGE_NAME,
            "message from same node, ignoring"
        );
        match message.ack().await {
            Ok(_) => {
                println!("networking node message acknowledged")
            }
            Err(e) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "error acknowledging message - {:?}",
                    e
                );
                bail!(NetworkingError::new(
                    NetworkingErrorCodes::MessageAcknowledgeError,
                    format!("error acknowledging message - {:?}", e),
                    true
                ))
            }
        };
        return Ok(true);
    }
    // Process mesaage
    let payload_str = match std::str::from_utf8(&message.payload) {
        Ok(s) => s,
        Err(e) => {
            bail!(NetworkingError::new(
                NetworkingErrorCodes::ExtractMessagePayloadError,
                format!("error converting payload to string - {}", e),
                true
            ))
        }
    };
    let request_payload: NetworkDetails = match serde_json::from_str(&payload_str) {
        Ok(s) => s,
        Err(e) => bail!(NetworkingError::new(
            NetworkingErrorCodes::PayloadDeserializationError,
            format!("error while deserializing message payload {}", e),
            true
        )),
    };
    let peer_configuration = PeerConfiguration {
        name: request_payload.machine_id.clone(),
        public_key: request_payload.pub_key.clone(),
        endpoint: vec![],
        allowed_ips: vec![],
    };
    println!("peer configuration {:?}", peer_configuration.clone());
    match wireguard.add_peer(peer_configuration) {
        Ok(_) => (),
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error adding peer - {:?}",
                e
            );
            bail!("error while adding peer {:?}", e)
        }
    }
    match message.ack().await {
        Ok(_) => {
            println!("networking node message acknowledged")
        }
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error acknowledging message - {:?}",
                e
            );
            bail!(NetworkingError::new(
                NetworkingErrorCodes::MessageAcknowledgeError,
                format!("error acknowledging message - {:?}", e),
                true
            ))
        }
    }
    info!(
        func = fn_name,
        package = PACKAGE_NAME,
        "message processed successfully"
    );
    Ok(true)
}
pub async fn create_pull_consumer(
    messaging_tx: Sender<MessagingMessage>,
    identity_tx: Sender<IdentityMessage>,
) -> Result<Consumer<Config>> {
    let fn_name = "create_pull_consumer";
    // read settings from settings.yml
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
    let (tx, rx) = oneshot::channel();
    match messaging_tx
        .send(MessagingMessage::InitJetStream { reply_to: tx })
        .await
    {
        Ok(_) => {}
        Err(err) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error sending init jetstream message - {:?}",
                err
            );
            bail!(NetworkingError::new(
                NetworkingErrorCodes::ChannelSendMessageError,
                format!(
                    "error sending init jetstream message - {:?}",
                    err.to_string()
                ),
                true
            ))
        }
    }

    let jet_stream_client = match recv_with_timeout(rx).await {
        Ok(js_client) => js_client,
        Err(err) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error receiving init jetstream message - {:?}",
                err
            );
            bail!(NetworkingError::new(
                NetworkingErrorCodes::ChannelReceiveMessageError,
                format!("error receiving init jetstream message - {:?}", err),
                true
            ))
        }
    };
    debug!(
        func = fn_name,
        package = PACKAGE_NAME,
        "jetstream client created"
    );
    let stream_name = "networking_v1";
    let stream = match jet_stream_client.get_stream(stream_name.to_string()).await {
        Ok(s) => s,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error getting stream, name - {}, error -  {:?}",
                stream_name,
                e
            );
            bail!(e)
        }
    };

    // Create consumer
    let consumer_name = generate_random_alphanumeric(10);
    debug!(
        func = fn_name,
        package = PACKAGE_NAME,
        "consumer name generated - {}",
        &consumer_name
    );
    let network_id = settings.networking.peer_settings.network_id.clone();
    let filter_subject = format!("networking.networks.{}.nodes.*", digest(network_id));
    println!("filter subject {:?}", filter_subject);
    let consumer = match jet_stream_client
        .create_consumer(stream, filter_subject, consumer_name.clone())
        .await
    {
        Ok(s) => s,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error creating consumer, name - {}, error -  {:?}",
                &consumer_name,
                e
            );
            bail!(e)
        }
    };
    println!("pull consumer created!");
    Ok(consumer)
}
async fn get_machine_id(identity_tx: mpsc::Sender<IdentityMessage>) -> Result<String> {
    let (tx, rx) = oneshot::channel();
    match identity_tx
        .clone()
        .send(IdentityMessage::GetMachineId { reply_to: tx })
        .await
    {
        Ok(_) => {}
        Err(e) => {
            error!(
                func = "get_machine_id",
                package = PACKAGE_NAME,
                "error sending get machine id message - {}",
                e
            );
            bail!(NetworkingError::new(
                NetworkingErrorCodes::ChannelSendMessageError,
                format!("error sending get machine id message - {}", e),
                true
            ));
        }
    }
    let machine_id = match recv_with_timeout(rx).await {
        Ok(id) => id,
        Err(e) => {
            error!(
                func = "get_machine_id",
                package = PACKAGE_NAME,
                "error receiving get machine id message - {}",
                e
            );
            bail!(NetworkingError::new(
                NetworkingErrorCodes::ChannelReceiveMessageError,
                format!("error receiving get machine id message - {}", e),
                true
            ));
        }
    };
    info!(
        func = "get_machine_id",
        package = PACKAGE_NAME,
        "get machine id request completed",
    );
    Ok(machine_id)
}

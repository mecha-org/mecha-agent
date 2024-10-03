use std::collections::HashMap;
use std::net::SocketAddr;

use agent_settings::{read_settings_yml, AgentSettings};
use anyhow::{bail, Result};
use channel::{recv_with_custom_timeout, recv_with_timeout};
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
use settings::handler::SettingMessage;
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
pub struct ChannelDetails {
    pub machine_id: String,
    pub network_id: String,
    pub channel: String,
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
pub async fn get_networking_subscriber(
    settings_tx: mpsc::Sender<SettingMessage>,
    messaging_tx: mpsc::Sender<MessagingMessage>,
    channel_id: String,
) -> Result<NetworkingSubscriber> {
    let fn_name = "get_networking_subscriber";
    let network_id =
        match get_settings_by_key(settings_tx.clone(), String::from("networking.network_id")).await
        {
            Ok(id) => id,
            Err(e) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    error = e.to_string().as_str(),
                    "error getting network id"
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
                    "error sending get subscriber networking- {}",
                    e
                );
                bail!(NetworkingError::new(
                    NetworkingErrorCodes::ChannelSendMessageError,
                    format!("error sending subscriber message - {}", e),
                ));
            }
        }
        match recv_with_custom_timeout(5000, rx).await {
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
                ));
            }
        };
    }

    Ok(networking_subscriber)
}
pub async fn configure_wireguard(
    wg_port: u32,
    if_name: &str,
    settings_tx: Sender<SettingMessage>,
) -> Result<Wireguard> {
    let fn_name = "configure_wireguard";

    // The agent will pull the networking settings
    // Generate a wireguard private key + public key
    let keys = match wireguard::generate_new_key_pair() {
        Ok(keys) => keys,
        Err(e) => {
            warn!(
                func = fn_name,
                package = PACKAGE_NAME,
                error = e.to_string().as_str(),
                "error generating wireguard key pair"
            );
            bail!(NetworkingError::new(
                NetworkingErrorCodes::GenerateKeyPairError,
                format!("error generating key pair - {}", e),
            ))
        }
    };

    let ip_address = match get_ip_address(settings_tx.clone()).await {
        Ok(ip) => ip,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                error = e.to_string().as_str(),
                "error getting ip address"
            );
            bail!(e)
        }
    };
    debug!(
        func = fn_name,
        package = PACKAGE_NAME,
        "ip address fetched - {}",
        ip_address
    );
    // Configure a wireguard interface as per settings.yml and machine settings
    let mut wireguard = Wireguard::new(if_name.to_string());
    let wg_config = wireguard::WgConfig {
        ip_address: ip_address,
        port: wg_port,
    };
    match wireguard.setup_wireguard(&wg_config, keys.secret_key.clone()) {
        Ok(_) => (),
        Err(e) => {
            warn!(
                func = fn_name,
                package = PACKAGE_NAME,
                error = e.to_string().as_str(),
                "error setting up wireguard interface"
            );
            bail!(NetworkingError::new(
                NetworkingErrorCodes::SettingUpWireguardError,
                format!("error setting up wireguard interface - {}", e),
            ))
        }
    }
    Ok(wireguard)
}

pub async fn publish_networking_channel(
    channel_id: String,
    messaging_tx: Sender<MessagingMessage>,
    identity_tx: Sender<IdentityMessage>,
    settings_tx: Sender<SettingMessage>,
) -> Result<bool> {
    let fn_name = "publish_networking_channel";
    let machine_id = match get_machine_id(identity_tx.clone()).await {
        Ok(id) => id,
        Err(e) => {
            warn!(
                func = fn_name,
                package = PACKAGE_NAME,
                error = e.to_string().as_str(),
                "error getting machine id"
            );
            bail!(e)
        }
    };
    let network_id =
        match get_settings_by_key(settings_tx.clone(), String::from("networking.network_id")).await
        {
            Ok(id) => id,
            Err(e) => {
                warn!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    error = e.to_string().as_str(),
                    "error getting machine id"
                );
                bail!(e)
            }
        };
    // Exchange the channel details
    let (tx, rx) = tokio::sync::oneshot::channel();
    let subject = format!(
        "machine.{}.networking.network.{}.channel",
        digest(machine_id.clone()),
        digest(network_id.clone()),
    );
    println!("subject to publish channel details: {:?}", subject);
    let payload: ChannelDetails = ChannelDetails {
        machine_id: machine_id.clone(),
        network_id: network_id.clone(),
        channel: channel_id, //TODO: Change to channel id
    };

    // Publish channel information
    let _ = match messaging_tx
        .send(MessagingMessage::Send {
            reply_to: tx,
            message: json!(payload).to_string(),
            subject: subject,
            headers: None,
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
            ))
        }
    };
    Ok(true)
}
pub async fn await_consumer_message(
    consumer: Consumer<Config>,
    messaging_tx: Sender<MessagingMessage>,
    settings_tx: Sender<SettingMessage>,
    channel_id: String,
    machine_id: String,
) -> Result<bool> {
    println!("awaiting consumer message");
    let fn_name = "await_consumer_message";
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
            ))
        }
    };
    let network_id =
        match get_settings_by_key(settings_tx.clone(), String::from("networking.network_id")).await
        {
            Ok(id) => id,
            Err(e) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    error = e.to_string().as_str(),
                    "error getting network id"
                );
                bail!(e)
            }
        };
    while let Some(Ok(message)) = messages.next().await {
        println!("message in consumer stream {:?}", message.payload);
        match process_consumer_message(
            message.clone(),
            network_id.clone(),
            channel_id.clone(),
            machine_id.clone(),
            messaging_tx.clone(),
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
    }
    info!(
        func = fn_name,
        package = PACKAGE_NAME,
        "message delivery acknowledged"
    );
    Ok(true)
}

async fn process_consumer_message(
    message: Message,
    network_id: String,
    channel_id: String,
    machine_id: String,
    messaging_tx: Sender<MessagingMessage>,
) -> Result<bool> {
    let fn_name = "process_message";
    trace!(
        func = fn_name,
        package = PACKAGE_NAME,
        "processing message - {:?}",
        message
    );

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
    let request_payload: ChannelDetails = match serde_json::from_str(&payload_str) {
        Ok(s) => s,
        Err(e) => bail!(NetworkingError::new(
            NetworkingErrorCodes::PayloadDeserializationError,
            format!("error while deserializing message payload {}", e),
        )),
    };
    //Do not process message if it is from same channel
    if channel_id.contains(&request_payload.channel) {
        println!("channel id matched!");
        info!(
            func = fn_name,
            package = PACKAGE_NAME,
            "message from same node, ignoring"
        );
        return Ok(true);
    }
    let subject_to_publish_channel_info = format!(
        "network.{}.node.handshake.{}",
        digest(network_id.clone()),
        request_payload.channel.clone()
    );
    println!(
        "subject to publish handshake request {}",
        subject_to_publish_channel_info
    );
    let channel_details_payload = ChannelDetails {
        machine_id: machine_id.clone(),
        network_id: network_id.clone(),
        channel: channel_id.clone(),
    };
    let mut headers = HashMap::new();
    headers.insert(String::from("Message-Type"), String::from("REQUEST"));
    let (tx, rx) = oneshot::channel();
    match messaging_tx
        .send(MessagingMessage::Send {
            reply_to: tx,
            message: json!(channel_details_payload).to_string(),
            subject: subject_to_publish_channel_info,
            headers: Some(headers),
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
            ));
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
pub async fn create_channel_sync_consumer(
    messaging_tx: Sender<MessagingMessage>,
    identity_tx: Sender<IdentityMessage>,
    settings_tx: Sender<SettingMessage>,
) -> Result<Consumer<Config>> {
    let fn_name = "create_channel_sync_consumer";

    let machine_id = match get_machine_id(identity_tx.clone()).await {
        Ok(id) => id,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                error = e.to_string().as_str(),
                "Error getting machine id"
            );
            bail!(e)
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
    let network_id =
        match get_settings_by_key(settings_tx.clone(), String::from("networking.network_id")).await
        {
            Ok(id) => id,
            Err(e) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    error = e.to_string().as_str(),
                    "Error getting network id"
                );
                bail!(e)
            }
        };

    let filter_subject = format!("networking.networks.{}.channels.*", digest(network_id));

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
    info!(
        func = fn_name,
        package = PACKAGE_NAME,
        "consumer created successfully"
    );
    Ok(consumer)
}
pub async fn get_machine_id(identity_tx: mpsc::Sender<IdentityMessage>) -> Result<String> {
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

async fn get_settings_by_key(setting_tx: Sender<SettingMessage>, key: String) -> Result<String> {
    let fn_name = "get_settings_by_key";
    let (tx, rx) = oneshot::channel();
    match setting_tx
        .send(SettingMessage::GetSettingsByKey {
            reply_to: tx,
            key: key,
        })
        .await
    {
        Ok(_) => {}
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "failed to send message - {}",
                e
            );
            bail!(NetworkingError::new(
                NetworkingErrorCodes::ChannelSendMessageError,
                format!("failed to send message - {}", e),
            ))
        }
    }

    let machine_alias = match recv_with_timeout(rx).await {
        Ok(machine_alias) => machine_alias,
        Err(err) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "failed to receive message - {}",
                err
            );
            bail!(err);
        }
    };
    Ok(machine_alias)
}

async fn get_ip_address(settings_tx: mpsc::Sender<SettingMessage>) -> Result<String> {
    let (tx, rx) = oneshot::channel();
    match settings_tx
        .send(SettingMessage::GetSettingsByKey {
            reply_to: tx,
            key: String::from("networking.ipv4.subnet"),
        })
        .await
    {
        Ok(_) => {}
        Err(e) => {
            error!(
                func = "get_ip_address",
                package = PACKAGE_NAME,
                "error sending get ip address message - {}",
                e
            );
            bail!(NetworkingError::new(
                NetworkingErrorCodes::ChannelSendMessageError,
                format!("error sending get ip address message - {}", e),
            ));
        }
    }
    let ip_address = match recv_with_timeout(rx).await {
        Ok(ip) => ip,
        Err(e) => {
            error!(
                func = "get_ip_address",
                package = PACKAGE_NAME,
                "error receiving get ip address message - {}",
                e
            );
            bail!(NetworkingError::new(
                NetworkingErrorCodes::ChannelReceiveMessageError,
                format!("error receiving get ip address message - {}", e),
            ));
        }
    };
    info!(
        func = "get_ip_address",
        package = PACKAGE_NAME,
        "get ip address request completed",
    );
    Ok(ip_address)
}

pub async fn reconnect_messaging_service(
    messaging_tx: Sender<MessagingMessage>,
    new_setting: String,
    existing_settings: HashMap<String, String>,
) -> Result<bool> {
    let fn_name = "reconnect_messaging_service";
    match existing_settings.get("networking.enabled") {
        Some(setting) => {
            if setting == &new_setting {
                info!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "networking settings are same, no need to reconnect"
                );
                return Ok(true);
            }
        }
        None => {
            info!(
                func = fn_name,
                package = PACKAGE_NAME,
                "existing networking settings not found, reconnecting"
            );
        }
    }
    let (tx, rx) = oneshot::channel();
    match messaging_tx
        .send(MessagingMessage::Reconnect { reply_to: tx })
        .await
    {
        Ok(_) => {}
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error sending reconnect message - {}",
                e
            );
            bail!(NetworkingError::new(
                NetworkingErrorCodes::ChannelSendMessageError,
                format!("error sending reconnect message - {}", e),
            ));
        }
    }
    let result = match recv_with_timeout(rx).await {
        Ok(res) => res,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error receiving reconnect message - {}",
                e
            );
            bail!(NetworkingError::new(
                NetworkingErrorCodes::ChannelReceiveMessageError,
                format!("error receiving reconnect message - {}", e),
            ));
        }
    };
    info!(
        func = fn_name,
        package = PACKAGE_NAME,
        "reconnect request completed",
    );
    Ok(result)
}

use std::collections::HashMap;

use agent_settings::AgentSettings;
use anyhow::{bail, Result};
use channel::recv_with_timeout;
use crypto::random::generate_random_alphanumeric;
use events::Event;
use futures::StreamExt;
use identity::handler::IdentityMessage;
use kv_store::KeyValueStoreClient;
use messaging::handler::MessagingMessage;
use nats_client::{
    async_nats::jetstream::consumer::{pull::Config, Consumer},
    Bytes, Message,
};
use serde::{Deserialize, Serialize};
use serde_json::{de, json};
use sha256::digest;
use tokio::sync::{broadcast, mpsc::Sender, oneshot};
use tracing::{debug, error, info, trace};

use crate::errors::{DeviceSettingError, DeviceSettingErrorCodes};
const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");
#[derive(Serialize, Deserialize, Debug)]
pub struct SettingsAckPayload {
    pub status: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AddTaskRequestPayload {
    pub key: String,
    pub value: String,
    pub created_at: String,
}
#[derive(Serialize, Deserialize, Clone)]
pub struct DeviceSettings {
    settings: AgentSettings,
}

pub async fn create_pull_consumer(
    messaging_tx: Sender<MessagingMessage>,
    identity_tx: Sender<IdentityMessage>,
) -> Result<Consumer<Config>> {
    let fn_name = "create_pull_consumer";
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
            bail!(DeviceSettingError::new(
                DeviceSettingErrorCodes::ChannelSendMessageError,
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
            bail!(DeviceSettingError::new(
                DeviceSettingErrorCodes::ChannelReceiveMessageError,
                format!("error receiving init jetstream message - {:?}", err),
                true
            ))
        }
    };
    let stream_name = "machine_settings";
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
    let (tx, rx) = oneshot::channel();
    match identity_tx
        .clone()
        .send(IdentityMessage::GetMachineId { reply_to: tx })
        .await
    {
        Ok(_) => {}
        Err(err) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error sending get machine id message - {:?}",
                err
            );
            bail!(DeviceSettingError::new(
                DeviceSettingErrorCodes::ChannelSendMessageError,
                format!(
                    "error sending get machine id message - {:?}",
                    err.to_string()
                ),
                true
            ))
        }
    }
    let machine_id = match recv_with_timeout(rx).await {
        Ok(machine_id) => machine_id,
        Err(err) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error receiving get machine id message - {:?}",
                err
            );
            bail!(DeviceSettingError::new(
                DeviceSettingErrorCodes::ChannelReceiveMessageError,
                format!("error receiving get machine id message - {:?}", err),
                true
            ))
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
    let filter_subject = format!("machine.{}.settings.kv.>", digest(machine_id.clone()));
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
    info!(func = fn_name, package = PACKAGE_NAME, "consumer created");
    Ok(consumer)
}
pub async fn sync_settings(
    consumer: Consumer<Config>,
    event_tx: broadcast::Sender<Event>,
    messaging_tx: Sender<MessagingMessage>,
) -> Result<bool> {
    let fn_name = "sync_settings";
    let key_value_store = KeyValueStoreClient::new();
    let mut messages = match consumer.fetch().messages().await {
        Ok(s) => s,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error fetching messages, error -  {:?}",
                e
            );
            bail!(DeviceSettingError::new(
                DeviceSettingErrorCodes::PullMessagesError,
                format!("pull messages error - {:?} - {}", e.kind(), e.to_string()),
                true
            ))
        }
    };
    while let Some(Ok(message)) = messages.next().await {
        let _status =
            process_message(message.clone(), messaging_tx.clone(), event_tx.clone()).await;
        // Acknowledges a message delivery
        match message.ack().await {
            Ok(res) => {
                trace!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "message Acknowledged",
                )
            }
            Err(err) => error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "message acknowledge failed {}",
                err
            ),
        };
    }
    trace!(
        func = fn_name,
        package = PACKAGE_NAME,
        "message delivery acknowledged"
    );
    match event_tx.send(Event::Settings(events::SettingEvent::Synced)) {
        Ok(_) => {
            info!(
                func = fn_name,
                package = PACKAGE_NAME,
                "settings synced event sent"
            );
        }
        Err(err) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error sending settings synced event - {:?}",
                err.to_string()
            );
            bail!(DeviceSettingError::new(
                DeviceSettingErrorCodes::ChannelSendMessageError,
                format!(
                    "error sending settings synced event - {:?}",
                    err.to_string()
                ),
                true
            ))
        }
    }
    info!(
        func = fn_name,
        package = PACKAGE_NAME,
        "settings synced successfully"
    );
    Ok(true)
}
pub async fn await_settings_message(
    consumer: Consumer<Config>,
    messaging_tx: Sender<MessagingMessage>,
    event_tx: broadcast::Sender<Event>,
) -> Result<bool> {
    let fn_name = "await_settings_message";
    let mut messages = match consumer.messages().await {
        Ok(s) => s,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error fetching messages, error -  {:?}",
                e
            );
            bail!(DeviceSettingError::new(
                DeviceSettingErrorCodes::PullMessagesError,
                format!("pull messages error - {:?} - {}", e.kind(), e.to_string()),
                true
            ))
        }
    };

    while let Some(Ok(message)) = messages.next().await {
        match process_message(message.clone(), messaging_tx.clone(), event_tx.clone()).await {
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
            Ok(_res) => trace!(
                func = fn_name,
                package = PACKAGE_NAME,
                "message Acknowledged",
            ),
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
    Ok(true)
}

pub async fn get_settings_by_key(key: String) -> Result<String> {
    debug!(
        func = "get_settings_by_key",
        package = PACKAGE_NAME,
        "getting settings by key - {:?}",
        key
    );
    let key_value_store = KeyValueStoreClient::new();
    let result = match key_value_store.get(&key) {
        Ok(s) => s,
        Err(err) => {
            error!(
                func = "get_settings_by_key",
                package = PACKAGE_NAME,
                "error getting settings by key - {:?}",
                err
            );
            bail!(err)
        }
    };

    match result {
        Some(s) => Ok(s),
        None => Ok(String::new()),
    }
}

pub async fn set_settings(
    event_tx: broadcast::Sender<Event>,
    settings: HashMap<String, String>,
) -> Result<bool> {
    debug!(
        func = "set_settings",
        package = PACKAGE_NAME,
        "set settings - {:?}",
        settings
    );
    let mut key_value_store = KeyValueStoreClient::new();
    let result = match key_value_store.set(settings.clone()) {
        Ok(s) => s,
        Err(err) => {
            error!(
                func = "set_settings",
                package = PACKAGE_NAME,
                "error setting settings - {:?}",
                err
            );
            bail!(err)
        }
    };
    // Publish event
    match event_tx.send(Event::Settings(events::SettingEvent::Updated { settings })) {
        Ok(_) => {}
        Err(err) => {
            error!(
                func = "set_settings",
                package = PACKAGE_NAME,
                "error sending settings updated event - {:?}",
                err.to_string()
            );
            bail!(DeviceSettingError::new(
                DeviceSettingErrorCodes::ChannelSendMessageError,
                format!(
                    "error sending settings updated event - {:?}",
                    err.to_string()
                ),
                true
            ))
        }
    }
    info!(
        func = "set_settings",
        package = PACKAGE_NAME,
        "settings updated"
    );
    Ok(result)
}
async fn process_message(
    message: Message,
    messaging_tx: Sender<MessagingMessage>,
    event_tx: broadcast::Sender<Event>,
) -> Result<bool> {
    let fn_name = "process_message";
    debug!(
        func = fn_name,
        package = PACKAGE_NAME,
        "processing message - {:?}",
        message
    );
    // Process mesaage
    let add_task_payload: AddTaskRequestPayload =
        match parse_message_payload(message.payload.clone()) {
            Ok(s) => {
                debug!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "message payload parsed - {:?}",
                    s
                );
                s
            }
            Err(e) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "error parsing message payload - {:?}",
                    e
                );
                bail!(e)
            }
        };
    trace!(
        func = fn_name,
        package = PACKAGE_NAME,
        "message payload parsed"
    );
    let mut settings_payload: HashMap<String, String> = HashMap::new();
    settings_payload.insert(add_task_payload.key, add_task_payload.value);
    match set_settings(event_tx.clone(), settings_payload).await {
        Ok(s) => s,
        Err(err) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error setting machine settings - {:?}",
                err
            );
            bail!(err)
        }
    };
    // Construct payload to acknowledge
    let ack_payload = SettingsAckPayload {
        status: "SYNC_COMPLETE".to_string(),
    };

    // Specify the header name you want to retrieve
    let header_name = "Ack-To";
    let header_map_values = match &message.headers {
        Some(header_map) => header_map,
        None => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "message doesn't contain any headers"
            );
            bail!(DeviceSettingError::new(
                DeviceSettingErrorCodes::MessageHeaderEmptyError,
                format!("message doesn't contain any headers"),
                false
            ))
        }
    };

    // Use the get method to retrieve the value associated with the header name
    if let Some(header_value) = header_map_values.get(header_name) {
        let (tx, _rx) = oneshot::channel();
        // Publish ack message to service
        match messaging_tx
            .send(MessagingMessage::Send {
                reply_to: tx,
                message: json!(ack_payload).to_string(),
                subject: header_value.to_string(),
            })
            .await
        {
            Ok(_) => {
                trace!(func = fn_name, package = PACKAGE_NAME, "ack message sent")
            }
            Err(err) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "error sending ack message - {:?}",
                    err
                );
                bail!(DeviceSettingError::new(
                    DeviceSettingErrorCodes::ChannelSendMessageError,
                    format!("error sending ack message - {:?}", err.to_string()),
                    true
                ))
            }
        }
    } else {
        error!(
            func = fn_name,
            package = PACKAGE_NAME,
            "ack header not found"
        );
        bail!(DeviceSettingError::new(
            DeviceSettingErrorCodes::AckHeaderNotFoundError,
            format!("ack header not found"),
            false
        ));
    }
    info!(
        func = fn_name,
        package = PACKAGE_NAME,
        "message processed successfully"
    );
    Ok(true)
}

fn parse_message_payload(payload: Bytes) -> Result<AddTaskRequestPayload> {
    debug!(
        func = "parse_message_payload",
        package = PACKAGE_NAME,
        "parsing message payload - {:?}",
        payload
    );
    let payload_value = match std::str::from_utf8(&payload) {
        Ok(s) => s,
        Err(e) => {
            error!(
                func = "parse_message_payload",
                package = PACKAGE_NAME,
                "error parsing message payload - {:?}",
                e
            );
            bail!(DeviceSettingError::new(
                DeviceSettingErrorCodes::ExtractAddTaskPayloadError,
                format!("error parsing message payload - {}", e),
                true
            ))
        }
    };
    let payload: AddTaskRequestPayload = match serde_json::from_str(payload_value) {
        Ok(s) => s,
        Err(e) => {
            error!(
                func = "parse_message_payload",
                package = PACKAGE_NAME,
                "error converting payload to AddTaskRequestPayload - {:?}",
                e
            );
            bail!(DeviceSettingError::new(
                DeviceSettingErrorCodes::ExtractAddTaskPayloadError,
                format!("error converting payload to AddTaskRequestPayload - {}", e),
                true
            ))
        }
    };
    info!(
        func = "parse_message_payload",
        package = PACKAGE_NAME,
        "payload parsed",
    );
    Ok(payload)
}

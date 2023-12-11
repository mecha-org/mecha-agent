use agent_settings::AgentSettings;
use anyhow::{bail, Result};
use events::Event;
use futures::StreamExt;
use identity::handler::IdentityMessage;
use kv_store::KeyValueStoreClient;
use messaging::handler::MessagingMessage;
use nats_client::{Bytes, Message};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha256::digest;
use tokio::sync::{mpsc::Sender, oneshot};
use tracing::{info, trace};
use tracing_opentelemetry_instrumentation_sdk::find_current_trace_id;

use crate::errors::{DeviceSettingError, DeviceSettingErrorCodes};

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

pub async fn sync_settings(
    event_tx: tokio::sync::broadcast::Sender<Event>,
    messaging_tx: Sender<MessagingMessage>,
    identity_tx: Sender<IdentityMessage>,
) -> Result<bool> {
    let trace_id = find_current_trace_id();
    info!(
        task = "start",
        target = "device_settings",
        trace_id = trace_id,
        "starting device settings service"
    );
    println!("starting device settings service");
    let (tx, rx) = oneshot::channel();
    let _ = messaging_tx
        .send(MessagingMessage::InitJetStream { reply_to: tx })
        .await;
    let jet_stream_client = match rx.await {
        Ok(cl) => match cl {
            Ok(js_client) => js_client,
            Err(err) => bail!(err),
        },
        Err(err) => bail!(err),
    };

    let stream = match jet_stream_client
        .get_stream("device_settings".to_string())
        .await
    {
        Ok(s) => s,
        Err(e) => bail!(e),
    };
    let (tx, rx) = oneshot::channel();
    let mut machine_id = String::new();
    let _ = identity_tx
        .clone()
        .send(IdentityMessage::GetMachineId { reply_to: tx })
        .await;
    match rx.await {
        Ok(machine_id_result) => {
            if machine_id_result.is_ok() {
                match machine_id_result {
                    Ok(machine_id_value) => {
                        println!("Machine ID: {}", machine_id_value);
                        machine_id = machine_id_value;
                    }
                    Err(_) => {
                        println!("Error getting machine ID");
                    }
                }
            } else {
                println!(
                    "Error getting machine ID: {:?}",
                    machine_id_result.err().unwrap()
                );
            }
        }
        Err(_) => {
            println!("Error receiving machine ID");
        }
    }

    // Create consumer
    let filter_subject = format!("device.{}.settings.kv.>", digest(machine_id.clone()));
    let consumer = match jet_stream_client
        .create_consumer(stream, filter_subject)
        .await
    {
        Ok(s) => s,
        Err(e) => bail!(e),
    };

    let key_value_store = KeyValueStoreClient::new();
    trace!(
        task = "start",
        target = "device_settings",
        trace_id = trace_id,
        "key value inserted to store, consumer waiting for messages"
    );
    //todo: confirm with sm that should we process in batch of process all
    let mut messages = match consumer.fetch().messages().await {
        Ok(s) => s,
        Err(e) => bail!(DeviceSettingError::new(
            DeviceSettingErrorCodes::PullMessagesError,
            format!("pull messages error - {:?} - {}", e.kind(), e.to_string()),
            true
        )),
    };
    while let Some(Ok(message)) = messages.next().await {
        let _status = process_message(
            message.clone(),
            key_value_store.clone(),
            messaging_tx.clone(),
        )
        .await;

        // Acknowledges a message delivery
        match message.ack().await {
            Ok(res) => println!("message Acknowledged {:?}", res),
            Err(err) => print!("ack error {:?}", err),
        };
    }
    info!(
        task = "start",
        target = "device_settings",
        trace_id = trace_id,
        "message delivery acknowledged"
    );
    let _ = event_tx.send(Event::Settings(events::SettingEvent::Synced));

    Ok(true)
}
pub async fn start_consumer(
    messaging_tx: Sender<MessagingMessage>,
    identity_tx: Sender<IdentityMessage>,
) -> Result<bool> {
    println!("starting consumer to get real time messages");
    let trace_id = find_current_trace_id();
    info!(
        task = "start",
        target = "device_settings",
        trace_id = trace_id,
        "starting device settings service"
    );
    let (tx, rx) = oneshot::channel();
    let _ = messaging_tx
        .send(MessagingMessage::InitJetStream { reply_to: tx })
        .await;
    let jet_stream_client = match rx.await {
        Ok(cl) => match cl {
            Ok(js_client) => js_client,
            Err(err) => bail!(err),
        },
        Err(err) => bail!(err),
    };

    let stream = match jet_stream_client
        .get_stream("device_settings".to_string())
        .await
    {
        Ok(s) => s,
        Err(e) => bail!(e),
    };

    let machine_id = match get_machine_id(identity_tx.clone()).await {
        Ok(s) => s,
        Err(e) => bail!(e),
    };

    // Create consumer
    let filter_subject = format!("device.{}.settings.kv.>", digest(machine_id.clone()));
    let consumer = match jet_stream_client
        .create_consumer(stream, filter_subject)
        .await
    {
        Ok(s) => s,
        Err(e) => bail!(e),
    };

    let key_value_store = KeyValueStoreClient::new();
    trace!(
        task = "start",
        target = "device_settings",
        trace_id = trace_id,
        "key value inserted to store, consumer waiting for messages"
    );
    let mut messages = match consumer.messages().await {
        Ok(s) => s,
        Err(e) => bail!(DeviceSettingError::new(
            DeviceSettingErrorCodes::PullMessagesError,
            format!("pull messages error - {:?} - {}", e.kind(), e.to_string()),
            true
        )),
    };

    // mpsc getting blocked due to while that's why spawning a new task
    tokio::spawn(async move {
        while let Some(Ok(message)) = messages.next().await {
            let _status = process_message(
                message.clone(),
                key_value_store.clone(),
                messaging_tx.clone(),
            )
            .await;

            // Acknowledges a message delivery
            match message.ack().await {
                Ok(res) => println!("message Acknowledged {:?}", res),
                Err(err) => print!("ack error {:?}", err),
            };
        }
        info!(
            task = "start",
            target = "device_settings",
            trace_id = trace_id,
            "message delivery acknowledged"
        );
    });
    Ok(true)
}

pub async fn get_settings_by_key(key: String) -> Result<String> {
    println!("key to get settings :{:?}", key);
    let key_value_store = KeyValueStoreClient::new();
    let result = match key_value_store.get(&key) {
        Ok(s) => s,
        Err(err) => bail!(err),
    };

    match result {
        Some(s) => Ok(s),
        None => Ok(String::new()),
    }
}
async fn process_message(
    message: Message,
    mut kv_store: KeyValueStoreClient,
    messaging_tx: Sender<MessagingMessage>,
) -> Result<bool> {
    let trace_id = find_current_trace_id();
    // Process mesaage
    let add_task_payload: AddTaskRequestPayload =
        match parse_message_payload(message.payload.clone()) {
            Ok(s) => s,
            Err(e) => bail!(e),
        };
    println!("new message payload {:?}", add_task_payload);
    match kv_store.set(&add_task_payload.key, &add_task_payload.value) {
        Ok(s) => s,
        Err(err) => bail!(err),
    };
    // Construct payload to acknowledge
    let ack_payload = SettingsAckPayload {
        status: "SYNC_COMPLETE".to_string(),
    };
    trace!(
        task = "start",
        target = "device_settings",
        trace_id = trace_id,
        "ack paylod constructed"
    );

    // Specify the header name you want to retrieve
    let header_name = "Ack-To";
    let header_map_values = match &message.headers {
        Some(header_map) => header_map,
        None => bail!(DeviceSettingError::new(
            DeviceSettingErrorCodes::MessageHeaderEmptyError,
            format!("Message doesn't contain any headers"),
            false
        )),
    };
    // Use the get method to retrieve the value associated with the header name
    if let Some(header_value) = header_map_values.get(header_name) {
        let (tx, _rx) = oneshot::channel();
        // Publish ack message to service
        let _ = messaging_tx.send(MessagingMessage::Send {
            reply_to: tx,
            message: header_value.to_string(),
            subject: json!(ack_payload).to_string(),
        });
    } else {
        bail!(DeviceSettingError::new(
            DeviceSettingErrorCodes::AckHeaderNotFoundError,
            format!("Ack header not found"),
            true
        ));
    }
    Ok(true)
}

async fn get_machine_id(identity_tx: Sender<IdentityMessage>) -> Result<String> {
    let (tx, rx) = oneshot::channel();
    let mut machine_id = String::new();
    let _ = identity_tx
        .send(IdentityMessage::GetMachineId { reply_to: tx })
        .await;
    match rx.await {
        Ok(machine_id_result) => {
            if machine_id_result.is_ok() {
                match machine_id_result {
                    Ok(machine_id_value) => {
                        println!("Machine ID: {}", machine_id_value);
                        machine_id = machine_id_value;
                    }
                    Err(_) => {
                        println!("Error getting machine ID");
                    }
                }
            } else {
                println!(
                    "Error getting machine ID: {:?}",
                    machine_id_result.err().unwrap()
                );
            }
        }
        Err(_) => {
            println!("Error receiving machine ID");
        }
    }
    Ok(machine_id)
}

fn parse_message_payload(payload: Bytes) -> Result<AddTaskRequestPayload> {
    let payload_value = match std::str::from_utf8(&payload) {
        Ok(s) => s,
        Err(e) => bail!(DeviceSettingError::new(
            DeviceSettingErrorCodes::ExtractAddTaskPayloadError,
            format!("Error converting payload to string - {}", e),
            true
        )),
    };
    let payload: AddTaskRequestPayload = match serde_json::from_str(payload_value) {
        Ok(s) => s,
        Err(e) => bail!(DeviceSettingError::new(
            DeviceSettingErrorCodes::ExtractAddTaskPayloadError,
            format!("Error converting payload to AddTaskRequestPayload - {}", e),
            true
        )),
    };
    Ok(payload)
}

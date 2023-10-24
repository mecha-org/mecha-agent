use anyhow::{bail, Result};
use futures::StreamExt;
use key_value_store::KeyValueStoreClient;
use messaging::service::{Messaging, MessagingScope};
use nats_client::{jetstream::JetStreamClient, Bytes};
use serde::{Deserialize, Serialize};
use serde_json::json;
use settings::AgentSettings;
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

impl DeviceSettings {
    pub fn new(settings: AgentSettings) -> Self {
        Self {
            settings: settings.clone(),
        }
    }

    pub async fn start(&self) -> Result<bool> {
        let trace_id = find_current_trace_id();
        info!(
            task = "start",
            target = "device_settings",
            trace_id = trace_id,
            "starting device settings service"
        );
        // Initiate messaging client to subscribe topic
        let mut messaging_client: Messaging = Messaging::new(MessagingScope::System, true);
        let _ = match messaging_client.connect().await {
            Ok(s) => s,
            Err(e) => bail!(e),
        };
        trace!(
            task = "start",
            target = "device_settings",
            trace_id = trace_id,
            "messaging client connected"
        );
        // Messaging client clonned to reuse
        let cloned_messaging_client = messaging_client.clone();

        let nats_client = match cloned_messaging_client.get_nats_client().await {
            Ok(s) => s,
            Err(e) => bail!(e),
        };
        // Find jetstream with nats client
        let jet_stream_client = JetStreamClient::new(nats_client.client.clone().unwrap());
        let stream = match jet_stream_client
            .get_stream("device_settings".to_string())
            .await
        {
            Ok(s) => s,
            Err(e) => bail!(e),
        };

        // Create consumer
        let consumer = match jet_stream_client.create_consumer(stream, None).await {
            Ok(s) => s,
            Err(e) => bail!(e),
        };

        let mut key_value_store = KeyValueStoreClient::new();
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
        println!("puller waiting for messages");
        while let Some(Ok(message)) = messages.next().await {
            // Process mesaage
            let add_task_payload: AddTaskRequestPayload =
                match parse_message_payload(message.payload.clone()) {
                    Ok(s) => s,
                    Err(e) => bail!(e),
                };

            match key_value_store.set(&add_task_payload.key, &add_task_payload.value) {
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
                // Publish ack message to service
                match messaging_client
                    .publish(
                        header_value.as_str(),
                        Bytes::from(json!(ack_payload).to_string()),
                    )
                    .await
                {
                    Ok(_res) => (),
                    Err(err) => bail!(err),
                }
            } else {
                bail!(DeviceSettingError::new(
                    DeviceSettingErrorCodes::AckHeaderNotFoundError,
                    format!("Ack header not found"),
                    true
                ));
            }

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
        Ok(true)
    }
    pub async fn get_settings(&self, key: String) -> Result<String> {
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

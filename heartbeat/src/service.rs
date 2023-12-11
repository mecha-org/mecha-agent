use std::time::Duration;

use agent_settings::{read_settings_yml, AgentSettings};
use anyhow::{bail, Result};
use identity::handler::IdentityMessage;
use messaging::{
    handler::MessagingMessage,
    service::{Messaging, MessagingScope},
    Bytes,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha256::digest;
use tokio::sync::{broadcast, mpsc::Sender};
use tracing_opentelemetry_instrumentation_sdk::find_current_trace_id;

#[derive(Serialize, Deserialize, Debug)]
pub struct HeartbeatPublishPayload {
    pub time: String,
    pub machine_id: String,
}
pub struct SendHeartbeatOptions {
    pub messaging_tx: Sender<MessagingMessage>,
    pub identity_tx: Sender<IdentityMessage>,
}
pub fn get_time_interval() -> u64 {
    let settings = match read_settings_yml() {
        Ok(settings) => settings,
        Err(_) => AgentSettings::default(),
    };
    settings.heartbeat.time_interval_sec
}
pub async fn send_heartbeat(heartbeat_options: SendHeartbeatOptions) -> Result<bool> {
    let trace_id = find_current_trace_id();
    tracing::info!(
        task = "start",
        trace_id = trace_id,
        "starting heartbeat service"
    );
    let (tx, rx) = tokio::sync::oneshot::channel();
    let (publish_result_tx, publish_result_rx) = tokio::sync::oneshot::channel();
    let mut machine_id = String::new();
    let _ = heartbeat_options
        .identity_tx
        .send(IdentityMessage::GetMachineId { reply_to: tx })
        .await;
    match rx.await {
        Ok(machine_id_result) => {
            if machine_id_result.is_ok() {
                match machine_id_result {
                    Ok(machine_id_value) => {
                        machine_id = machine_id_value;
                    }
                    Err(_) => {
                        bail!("Error getting machine ID");
                    }
                }
            } else {
                bail!("Error getting machine ID");
            }
        }
        Err(err) => {
            bail!("Error getting machine ID: {:?}", err);
        }
    }
    let current_utc_time = chrono::Utc::now();
    let formatted_utc_time = current_utc_time.format("%Y-%m-%dT%H:%M:%S%:z").to_string();
    let publish_payload = HeartbeatPublishPayload {
        time: formatted_utc_time,
        machine_id: machine_id.clone(),
    };
    let _ = heartbeat_options
        .messaging_tx
        .send(MessagingMessage::Send {
            reply_to: publish_result_tx,
            message: json!(publish_payload).to_string(),
            subject: format!("device.{}.heartbeat", digest(machine_id.clone())),
        })
        .await;

    match publish_result_rx.await {
        Ok(publish_result) => {
            if publish_result.is_ok() {
                match publish_result {
                    Ok(true) => {
                        println!("Heartbeat published successfully");
                        // Handle the case where the result is Ok(true)
                        // Do something when the result is true
                    }
                    Ok(false) => {
                        println!("Heartbeat not published");
                        // Handle the case where the result is Ok(false)
                        // Do something when the result is false
                    }
                    Err(err) => {
                        println!("Error publishing heartbeat: {:?}", err);
                        // Handle the case where there's an error
                        // You can use the 'err' variable to access the error details
                    }
                }
            } else {
                bail!("Error publishing heartbeat");
            }
        }
        Err(_) => {
            bail!("Error publishing heartbeat");
        }
    }
    Ok(true)
}

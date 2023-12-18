use crate::errors::{TelemetryError, TelemetryErrorCodes};
use agent_settings::{read_settings_yml, AgentSettings};
use anyhow::{bail, Result};
use identity::handler::IdentityMessage;
use messaging::handler::MessagingMessage;
use serde::{Deserialize, Serialize};
use sha256::digest;
use std::process::Command;
use tokio::sync::{mpsc::Sender, oneshot};
use tracing::info;
use tracing_opentelemetry_instrumentation_sdk::find_current_trace_id;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TelemetryResponseGeneric<T> {
    pub success: bool,
    pub status: String,
    pub status_code: i16,
    pub message: Option<String>,
    pub error_code: Option<String>,
    pub sub_errors: Option<String>,
    pub payload: T,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EncodeData {
    encoded: Vec<u8>,
    user_type: String,
    machine_id: String,
}

pub fn telemetry_init() -> Result<String> {
    let trace_id = find_current_trace_id();
    tracing::info!(trace_id, task = "telemetry_init", "init");
    let settings = match read_settings_yml() {
        Ok(v) => v,
        Err(e) => AgentSettings::default(),
    };
    if settings.telemetry.enabled {
        let r = Command::new(settings.telemetry.otel_collector.bin)
            .arg("--config")
            .arg(settings.telemetry.otel_collector.conf.clone())
            .spawn();
        match r {
            Ok(_) => {
                tracing::info!(trace_id, task = "telemetry_init", "telemetry initialized");
            }
            Err(e) => {
                tracing::error!(
                    trace_id,
                    task = "telemetry_init",
                    "telemetry initialization failed - {}",
                    e
                );
                bail!("Failed to initialize telemetry - {}", e);
            }
        };

        Ok("success".to_string())
    } else {
        bail!(TelemetryError::new(
            TelemetryErrorCodes::DataCollectionDisabled,
            format!("Telemetry data collection is diabled"),
            true
        ))
    }
}

pub async fn user_metrics(
    content: Vec<u8>,
    metrics_type: &str,
    messaging_tx: Sender<MessagingMessage>,
    identity_tx: Sender<IdentityMessage>,
) -> Result<bool> {
    let trace_id = find_current_trace_id();
    tracing::trace!(trace_id, task = "user_metrics", "init");
    let settings = match read_settings_yml() {
        Ok(v) => v,
        Err(e) => AgentSettings::default(),
    };
    let machine_id = match get_machine_id(identity_tx.clone()).await {
        Ok(v) => v,
        Err(e) => bail!(e),
    };

    // Construct message payload
    let content: String = match serde_json::to_string(&EncodeData {
        encoded: content,
        user_type: metrics_type.to_string(),
        machine_id: machine_id.clone(),
    }) {
        Ok(k) => k,
        Err(e) => bail!(TelemetryError::new(
            TelemetryErrorCodes::MetricsSerializeFailed,
            format!("Failed to serialize metrics - {}", e),
            true
        )),
    };
    let (tx, rx) = oneshot::channel();
    // Publish data on the subject
    if settings.telemetry.collect.user {
        match messaging_tx
            .send(MessagingMessage::Send {
                reply_to: tx,
                message: content.into(),
                subject: format!("machine.{}.telemetry.metrics", digest(machine_id.clone())),
            })
            .await
        {
            Ok(_) => match rx.await {
                Ok(_) => {
                    tracing::info!(
                        trace_id,
                        task = "user_metrics",
                        "user metrics sent successfully"
                    );
                    return Ok(true);
                }
                Err(e) => {
                    bail!(TelemetryError::new(
                        TelemetryErrorCodes::MessageSentFailed,
                        format!("Failed to send message - {}", e),
                        true
                    ))
                }
            },
            Err(e) => {
                bail!(TelemetryError::new(
                    TelemetryErrorCodes::MessageSentFailed,
                    format!("Failed to send message - {}", e),
                    true
                ))
            }
        }
    } else {
        bail!(TelemetryError::new(
            TelemetryErrorCodes::DataCollectionDisabled,
            format!("Telemetry data collection is disabled"),
            true
        ))
    }
}

pub async fn process_logs(
    logs_type: String,
    content: Vec<u8>,
    identity_tx: Sender<IdentityMessage>,
    messaging_tx: Sender<MessagingMessage>,
) -> Result<bool> {
    let settings = match read_settings_yml() {
        Ok(v) => v,
        Err(e) => AgentSettings::default(),
    };
    let machine_id = match get_machine_id(identity_tx.clone()).await {
        Ok(v) => v,
        Err(e) => bail!(e),
    };

    // Construct message payload
    let payload: String = match serde_json::to_string(&EncodeData {
        encoded: content,
        user_type: logs_type.to_string(),
        machine_id: machine_id.clone(),
    }) {
        Ok(k) => k,
        Err(e) => bail!(TelemetryError::new(
            TelemetryErrorCodes::LogsSeralizeFailed,
            format!("Failed to serialize logs - {}", e),
            true
        )),
    };

    if settings.telemetry.collect.user {
        let subject = format!("machine.{}.telemetry.logs", sha256::digest(machine_id));
        let (tx, rx) = oneshot::channel();
        // Publish data on the subject
        match messaging_tx
            .send(MessagingMessage::Send {
                reply_to: tx,
                message: payload.into(),
                subject: subject.clone(),
            })
            .await
        {
            Ok(_) => match rx.await {
                Ok(_) => {
                    return Ok(true);
                }
                Err(e) => {
                    bail!(TelemetryError::new(
                        TelemetryErrorCodes::MessageSentFailed,
                        format!("Failed to send message - {}", e),
                        true
                    ))
                }
            },
            Err(e) => {
                bail!(TelemetryError::new(
                    TelemetryErrorCodes::MessageSentFailed,
                    format!("Failed to send message - {}", e),
                    true
                ))
            }
        }
    } else {
        bail!(TelemetryError::new(
            TelemetryErrorCodes::DataCollectionDisabled,
            format!("Telemetry data collection is disabled"),
            true
        ))
    }
}

async fn get_machine_id(identity_tx: Sender<IdentityMessage>) -> Result<String> {
    let mut machine_id = String::new();
    let (tx, rx) = oneshot::channel();
    let _ = identity_tx
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
    Ok(machine_id)
}

pub async fn device_provision_status(identity_tx: Sender<IdentityMessage>) -> bool {
    let trace_id = find_current_trace_id();
    info!(
        task = "device_provision_status",
        trace_id = trace_id,
        "init"
    );
    let (tx, rx) = tokio::sync::oneshot::channel();
    let _ = identity_tx
        .send(IdentityMessage::GetProvisionStatus { reply_to: tx })
        .await;
    let status = match rx.await {
        Ok(provisioning_status_result) => {
            if provisioning_status_result.is_ok() {
                match provisioning_status_result {
                    Ok(provisioning_status_value) => provisioning_status_value,
                    Err(_) => false,
                }
            } else {
                false
            }
        }
        Err(_err) => false,
    };
    status
}

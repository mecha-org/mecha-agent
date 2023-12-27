use crate::errors::{TelemetryError, TelemetryErrorCodes};
use agent_settings::{read_settings_yml, AgentSettings};
use anyhow::{bail, Result};
use channel::recv_with_timeout;
use identity::handler::IdentityMessage;
use messaging::handler::MessagingMessage;
use serde::{Deserialize, Serialize};
use std::process::Command;
use tokio::sync::{mpsc::Sender, oneshot};
use tracing::{info, warn};

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
    tracing::info!(task = "telemetry_init", "init");
    let settings = match read_settings_yml() {
        Ok(v) => v,
        Err(e) => {
            warn!(
                task = "telemetry_init",
                "Failed to read settings.yml - {}", e
            );
            AgentSettings::default()
        }
    };
    if settings.telemetry.enabled {
        let r = Command::new(settings.telemetry.otel_collector.bin)
            .arg("--config")
            .arg(settings.telemetry.otel_collector.conf.clone())
            .spawn();
        match r {
            Ok(_) => {
                tracing::info!(task = "telemetry_init", "telemetry initialized");
            }
            Err(e) => {
                tracing::error!(
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
            format!("telemetry data collection is diabled"),
            true
        ))
    }
}

pub async fn process_metrics(
    content: Vec<u8>,
    metrics_type: String,
    identity_tx: Sender<IdentityMessage>,
    messaging_tx: Sender<MessagingMessage>,
) -> Result<bool> {
    tracing::trace!(task = "process metrics", "init");
    let settings = match read_settings_yml() {
        Ok(v) => v,
        Err(e) => {
            warn!(
                task = "process_metrics",
                "failed to read settings.yml - {}", e
            );
            AgentSettings::default()
        }
    };
    let machine_id = match get_machine_id(identity_tx.clone()).await {
        Ok(v) => v,
        Err(e) => bail!(e),
    };

    // Construct message payload
    let payload: String = match serde_json::to_string(&EncodeData {
        encoded: content,
        user_type: metrics_type,
        machine_id: machine_id.clone(),
    }) {
        Ok(k) => k,
        Err(e) => bail!(TelemetryError::new(
            TelemetryErrorCodes::MetricsSerializeFailed,
            format!("Failed to serialize metrics - {}", e),
            true
        )),
    };

    if settings.telemetry.collect.user {
        let subject = format!("machine.{}.telemetry.metrics", sha256::digest(machine_id));
        let (tx, rx) = oneshot::channel();
        // Publish data on the subject
        let send_output = messaging_tx
            .send(MessagingMessage::Send {
                reply_to: tx,
                message: payload.into(),
                subject: subject.clone(),
            })
            .await;
        if send_output.is_err() {
            bail!(TelemetryError::new(
                TelemetryErrorCodes::ChannelSendMessageError { num: 1001 },
                format!("Failed to send message - {}", send_output.err().unwrap()),
                true
            ))
        }
        match recv_with_timeout(rx).await {
            Ok(_) => return Ok(true),
            Err(e) => {
                bail!(TelemetryError::new(
                    TelemetryErrorCodes::ChannelReceiveMessageError { num: 1001 },
                    format!("Failed to receive message - {}", e),
                    true
                ))
            }
        }
    }
    Ok(false)
}

pub async fn process_logs(
    logs_type: String,
    content: Vec<u8>,
    identity_tx: Sender<IdentityMessage>,
    messaging_tx: Sender<MessagingMessage>,
) -> Result<bool> {
    let settings = match read_settings_yml() {
        Ok(v) => v,
        Err(e) => {
            warn!(task = "process_logs", "failed to read settings.yml - {}", e);
            AgentSettings::default()
        }
    };
    let machine_id = match get_machine_id(identity_tx.clone()).await {
        Ok(v) => v,
        Err(e) => bail!(e),
    };
    println!("PROCESSING LOGS");
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
        let send_output = messaging_tx
            .send(MessagingMessage::Send {
                reply_to: tx,
                message: payload.into(),
                subject: subject.clone(),
            })
            .await;
        if send_output.is_err() {
            bail!(TelemetryError::new(
                TelemetryErrorCodes::ChannelSendMessageError { num: 1002 },
                format!("Failed to send message - {}", send_output.err().unwrap()),
                true
            ))
        }
        match recv_with_timeout(rx).await {
            Ok(_) => return Ok(true),
            Err(e) => {
                println!("ERROR RECEIVING MESSAGE");
                bail!(TelemetryError::new(
                    TelemetryErrorCodes::ChannelReceiveMessageError { num: 1002 },
                    format!("Failed to receive message - {}", e),
                    true
                ))
            }
        }
    }
    Ok(false)
}

async fn get_machine_id(identity_tx: Sender<IdentityMessage>) -> Result<String> {
    let (tx, rx) = oneshot::channel();
    let send_output = identity_tx
        .send(IdentityMessage::GetMachineId { reply_to: tx })
        .await;
    if send_output.is_err() {
        bail!(TelemetryError::new(
            TelemetryErrorCodes::ChannelSendMessageError { num: 1000 },
            format!("get machine id error: {}", send_output.err().unwrap()),
            true
        ));
    }
    let machine_id = match recv_with_timeout(rx).await {
        Ok(machine_id) => machine_id,
        Err(err) => {
            bail!(err);
        }
    };
    Ok(machine_id)
}

pub async fn device_provision_status(identity_tx: Sender<IdentityMessage>) -> Result<bool> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let send_output = identity_tx
        .send(IdentityMessage::GetProvisionStatus { reply_to: tx })
        .await;
    if send_output.is_err() {
        bail!(TelemetryError::new(
            TelemetryErrorCodes::ChannelSendMessageError { num: 1000 },
            format!("get machine id error: {}", send_output.err().unwrap()),
            true
        ));
    }
    let status = match recv_with_timeout(rx).await {
        Ok(status) => status,
        Err(err) => {
            bail!(err);
        }
    };
    Ok(status)
}

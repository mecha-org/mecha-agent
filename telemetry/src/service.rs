use crate::errors::{TelemetryError, TelemetryErrorCodes};
use agent_settings::{read_settings_yml, AgentSettings};
use anyhow::{bail, Result};
use channel::recv_with_timeout;
use identity::handler::IdentityMessage;
use messaging::handler::MessagingMessage;
use serde::{Deserialize, Serialize};
use settings::handler::SettingMessage;
use tokio::sync::{mpsc::Sender, oneshot};
use tracing::{error, info, warn};
const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");
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
    machine_alias: String,
}

#[derive(Debug)]
pub struct TelemetryStartResponse {
    pub telemetry_process: tokio::process::Child,
}
pub fn telemetry_init() -> Result<TelemetryStartResponse> {
    let fn_name = "telemetry_init";
    let settings = match read_settings_yml() {
        Ok(v) => v,
        Err(e) => {
            warn!(
                func = fn_name,
                package = PACKAGE_NAME,
                "failed to read settings.yml - {}",
                e
            );
            AgentSettings::default()
        }
    };
    if settings.telemetry.enabled {
        let cmd = &format!(
            "{} --config {}",
            settings.telemetry.otel_collector.bin,
            settings.telemetry.otel_collector.conf.clone()
        );
        let mut parts = cmd.split_whitespace();
        let program = parts.next().unwrap();
        let args = parts.collect::<Vec<_>>();

        let mut binding = tokio::process::Command::new(program);
        let spawn_result = binding.args(&args).spawn();

        let spawn_child = match spawn_result {
            Ok(v) => v,
            Err(e) => {
                error!(
                    func = "spawn_command",
                    package = PACKAGE_NAME,
                    "failed to spawn command {}, error - {}",
                    cmd,
                    e
                );
                bail!("failed to spawn command {}, error - {}", cmd, e);
            }
        };
        info!(
            func = fn_name,
            package = PACKAGE_NAME,
            "telemetry initialized"
        );
        Ok(TelemetryStartResponse {
            telemetry_process: spawn_child,
        })
    } else {
        info!(func = fn_name, package = PACKAGE_NAME, "telemetry disabled");
        bail!(TelemetryError::new(
            TelemetryErrorCodes::DataCollectionDisabled,
            format!("telemetry data collection is disabled"),
            false
        ))
    }
}

pub async fn process_metrics(
    content: Vec<u8>,
    metrics_type: String,
    identity_tx: Sender<IdentityMessage>,
    messaging_tx: Sender<MessagingMessage>,
    settings_tx: Sender<SettingMessage>,
) -> Result<bool> {
    let fn_name = "process_metrics";
    let settings = match read_settings_yml() {
        Ok(v) => v,
        Err(e) => {
            warn!(
                func = fn_name,
                package = PACKAGE_NAME,
                "failed to read settings.yml - {}",
                e
            );
            AgentSettings::default()
        }
    };
    let machine_id = match get_machine_id(identity_tx.clone()).await {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "failed to get machine id - {}",
                e
            );
            bail!(e)
        }
    };

    let machine_alias = match get_machine_alias(settings_tx.clone()).await {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "failed to get machine alias - {}",
                e
            );
            bail!(e)
        }
    };

    // Construct message payload
    let payload: String = match serde_json::to_string(&EncodeData {
        encoded: content,
        user_type: metrics_type,
        machine_id: machine_id.clone(),
        machine_alias: machine_alias.clone(),
    }) {
        Ok(k) => k,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "failed to serialize metrics - {}",
                e
            );
            bail!(TelemetryError::new(
                TelemetryErrorCodes::MetricsSerializeFailed,
                format!("failed to serialize metrics - {}", e),
                true
            ))
        }
    };

    if settings.telemetry.collect.user {
        let subject = format!("machine.{}.telemetry.metrics", sha256::digest(machine_id));
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
            Ok(_) => {}
            Err(e) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "failed to send message - {}",
                    e
                );
                bail!(TelemetryError::new(
                    TelemetryErrorCodes::ChannelSendMessageError,
                    format!("failed to send metrics message - {}", e),
                    true
                ))
            }
        }
        match recv_with_timeout(rx).await {
            Ok(_) => return Ok(true),
            Err(e) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "failed to receive message - {}",
                    e
                );
                bail!(TelemetryError::new(
                    TelemetryErrorCodes::ChannelReceiveMessageError,
                    format!("failed to receive message - {}", e),
                    true
                ))
            }
        }
    }
    Ok(true)
}

pub async fn process_logs(
    logs_type: String,
    content: Vec<u8>,
    identity_tx: Sender<IdentityMessage>,
    messaging_tx: Sender<MessagingMessage>,
    settings_tx: Sender<SettingMessage>,
) -> Result<bool> {
    let fn_name = "process_logs";
    let settings = match read_settings_yml() {
        Ok(v) => v,
        Err(e) => {
            warn!(
                func = fn_name,
                package = PACKAGE_NAME,
                "failed to read settings.yml - {}",
                e
            );
            AgentSettings::default()
        }
    };
    let machine_id = match get_machine_id(identity_tx.clone()).await {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "failed to get machine id - {}",
                e
            );
            bail!(e)
        }
    };
    let machine_alias = match get_machine_alias(settings_tx.clone()).await {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "failed to get machine alias - {}",
                e
            );
            bail!(e)
        }
    };
    // Construct message payload
    let payload: String = match serde_json::to_string(&EncodeData {
        encoded: content,
        user_type: logs_type.to_string(),
        machine_id: machine_id.clone(),
        machine_alias: machine_alias.clone(),
    }) {
        Ok(k) => k,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "failed to serialize logs - {}",
                e
            );
            bail!(TelemetryError::new(
                TelemetryErrorCodes::LogsSeralizeFailed,
                format!("failed to serialize logs - {}", e),
                true
            ))
        }
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
            Ok(_) => {}
            Err(e) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "failed to send message - {}",
                    e
                );
                bail!(TelemetryError::new(
                    TelemetryErrorCodes::ChannelSendMessageError,
                    format!("failed to send logs message - {}", e),
                    true
                ))
            }
        }

        match recv_with_timeout(rx).await {
            Ok(_) => return Ok(true),
            Err(e) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "failed to receive message - {}",
                    e
                );
                bail!(TelemetryError::new(
                    TelemetryErrorCodes::ChannelReceiveMessageError,
                    format!("failed to receive message - {}", e),
                    true
                ))
            }
        }
    }
    Ok(true)
}

async fn get_machine_id(identity_tx: Sender<IdentityMessage>) -> Result<String> {
    let (tx, rx) = oneshot::channel();
    match identity_tx
        .send(IdentityMessage::GetMachineId { reply_to: tx })
        .await
    {
        Ok(_) => {}
        Err(e) => {
            error!(
                func = "get_machine_id",
                package = PACKAGE_NAME,
                "failed to send message - {}",
                e
            );
            bail!(TelemetryError::new(
                TelemetryErrorCodes::ChannelSendMessageError,
                format!("failed to send message - {}", e),
                true
            ))
        }
    }

    let machine_id = match recv_with_timeout(rx).await {
        Ok(machine_id) => machine_id,
        Err(err) => {
            error!(
                func = "get_machine_id",
                package = PACKAGE_NAME,
                "failed to receive message - {}",
                err
            );
            bail!(err);
        }
    };
    Ok(machine_id)
}

async fn get_machine_alias(setting_tx: Sender<SettingMessage>) -> Result<String> {
    let fn_name = "get_machine_alias";
    let (tx, rx) = oneshot::channel();
    match setting_tx
        .send(SettingMessage::GetSettingsByKey {
            reply_to: tx,
            key: "identity.machine.alias".to_string(),
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
            bail!(TelemetryError::new(
                TelemetryErrorCodes::ChannelSendMessageError,
                format!("failed to send message - {}", e),
                true
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
pub async fn device_provision_status(identity_tx: Sender<IdentityMessage>) -> Result<bool> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    match identity_tx
        .send(IdentityMessage::GetProvisionStatus { reply_to: tx })
        .await
    {
        Ok(_) => {}
        Err(e) => {
            error!(
                func = "device_provision_status",
                package = PACKAGE_NAME,
                "failed to send message - {}",
                e
            );
            bail!(TelemetryError::new(
                TelemetryErrorCodes::ChannelSendMessageError,
                format!("failed to send message - {}", e),
                true
            ))
        }
    }

    let status = match recv_with_timeout(rx).await {
        Ok(status) => status,
        Err(err) => {
            error!(
                func = "device_provision_status",
                package = PACKAGE_NAME,
                "failed to receive message - {}",
                err
            );
            bail!(err);
        }
    };
    Ok(status)
}

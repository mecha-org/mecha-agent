use crate::errors::{TelemetryError, TelemetryErrorCodes};
use anyhow::{bail, Result};
use identity::service::Identity;
use messaging::service::Messaging;
use serde::{Deserialize, Serialize};
use settings::AgentSettings;
use sha256::digest;
use std::process::Command;
use tracing::instrument;
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

#[derive(Clone, Debug)]
pub struct TelemetryService {
    pub settings: AgentSettings,
}

impl TelemetryService {
    pub fn telemetry_init(settings: AgentSettings) -> Result<String> {
        let trace_id = find_current_trace_id();
        tracing::info!(trace_id, task = "telemetry_init", "init");
        if settings.telemetry.enabled {
            let _ = Command::new(settings.telemetry.otel_collector.bin)
                .arg("--config")
                .arg(settings.telemetry.otel_collector.conf.clone())
                .spawn();

            tracing::info!(trace_id, task = "telemetry_init", "telemetry initialized");
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
        self,
        content: Vec<u8>,
        metrics_type: &str,
        messaging_client: Messaging,
    ) -> Result<String> {
        let trace_id = find_current_trace_id();
        tracing::trace!(trace_id, task = "user_metrics", "init");
        let machine_id = match get_machine_id() {
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

        // Publish data on the subject
        if self.settings.telemetry.collect.user {
            match messaging_client
                .publish(
                    &format!("device.{}.telemetry.metrics", digest(machine_id)),
                    content.into(),
                )
                .await
            {
                Ok(_) => {
                    println!("successfully sent metrics");
                    return Ok("Success".to_string());
                }
                Err(e) => {
                    bail!(TelemetryError::new(
                        TelemetryErrorCodes::MessageSentFailed,
                        format!("Failed to send message metrics - {}", e),
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

    pub async fn user_logs(
        self,
        content: Vec<u8>,
        logs_type: &str,
        messaging_client: Messaging,
    ) -> Result<String> {
        let trace_id = find_current_trace_id();
        tracing::trace!(trace_id, task = "user_logs", "init");

        let machine_id = match get_machine_id() {
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

        if self.settings.telemetry.collect.user {
            let subject = format!("device.{}.telemetry.logs", sha256::digest(machine_id));
            match messaging_client.publish(&subject, payload.into()).await {
                Ok(_) => {
                    tracing::info!(trace_id, task = "user_logs", "user logs sent successfully");
                    println!("logs sent successfully");
                    return Ok("Success".to_string());
                }
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

    pub async fn user_trace(
        self,
        content: Vec<u8>,
        trace_type: &str,
        messaging_client: Messaging,
    ) -> Result<String> {
        let trace_id = find_current_trace_id();
        tracing::trace!(trace_id, task = "user_trace", "init");
        let machine_id = match get_machine_id() {
            Ok(v) => v,
            Err(e) => bail!(e),
        };

        // Construct message payload
        let payload: String = match serde_json::to_string(&EncodeData {
            encoded: content,
            user_type: trace_type.to_string(),
            machine_id: machine_id.clone(),
        }) {
            Ok(k) => k,
            Err(e) => bail!(TelemetryError::new(
                TelemetryErrorCodes::TraceSeralizeFailed,
                format!("Failed to serialize trace - {}", e),
                true
            )),
        };
        if self.settings.telemetry.collect.user {
            match messaging_client
                .publish(
                    &format!("device.{}.telemetry.trace", machine_id),
                    payload.into(),
                )
                .await
            {
                Ok(_) => {
                    return Ok("Success".to_string());
                }
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

    pub async fn new(settings: AgentSettings) -> Self {
        Self { settings: settings }
    }
}

fn get_machine_id() -> Result<String> {
    let identity_client = Identity::new(AgentSettings::default());
    let machine_id = match identity_client.get_machine_id() {
        Ok(v) => v,
        Err(e) => bail!(e),
    };
    Ok(machine_id)
}

use crate::errors::{TelemetryError, TelemetryErrorCodes};
use anyhow::{bail, Result};
use messaging::{
    service::{Messaging, MessagingScope},
    Bytes,
};
use serde::{Deserialize, Serialize};
use settings::{telemetry::TelemetrySettings, AgentSettings};
use std::process::Command;
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

#[derive(Clone)]
pub struct TelemetryService {
    pub settings: TelemetrySettings,
    pub messaging_client: Messaging,
}

impl TelemetryService {
    pub fn telemetry_init(self) -> Result<String> {
        let trace_id = find_current_trace_id();
        tracing::trace!(trace_id, task = "Telemetry", "init");
        if self.settings.enabled {
            let _ = Command::new(self.settings.otel_collector.bin)
                .arg("--config")
                .arg(self.settings.otel_collector.conf)
                .spawn();
            Ok("success".to_string())
        } else {
            bail!(TelemetryError::new(
                TelemetryErrorCodes::DataCollectionDisabled,
                format!("Telemetry data collection is diabled"),
                true
            ))
        }
    }

    pub async fn start_telemetry(self) {
        // let _ = self.clone().messaging_init().await;
        let _ = self.clone().telemetry_init();
    }

    pub async fn user_metrics(self, content: Bytes) -> Result<String> {
        let trace_id = find_current_trace_id();
        tracing::trace!(trace_id, task = "user_metrics", "init");
        if self.settings.collect.user {
            match self
                .messaging_client
                .publish("devices.12.telemetry.metrics", content)
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

    pub async fn user_logs(self, content: Bytes) -> Result<String> {
        let trace_id = find_current_trace_id();
        tracing::trace!(trace_id, task = "user_logs", "init");
        if self.settings.collect.user {
            match self.messaging_client.publish("device1", content).await {
                Ok(_) => {
                    tracing::info!(trace_id, task = "user_logs", "User logs sent successfully");
                    println!("successfully sent logs");
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

    pub async fn user_trace(self, content: Bytes) -> Result<String> {
        let trace_id = find_current_trace_id();
        tracing::trace!(trace_id, task = "user_trace", "init");
        if self.settings.collect.user {
            match self
                .messaging_client
                .publish("devices.12.telemetry.trace", content)
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

    pub async fn new(settings: TelemetrySettings) -> Self {
        let messaging_settings = match settings::read_settings_yml() {
            Ok(v) => v.messaging,
            Err(e) => AgentSettings::default().messaging,
        };

        let mut messaging_client: Messaging =
            Messaging::new(MessagingScope::System, messaging_settings.system.enabled);
        let e = match messaging_client.connect().await {
            Ok(s) => Ok(s),
            Err(e) => Err(false), // TODO: dont stop the agent but add re-connection with exponential backoff
        };
        println!("client{:?}", e);
        Self {
            settings: settings,
            messaging_client: messaging_client,
        }
    }
}

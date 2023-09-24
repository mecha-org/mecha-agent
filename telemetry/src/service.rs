
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use settings::telemetry::TelemetrySettings;
use tracing_opentelemetry_instrumentation_sdk::find_current_trace_id;
use std::process::Command;

use crate::errors::{TelemetryError, TelemetryErrorCodes};

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
    settings: TelemetrySettings,
    // messaging_service: MessagingService,
}

impl TelemetryService {
    pub fn init(self) -> Result<String> {
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

    pub fn user_metrics(&self, content: String) -> Result<String> {
        let trace_id = find_current_trace_id();
        tracing::trace!(trace_id, task = "user_metrics", "init");
        if self.settings.collect.user {
            // match self.messaging_service
            //     .sendMessage("/telemetry/metrics".to_string(), content) {
            //         Ok(e) => {
            //             tracing::info!(
            //                 trace_id,
            //                 task = "user_metrics",
            //                 "User Metrics sent successfully"
            //             );
            //         },
            //         Err(e) =>{
            //             bail!(TelemetryError::new(
            //                 TelemetryErrorCodes::MessageSentFailed,
            //                 format!("Failed to send message - {}", e),
            //                 true
            //             ))
            //         }
            //     }
            Ok("Success".to_string())
        } else {
            bail!(TelemetryError::new(
                TelemetryErrorCodes::DataCollectionDisabled,
                format!("Telemetry data collection is disabled"),
                true
            ))
        }
    }

    pub fn user_logs(&self, content: String) -> Result<String> {
        let trace_id = find_current_trace_id();
        tracing::trace!(trace_id, task = "user_logs", "init");
        if self.settings.collect.user {
            // match self.messaging_service
            //     .sendMessage("/telemetry/logs".to_string(), content) {
            //         Ok(e) => {
            //             tracing::info!(
            //                 trace_id,
            //                 task = "user_logs",
            //                 "User logs sent successfully"
            //             );
            //         },
            //         Err(e) =>{
            //             bail!(TelemetryError::new(
            //                 TelemetryErrorCodes::MessageSentFailed,
            //                 format!("Failed to send message - {}", e),
            //                 true
            //             ))
            //         }
            //     }
            Ok("Success".to_string())
        } else {
            bail!(TelemetryError::new(
                TelemetryErrorCodes::DataCollectionDisabled,
                format!("Telemetry data collection is disabled"),
                true
            ))
        }
    }

    pub fn user_trace(&self, content: String) -> Result<String> {
        let trace_id = find_current_trace_id();
        tracing::trace!(trace_id, task = "user_trace", "init");
        if self.settings.collect.user {
            // match self.messaging_service
            //     .sendMessage("/telemetry/trace".to_string(), content) {
            //         Ok(e) => {
            //             tracing::info!(
            //                 trace_id,
            //                 task = "user_trace",
            //                 "User trace sent successfully"
            //             );
            //         },
            //         Err(e) =>{
            //             bail!(TelemetryError::new(
            //                 TelemetryErrorCodes::MessageSentFailed,
            //                 format!("Failed to send message - {}", e),
            //                 true
            //             ))
            //         }
            //     }
            Ok("Success".to_string())
        } else {
            bail!(TelemetryError::new(
                TelemetryErrorCodes::DataCollectionDisabled,
                format!("Telemetry data collection is disabled"),
                true
            ))
        }
    }
    
    pub fn new(settings: TelemetrySettings) -> Self {
        Self {
            settings: settings,
            // messaging_service: messaging_service,
        }
    }
}

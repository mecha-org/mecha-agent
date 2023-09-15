use crate::TelemetrySettings;
use anyhow::Result;
use axum::http::StatusCode;
use dotenv::dotenv;
use messaging::service::MessagingService;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::process::Command;
use tonic::Code;

#[derive(Debug)]
pub struct TelemetryErrorResponseCode {
    pub code: Code,
    pub message: String,
}
pub struct ServerError {
    pub code: StatusCode,
    pub message: String,
}

#[derive(Debug, Default, Clone, Copy)]
pub enum TelemetryErrorCodes {
    #[default]
    ManifestationNotFound,
    CertificateGenerationFailed,
}
impl std::fmt::Display for TelemetryErrorCodes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TelemetryErrorCodes::ManifestationNotFound => {
                write!(f, "Manifestation not found error")
            }
            TelemetryErrorCodes::CertificateGenerationFailed => {
                write!(f, "Certification generation failed")
            }
        }
    }
}
#[derive(Serialize, Deserialize, Debug)]
pub struct EventDetails {
    pub event: String,
    #[serde(rename = "clientid")]
    pub client_id: String,
}
#[derive(Debug)]
pub struct TelemetryServiceError {
    pub code: TelemetryErrorCodes,
    pub message: String,
}

impl std::fmt::Display for TelemetryServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(code: {:?}, message: {})", self.code, self.message)
    }
}

impl TelemetryServiceError {
    pub fn new(code: TelemetryErrorCodes, message: String, capture_error: bool) -> Self {
        Self {
            code,
            message: message,
        }
    }
}

#[derive(Clone)]
pub struct TelemetryService {
    settings: TelemetrySettings,
    messaging_service: MessagingService,
}

impl TelemetryService {
    pub fn init(self) {
        // let setting_new = self.settings.clone();
        dotenv().ok();
        if self.settings.enabled {
            let _ = Command::new(self.settings.otel_collector.bin)
                .arg("--config")
                .arg(self.settings.otel_collector.conf)
                .spawn();
        }
    }

    pub fn user_metrics(&self, content: String) -> Result<String, TelemetryErrorResponseCode> {
        if self.settings.collect.user {
            println!("User data");
            self.messaging_service
                .sendMessage("/telemetry/metrics".to_string(), content);
            Ok("Success".to_string())
        } else {
            return Err(TelemetryErrorResponseCode {
                code: Code::InvalidArgument,
                message: format!("Error Manifest not Found"),
            });
        }
    }
    pub fn system_metrics(&self, content: String) -> Result<String, TelemetryErrorResponseCode> {
        if self.settings.collect.system {
            println!("system data");
            self.messaging_service
                .sendMessage("/telemetry/metrics".to_string(), content);
            Ok("Success".to_string())
        } else {
            return Err(TelemetryErrorResponseCode {
                code: Code::InvalidArgument,
                message: format!("Error Manifest not Found"),
            });
        }
    }

    pub fn user_logs(&self, content: String) -> Result<String, TelemetryErrorResponseCode> {
        if self.settings.collect.user {
            println!("User data");
            self.messaging_service
                .sendMessage("/telemetry/logs".to_string(), content);
            Ok("Success".to_string())
        } else {
            return Err(TelemetryErrorResponseCode {
                code: Code::InvalidArgument,
                message: format!("Error Manifest not Found"),
            });
        }
    }
    pub fn system_logs(&self, content: String) -> Result<String, TelemetryErrorResponseCode> {
        if self.settings.collect.system {
            self.messaging_service
                .sendMessage("/telemetry/logs".to_string(), content);
            Ok("Success".to_string())
        } else {
            return Err(TelemetryErrorResponseCode {
                code: Code::InvalidArgument,
                message: format!("Error Manifest not Found"),
            });
        }
    }

    pub fn user_trace(&self, content: String) -> Result<String, TelemetryErrorResponseCode> {
        if self.settings.collect.user {
            println!("User data");
            self.messaging_service
                .sendMessage("/telemetry/trace".to_string(), content);
            Ok("Success".to_string())
        } else {
            return Err(TelemetryErrorResponseCode {
                code: Code::InvalidArgument,
                message: format!("Error Manifest not Found"),
            });
        }
    }
    pub fn system_trace(&self, content: String) -> Result<String, TelemetryErrorResponseCode> {
        if self.settings.collect.system {
            println!("system data");
            self.messaging_service
                .sendMessage("/telemetry/trace".to_string(), content);
            Ok("Success".to_string())
        } else {
            return Err(TelemetryErrorResponseCode {
                code: Code::InvalidArgument,
                message: format!("Error Manifest not Found"),
            });
        }
    }
    pub fn new(settings: TelemetrySettings, messaging_service: MessagingService) -> Self {
        Self {
            settings: settings,
            messaging_service: messaging_service,
        }
    }
}

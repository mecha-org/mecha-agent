use std::fmt;
use sentry_anyhow::capture_anyhow;
use tracing::error;
use tracing_opentelemetry_instrumentation_sdk::find_current_trace_id;

#[derive(Debug, Default, Clone, Copy)]
pub enum TelemetryErrorCodes {
    #[default]
    DataCollectionDisabled,
    MessageSentFailed,
    
}

impl fmt::Display for TelemetryErrorCodes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TelemetryErrorCodes::DataCollectionDisabled => write!(f, "TelemetryErrorCodes: DataCollectionDisabled"),
            TelemetryErrorCodes::MessageSentFailed => write!(f, "TelemetryErrorCodes: MessageSentFailed"),
        }
    }
}

#[derive(Debug)]
pub struct TelemetryError {
    pub code: TelemetryErrorCodes,
    pub message: String,
}

impl std::fmt::Display for TelemetryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TelemetryErrorCodes:(code: {:?}, message: {})", self.code, self.message)
    }
}

impl TelemetryError {
    pub fn new(code: TelemetryErrorCodes, message: String, capture_error: bool) -> Self {
        let trace_id = find_current_trace_id();
        error!(
            target = "Telemetry",
            "error: (code: {:?}, message: {})", code, message
        );
        if capture_error {
            let error = &anyhow::anyhow!(code).context(format!(
                "error: (code: {:?}, message: {} trace:{:?})",
                code, message, trace_id
            ));
            capture_anyhow(error);
        }
        Self { code, message }
    }
}


use sentry_anyhow::capture_anyhow;
use std::fmt;
use tracing::error;
use tracing_opentelemetry_instrumentation_sdk::find_current_trace_id;

#[derive(Debug, Default, Clone, Copy)]
pub enum HeatbeatErrorCodes {
    #[default]
    UnknownError,
    InitMessagingClientError,
}

impl fmt::Display for HeatbeatErrorCodes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            HeatbeatErrorCodes::UnknownError => write!(f, "HeatbeatErrorCodes: UnknownError"),
            HeatbeatErrorCodes::InitMessagingClientError => {
                write!(f, "HeatbeatErrorCodes: InitMessagingClientError")
            }
        }
    }
}

#[derive(Debug)]
pub struct HeatbeatError {
    pub code: HeatbeatErrorCodes,
    pub message: String,
}

impl std::fmt::Display for HeatbeatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "HeatbeatErrorCodes:(code: {:?}, message: {})",
            self.code, self.message
        )
    }
}

impl HeatbeatError {
    pub fn new(code: HeatbeatErrorCodes, message: String, capture_error: bool) -> Self {
        let trace_id = find_current_trace_id();
        error!(
            target = "heartbeat",
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

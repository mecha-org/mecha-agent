use std::fmt;
use sentry_anyhow::capture_anyhow;
use tracing::error;
use tracing_opentelemetry_instrumentation_sdk::find_current_trace_id;

#[derive(Debug, Default, Clone, Copy)]
pub enum AgentServerErrorCodes {
    #[default]
    UnknownError,
    InitGRPCServerError,
    InitMessagingClientError
}

impl fmt::Display for AgentServerErrorCodes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AgentServerErrorCodes::UnknownError => write!(f, "AgentServerErrorCodes: UnknownError"),
            AgentServerErrorCodes::InitGRPCServerError => write!(f, "AgentServerErrorCodes: InitGRPCServerError"),
            AgentServerErrorCodes::InitMessagingClientError => write!(f, "AgentServerErrorCodes: InitMessagingClientError"),
        }
    }
}

#[derive(Debug)]
pub struct AgentServerError {
    pub code: AgentServerErrorCodes,
    pub message: String,
}

impl std::fmt::Display for AgentServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AgentServerErrorCodes:(code: {:?}, message: {})", self.code, self.message)
    }
}

impl AgentServerError {
    pub fn new(code: AgentServerErrorCodes, message: String, capture_error: bool) -> Self {
        let trace_id = find_current_trace_id();
        error!(
            target = "agent_server",
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

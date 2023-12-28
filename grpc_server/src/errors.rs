use sentry_anyhow::capture_anyhow;
use std::fmt;

#[derive(Debug, Default, Clone, Copy)]
pub enum AgentServerErrorCodes {
    #[default]
    UnknownError,
    InitGRPCServerError,
    InitMessagingClientError,
}

impl fmt::Display for AgentServerErrorCodes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AgentServerErrorCodes::UnknownError => write!(f, "AgentServerErrorCodes: UnknownError"),
            AgentServerErrorCodes::InitGRPCServerError => {
                write!(f, "AgentServerErrorCodes: InitGRPCServerError")
            }
            AgentServerErrorCodes::InitMessagingClientError => {
                write!(f, "AgentServerErrorCodes: InitMessagingClientError")
            }
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
        write!(
            f,
            "AgentServerErrorCodes:(code: {:?}, message: {})",
            self.code, self.message
        )
    }
}

impl AgentServerError {
    pub fn new(code: AgentServerErrorCodes, message: String, capture_error: bool) -> Self {
        if capture_error {
            let error = &anyhow::anyhow!(code)
                .context(format!("error: (code: {:?}, message: {})", code, message));
            capture_anyhow(error);
        }
        Self { code, message }
    }
}

use provisioning::errors::ProvisioningError;
use regex::Regex;
use sentry_anyhow::capture_anyhow;
use std::fmt;
use tonic::{Code, Status};
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

pub fn resolve_tonic_error_code(code: &str) -> Code {
    match code.to_uppercase().as_str() {
        "OK" => Code::Ok,
        "CANCELLED" => Code::Cancelled,
        "UNKNOWN" => Code::Unknown,
        "INVALID_ARGUMENT" => Code::InvalidArgument,
        "DEADLINE_EXCEEDED" => Code::DeadlineExceeded,
        "NOT_FOUND" => Code::NotFound,
        "ABORTED" => Code::Aborted,
        "UNIMPLEMENTED" => Code::Unimplemented,
        "INTERNAL" => Code::Internal,
        "UNAVAILABLE" => Code::Unavailable,
        "UNAUTHENTICATED" => Code::Unauthenticated,
        _ => Code::Unknown,
    }
}
pub fn resolve_error(error: ProvisioningError) -> Status {
    let re = Regex::new(r#"status: (\w+),"#).unwrap();
    let caps = re.captures(&error.message).unwrap();
    let code = caps.get(1).unwrap().as_str();
    let code = resolve_tonic_error_code(code);
    Status::new(code, error.to_string())
}

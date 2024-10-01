use std::fmt;

#[derive(Debug, Default, Clone, Copy)]
pub enum AgentErrorCodes {
    #[default]
    UnknownError,
    ProvisioningInitError,
    IdentityInitError,
    MessagingInitError,
    StatusInitError,
    SettingsInitError,
    NetworkingInitError,
    TelemetryInitError,
    AppServiceInitError,
    GlobalSubscriberInitError,
    InitGRPCError,
    ChannelReceiveMessageError,
    InitLoggerError,
}

impl fmt::Display for AgentErrorCodes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AgentErrorCodes::UnknownError => write!(f, "UnknownError"),
            AgentErrorCodes::ProvisioningInitError => write!(f, "ProvisioningInitError"),
            AgentErrorCodes::IdentityInitError => write!(f, "IdentityInitError"),
            AgentErrorCodes::MessagingInitError => write!(f, "MessagingInitError"),
            AgentErrorCodes::StatusInitError => write!(f, "StatusInitError"),
            AgentErrorCodes::SettingsInitError => write!(f, "SettingsInitError"),
            AgentErrorCodes::NetworkingInitError => write!(f, "NetworkingInitError"),
            AgentErrorCodes::TelemetryInitError => write!(f, "TelemetryInitError"),
            AgentErrorCodes::InitGRPCError => write!(f, "InitGRPCError"),
            AgentErrorCodes::GlobalSubscriberInitError => write!(f, "GlobalSubscriberInitError"),
            AgentErrorCodes::ChannelReceiveMessageError => write!(f, "ChannelReceiveMessageError"),
            AgentErrorCodes::AppServiceInitError => write!(f, "AppServiceInitError"),
            AgentErrorCodes::InitLoggerError => write!(f, "InitLoggerError"),
        }
    }
}

#[derive(Debug)]
pub struct AgentError {
    pub code: AgentErrorCodes,
    pub message: String,
}

impl std::fmt::Display for AgentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "AgentErrorCodes:(code: {:?}, message: {})",
            self.code, self.message
        )
    }
}

impl AgentError {
    pub fn new(code: AgentErrorCodes, message: String) -> Self {
        Self { code, message }
    }
}

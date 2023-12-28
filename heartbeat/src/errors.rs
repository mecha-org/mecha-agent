use sentry_anyhow::capture_anyhow;
use std::fmt;

#[derive(Debug, Default, Clone, Copy)]
pub enum HeartbeatErrorCodes {
    #[default]
    UnknownError,
    InitMessagingClientError,
    ChannelSendMessageError,
    ChannelRecvTimeoutError,
}

impl fmt::Display for HeartbeatErrorCodes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            HeartbeatErrorCodes::UnknownError => write!(f, "HeartbeatErrorCodes: UnknownError"),
            HeartbeatErrorCodes::InitMessagingClientError => {
                write!(f, "HeartbeatErrorCodes: InitMessagingClientError")
            }
            HeartbeatErrorCodes::ChannelSendMessageError => {
                write!(f, "HeartbeatErrorCodes: ChannelSendMessageError")
            }
            HeartbeatErrorCodes::ChannelRecvTimeoutError => {
                write!(f, "HeartbeatErrorCodes: ChannelRecvTimeoutError",)
            }
        }
    }
}

#[derive(Debug)]
pub struct HeartbeatError {
    pub code: HeartbeatErrorCodes,
    pub message: String,
}

impl std::fmt::Display for HeartbeatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "HeartbeatErrorCodes:(code: {:?}, message: {})",
            self.code, self.message
        )
    }
}

impl HeartbeatError {
    pub fn new(code: HeartbeatErrorCodes, message: String, capture_error: bool) -> Self {
        if capture_error {
            let error = &anyhow::anyhow!(code)
                .context(format!("error: (code: {:?}, message: {})", code, message));
            capture_anyhow(error);
        }
        Self { code, message }
    }
}

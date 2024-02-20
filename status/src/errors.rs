use sentry_anyhow::capture_anyhow;
use std::fmt;

#[derive(Debug, Default, Clone, Copy)]
pub enum StatusErrorCodes {
    #[default]
    UnknownError,
    InitMessagingClientError,
    ChannelSendMessageError,
    ChannelRecvTimeoutError,
    FetchPlatformInfoError,
    FetchMachineIdError,
    FetchLoadAverageError,
    FetchUptimeError,
}

impl fmt::Display for StatusErrorCodes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            StatusErrorCodes::UnknownError => write!(f, "StatusErrorCodes: UnknownError"),
            StatusErrorCodes::InitMessagingClientError => {
                write!(f, "StatusErrorCodes: InitMessagingClientError")
            }
            StatusErrorCodes::ChannelSendMessageError => {
                write!(f, "StatusErrorCodes: ChannelSendMessageError")
            }
            StatusErrorCodes::ChannelRecvTimeoutError => {
                write!(f, "StatusErrorCodes: ChannelRecvTimeoutError",)
            }
            StatusErrorCodes::FetchPlatformInfoError => {
                write!(f, "StatusErrorCodes: FetchPlatformInfoError",)
            }
            StatusErrorCodes::FetchMachineIdError => {
                write!(f, "StatusErrorCodes: FetchMachineIdError",)
            }
            StatusErrorCodes::FetchLoadAverageError => {
                write!(f, "StatusErrorCodes: FetchLoadAverageError",)
            }
            StatusErrorCodes::FetchUptimeError => {
                write!(f, "StatusErrorCodes: FetchUptimeError",)
            }
        }
    }
}

#[derive(Debug)]
pub struct StatusError {
    pub code: StatusErrorCodes,
    pub message: String,
}

impl std::fmt::Display for StatusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "StatusErrorCodes:(code: {:?}, message: {})",
            self.code, self.message
        )
    }
}

impl StatusError {
    pub fn new(code: StatusErrorCodes, message: String, capture_error: bool) -> Self {
        if capture_error {
            let error = &anyhow::anyhow!(code)
                .context(format!("error: (code: {:?}, message: {})", code, message));
            capture_anyhow(error);
        }
        Self { code, message }
    }
}

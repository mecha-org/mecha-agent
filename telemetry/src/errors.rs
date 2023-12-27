use sentry_anyhow::capture_anyhow;
use std::fmt;
use tracing::error;

#[derive(Debug, Default, Clone, Copy)]
pub enum TelemetryErrorCodes {
    #[default]
    DataCollectionDisabled,
    MessageSentFailed,
    InitMessagingClientError,
    MetricsSerializeFailed,
    LogsSeralizeFailed,
    TraceSeralizeFailed,
    ChannelSendMessageError {
        num: u32,
    },
    ChannelReceiveMessageError {
        num: u32,
    },
}

impl fmt::Display for TelemetryErrorCodes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TelemetryErrorCodes::DataCollectionDisabled => {
                write!(f, "TelemetryErrorCodes: DataCollectionDisabled")
            }
            TelemetryErrorCodes::MessageSentFailed => {
                write!(f, "TelemetryErrorCodes: MessageSentFailed")
            }
            TelemetryErrorCodes::InitMessagingClientError => {
                write!(f, "TelemetryErrorCodes: InitMessagingClientError")
            }
            TelemetryErrorCodes::MetricsSerializeFailed => {
                write!(f, "TelemetryErrorCodes: MetricsSerializeFailed")
            }
            TelemetryErrorCodes::LogsSeralizeFailed => {
                write!(f, "TelemetryErrorCodes: LogsSeralizeFailed")
            }
            TelemetryErrorCodes::TraceSeralizeFailed => {
                write!(f, "TelemetryErrorCodes: TraceSeralizeFailed")
            }
            TelemetryErrorCodes::ChannelSendMessageError { num } => {
                write!(f, "TelemetryErrorCodes: ChannelSendMessageError({})", num)
            }
            TelemetryErrorCodes::ChannelReceiveMessageError { num } => {
                write!(
                    f,
                    "TelemetryErrorCodes: ChannelReceiveMessageError({})",
                    num
                )
            }
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
        write!(
            f,
            "TelemetryErrorCodes:(code: {:?}, message: {})",
            self.code, self.message
        )
    }
}

impl TelemetryError {
    pub fn new(code: TelemetryErrorCodes, message: String, capture_error: bool) -> Self {
        error!(
            target = "Telemetry",
            "error: (code: {:?}, message: {})", code, message
        );
        if capture_error {
            let error = &anyhow::anyhow!(code)
                .context(format!("error: (code: {:?}, message: {})", code, message));
            capture_anyhow(error);
        }
        Self { code, message }
    }
}

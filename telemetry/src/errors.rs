use std::fmt;

#[derive(Debug, Default, Clone, Copy)]
pub enum TelemetryErrorCodes {
    #[default]
    DataCollectionDisabled,
    MessageSentFailed,
    InitMessagingClientError,
    MetricsSerializeFailed,
    LogsSeralizeFailed,
    TraceSeralizeFailed,
    ChannelSendMessageError,
    ChannelReceiveMessageError,
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
            TelemetryErrorCodes::ChannelSendMessageError => {
                write!(f, "TelemetryErrorCodes: ChannelSendMessageError")
            }
            TelemetryErrorCodes::ChannelReceiveMessageError => {
                write!(f, "TelemetryErrorCodes: ChannelReceiveMessageError")
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
    pub fn new(code: TelemetryErrorCodes, message: String) -> Self {
        Self { code, message }
    }
}

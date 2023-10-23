use sentry_anyhow::capture_anyhow;
use std::fmt;
use tracing::error;
use tracing_opentelemetry_instrumentation_sdk::find_current_trace_id;

#[derive(Debug, Default, Clone, Copy)]
pub enum DeviceSettingErrorCodes {
    #[default]
    UnknownError,
    ExtractAddTaskPayloadError,
    MessageHeaderEmptyError,
    AckHeaderNotFoundError,
    PullMessagesError,
}

impl fmt::Display for DeviceSettingErrorCodes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DeviceSettingErrorCodes::UnknownError => {
                write!(f, "DeviceSettingErrorCodes: UnknownError")
            }
            DeviceSettingErrorCodes::ExtractAddTaskPayloadError => {
                write!(f, "DeviceSettingErrorCodes: ExtractAddTaskPayloadError")
            }
            DeviceSettingErrorCodes::AckHeaderNotFoundError => {
                write!(f, "DeviceSettingErrorCodes: AckHeaderNotFoundError")
            }
            DeviceSettingErrorCodes::MessageHeaderEmptyError => {
                write!(f, "DeviceSettingErrorCodes: MessageHeaderEmptyError")
            }
            DeviceSettingErrorCodes::PullMessagesError => {
                write!(f, "DeviceSettingErrorCodes: PullMessagesError")
            }
        }
    }
}

#[derive(Debug)]
pub struct DeviceSettingError {
    pub code: DeviceSettingErrorCodes,
    pub message: String,
}

impl std::fmt::Display for DeviceSettingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DeviceSettingErrorCodes:(code: {:?}, message: {})",
            self.code, self.message
        )
    }
}

impl DeviceSettingError {
    pub fn new(code: DeviceSettingErrorCodes, message: String, capture_error: bool) -> Self {
        let trace_id = find_current_trace_id();
        error!(
            target = "nats_client",
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

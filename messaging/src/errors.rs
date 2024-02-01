use sentry_anyhow::capture_anyhow;
use std::fmt;

#[derive(Debug, Default, Clone, Copy)]
pub enum MessagingErrorCodes {
    #[default]
    UnknownError,
    GetAuthNonceError,
    NatsClientNotInitialized,
    AuthNonceResponseParseError,
    AuthTokenResponseParseError,
    ChannelSendMessageError,
    ChannelReceiveMessageError,
    EventSendError,
}

impl fmt::Display for MessagingErrorCodes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MessagingErrorCodes::UnknownError => write!(f, "MessagingErrorCodes: UnknownError"),
            MessagingErrorCodes::GetAuthNonceError => {
                write!(f, "MessagingErrorCodes: GetAuthNonceError")
            }
            MessagingErrorCodes::NatsClientNotInitialized => {
                write!(f, "MessagingErrorCodes: NatsClientNotInitialized")
            }
            MessagingErrorCodes::AuthNonceResponseParseError => {
                write!(f, "MessagingErrorCodes: AuthNonceResponseParseError")
            }
            MessagingErrorCodes::AuthTokenResponseParseError => {
                write!(f, "MessagingErrorCodes: AuthTokenResponseParseError")
            }
            MessagingErrorCodes::ChannelSendMessageError => {
                write!(f, "MessagingErrorCodes: ChannelSendMessageError")
            }
            MessagingErrorCodes::ChannelReceiveMessageError => {
                write!(f, "MessagingErrorCodes: ChannelReceiveMessageError")
            }
            MessagingErrorCodes::EventSendError => {
                write!(f, "MessagingErrorCodes: EventSendError")
            }
        }
    }
}

#[derive(Debug)]
pub struct MessagingError {
    pub code: MessagingErrorCodes,
    pub message: String,
}

impl std::fmt::Display for MessagingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "MessagingErrorCodes:(code: {:?}, message: {})",
            self.code, self.message
        )
    }
}

impl MessagingError {
    pub fn new(code: MessagingErrorCodes, message: String, capture_error: bool) -> Self {
        if capture_error {
            let error = &anyhow::anyhow!(code)
                .context(format!("error: (code: {:?}, message: {})", code, message));
            capture_anyhow(error);
        }
        Self { code, message }
    }
}

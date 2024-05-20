use std::fmt;

#[derive(Debug, Default, Clone, Copy)]
pub enum MessagingErrorCodes {
    #[default]
    UnknownError,
    NatsClientNotInitialized,
    GetAuthNonceUnknownError,
    GetAuthNonceServerError,
    GetAuthNonceNotFoundError,
    GetAuthNonceBadRequestError,
    GetAuthTokenServerError,
    GetAuthTokenBadRequestError,
    GetAuthTokenNotFoundError,
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
            MessagingErrorCodes::NatsClientNotInitialized => {
                write!(f, "MessagingErrorCodes: NatsClientNotInitialized")
            }
            MessagingErrorCodes::GetAuthNonceUnknownError => {
                write!(f, "MessagingErrorCodes: GetAuthNonceUnknownError")
            }
            MessagingErrorCodes::GetAuthNonceServerError => {
                write!(f, "MessagingErrorCodes: GetAuthNonceServerError")
            }
            MessagingErrorCodes::GetAuthNonceNotFoundError => {
                write!(f, "MessagingErrorCodes: GetAuthNonceNotFoundError")
            }
            MessagingErrorCodes::GetAuthNonceBadRequestError => {
                write!(f, "MessagingErrorCodes: GetAuthNonceBadRequestError")
            }
            MessagingErrorCodes::GetAuthTokenServerError => {
                write!(f, "MessagingErrorCodes: GetAuthTokenServerError")
            }
            MessagingErrorCodes::GetAuthTokenBadRequestError => {
                write!(f, "MessagingErrorCodes: GetAuthTokenBadRequestError")
            }
            MessagingErrorCodes::GetAuthTokenNotFoundError => {
                write!(f, "MessagingErrorCodes: GetAuthTokenNotFoundError")
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
    pub fn new(code: MessagingErrorCodes, message: String) -> Self {
        Self { code, message }
    }
}

use std::fmt;
use sentry_anyhow::capture_anyhow;
use tracing::error;
use tracing_opentelemetry_instrumentation_sdk::find_current_trace_id;

#[derive(Debug, Default, Clone, Copy)]
pub enum MessagingErrorCodes {
    #[default]
    UnknownError,
    NatsClientNotInitialized,
    GetAuthNonceUnknownError,
    GetAuthNonceServerError,
    GetAuthNonceNotFoundError,
    GetAuthNonceBadRequestError,
    AuthNonceResponseParseError,
    AuthTokenResponseParseError,
}

impl fmt::Display for MessagingErrorCodes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MessagingErrorCodes::UnknownError => write!(f, "MessagingErrorCodes: UnknownError"),
            MessagingErrorCodes::NatsClientNotInitialized => write!(f, "MessagingErrorCodes: NatsClientNotInitialized"),
            MessagingErrorCodes::GetAuthNonceUnknownError => write!(f, "MessagingErrorCodes: GetAuthNonceUnknownError"),
            MessagingErrorCodes::GetAuthNonceServerError => write!(f, "MessagingErrorCodes: GetAuthNonceServerError"),
            MessagingErrorCodes::GetAuthNonceNotFoundError => write!(f, "MessagingErrorCodes: GetAuthNonceNotFoundError"),
            MessagingErrorCodes::GetAuthNonceBadRequestError => write!(f, "MessagingErrorCodes: GetAuthNonceBadRequestError"),
            MessagingErrorCodes::AuthNonceResponseParseError => write!(f, "MessagingErrorCodes: AuthNonceResponseParseError"),
            MessagingErrorCodes::AuthTokenResponseParseError => write!(f, "MessagingErrorCodes: AuthTokenResponseParseError"),
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
        write!(f, "MessagingErrorCodes:(code: {:?}, message: {})", self.code, self.message)
    }
}

impl MessagingError {
    pub fn new(code: MessagingErrorCodes, message: String, capture_error: bool) -> Self {
        let trace_id = find_current_trace_id();
        error!(
            target = "messaging",
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

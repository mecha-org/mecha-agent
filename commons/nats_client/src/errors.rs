use sentry_anyhow::capture_anyhow;
use std::fmt;
use tracing::error;
use tracing_opentelemetry_instrumentation_sdk::find_current_trace_id;

#[derive(Debug, Default, Clone, Copy)]
pub enum NatsClientErrorCodes {
    #[default]
    UnknownError,
    ClientConnectError,
    ClientUninitializedError,
    ClientNotConnectedError,
    ClientDisconnectedError,
    PublishError,
    SubscribeError,
    GetStreamError,
}

impl fmt::Display for NatsClientErrorCodes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            NatsClientErrorCodes::UnknownError => write!(f, "NatsClientErrorCodes: UnknownError"),
            NatsClientErrorCodes::ClientConnectError => {
                write!(f, "NatsClientErrorCodes: ClientConnectError")
            }
            NatsClientErrorCodes::ClientUninitializedError => {
                write!(f, "NatsClientErrorCodes: ClientUninitializedError")
            }
            NatsClientErrorCodes::ClientNotConnectedError => {
                write!(f, "NatsClientErrorCodes: ClientNotConnectedError")
            }
            NatsClientErrorCodes::ClientDisconnectedError => {
                write!(f, "NatsClientErrorCodes: ClientDisconnectedError")
            }
            NatsClientErrorCodes::PublishError => write!(f, "NatsClientErrorCodes: PublishError"),
            NatsClientErrorCodes::SubscribeError => {
                write!(f, "NatsClientErrorCodes: SubscribeError")
            }
            NatsClientErrorCodes::GetStreamError => {
                write!(f, "NatsClientErrorCodes: GetStreamError")
            }
        }
    }
}

#[derive(Debug)]
pub struct NatsClientError {
    pub code: NatsClientErrorCodes,
    pub message: String,
}

impl std::fmt::Display for NatsClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "NatsClientErrorCodes:(code: {:?}, message: {})",
            self.code, self.message
        )
    }
}

impl NatsClientError {
    pub fn new(code: NatsClientErrorCodes, message: String, capture_error: bool) -> Self {
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

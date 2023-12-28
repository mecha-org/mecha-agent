use sentry_anyhow::capture_anyhow;
use std::fmt;

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
    CreateConsumerError,
    RequestError,
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
            NatsClientErrorCodes::CreateConsumerError => {
                write!(f, "NatsClientErrorCodes: CreateConsumerError")
            }
            NatsClientErrorCodes::RequestError => {
                write!(f, "NatsClientErrorCodes: RequestError")
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
        if capture_error {
            let error = &anyhow::anyhow!(code)
                .context(format!("error: (code: {:?}, message: {})", code, message));
            capture_anyhow(error);
        }
        Self { code, message }
    }
}

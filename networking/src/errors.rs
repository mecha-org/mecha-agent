use sentry_anyhow::capture_anyhow;
use std::fmt;

const PACKAGE_NAME: &str = env!("CARGO_CRATE_NAME");
#[derive(Debug, Default, Clone, Copy)]
pub enum NetworkingErrorCodes {
    #[default]
    UnknownError,
    GenerateKeyPairError,
    SettingUpWireguardError,
    ChannelSendMessageError,
    ChannelReceiveMessageError,
    CreateConsumerError,
    PullMessagesError,
    ExtractMessagePayloadError,
    PayloadDeserializationError,
    MessageAcknowledgeError,
    NetworkingInitError,
    NetworkingDiscoSocketBindError,
}

impl fmt::Display for NetworkingErrorCodes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            NetworkingErrorCodes::UnknownError => {
                write!(f, "NetworkingErrorCodes: UnknownError")
            }
            NetworkingErrorCodes::GenerateKeyPairError => {
                write!(f, "NetworkingErrorCodes: GenerateKeyPairError")
            }
            NetworkingErrorCodes::SettingUpWireguardError => {
                write!(f, "NetworkingErrorCodes: SettingUpWireguardError")
            }
            NetworkingErrorCodes::ChannelSendMessageError => {
                write!(f, "NetworkingErrorCodes: ChannelSendMessageError")
            }
            NetworkingErrorCodes::ChannelReceiveMessageError => {
                write!(f, "NetworkingErrorCodes: ChannelReceiveMessageError")
            }
            NetworkingErrorCodes::CreateConsumerError => {
                write!(f, "NetworkingErrorCodes: CreateConsumerError")
            }
            NetworkingErrorCodes::PullMessagesError => {
                write!(f, "NetworkingErrorCodes: PullMessagesError")
            }
            NetworkingErrorCodes::ExtractMessagePayloadError => {
                write!(f, "NetworkingErrorCodes: ExtractMessagePayloadError")
            }
            NetworkingErrorCodes::PayloadDeserializationError => {
                write!(f, "NetworkingErrorCodes: PayloadDeserializationError")
            }
            NetworkingErrorCodes::MessageAcknowledgeError => {
                write!(f, "NetworkingErrorCodes: MessageAcknowledgeError")
            }
            NetworkingErrorCodes::NetworkingInitError => {
                write!(f, "NetworkingErrorCodes: NetworkingInitError")
            }
            NetworkingErrorCodes::NetworkingDiscoSocketBindError => {
                write!(f, "NetworkingErrorCodes: NetworkingDiscoSocketBindError")
            }
        }
    }
}

#[derive(Debug)]
pub struct NetworkingError {
    pub code: NetworkingErrorCodes,
    pub message: String,
}

impl std::fmt::Display for NetworkingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "NetworkingErrorCodes:(code: {:?}, message: {})",
            self.code, self.message
        )
    }
}

impl NetworkingError {
    pub fn new(code: NetworkingErrorCodes, message: String, capture_error: bool) -> Self {
        if capture_error {
            let error = &anyhow::anyhow!(code).context(format!(
                "error: (code: {:?}, message: {}, package: {})",
                code, message, PACKAGE_NAME
            ));
            capture_anyhow(error);
        }
        Self { code, message }
    }
}

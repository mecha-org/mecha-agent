use sentry_anyhow::capture_anyhow;
use serde::{Deserialize, Serialize};
use std::fmt;

const PACKAGE_NAME: &str = env!("CARGO_CRATE_NAME");
#[derive(Debug, Default, Clone, Copy)]
pub enum ProvisioningErrorCodes {
    #[default]
    UnknownError,
    UnauthorizedError,
    NotFoundError,
    BadRequestError,
    UnreachableError,
    InternalServerError,
    CSRSignReadFileError,
    CertificateWriteError,
    SendEventError,
    DatabaseDeleteError,
    ParseResponseError,
    ChannelSendMessageError,
    ChannelReceiveMessageError,
    MachineIdError,
    ExtractMessagePayloadError,
}

impl fmt::Display for ProvisioningErrorCodes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ProvisioningErrorCodes::UnknownError => {
                write!(f, "ProvisioningErrorCodes: UnknownError")
            }
            ProvisioningErrorCodes::UnauthorizedError => {
                write!(f, "ProvisioningErrorCodes: UnauthorizedError")
            }
            ProvisioningErrorCodes::NotFoundError => {
                write!(f, "ProvisioningErrorCodes: NotFoundError")
            }
            ProvisioningErrorCodes::BadRequestError => {
                write!(f, "ProvisioningErrorCodes: BadRequestError")
            }
            ProvisioningErrorCodes::UnreachableError => {
                write!(f, "ProvisioningErrorCodes: UnreachableError")
            }
            ProvisioningErrorCodes::InternalServerError => {
                write!(f, "ProvisioningErrorCodes: InternalServerError")
            }
            ProvisioningErrorCodes::CSRSignReadFileError => {
                write!(f, "ProvisioningErrorCodes: CSRSignReadFileError")
            }
            ProvisioningErrorCodes::CertificateWriteError => {
                write!(f, "ProvisioningErrorCodes: CertificateWriteError")
            }
            ProvisioningErrorCodes::SendEventError => {
                write!(f, "ProvisioningErrorCodes: SendEventError")
            }
            ProvisioningErrorCodes::DatabaseDeleteError => {
                write!(f, "ProvisioningErrorCodes: DatabaseDeleteError")
            }
            ProvisioningErrorCodes::ParseResponseError => {
                write!(f, "ProvisioningErrorCodes: ParseResponseError")
            }
            ProvisioningErrorCodes::ChannelSendMessageError => {
                write!(f, "ProvisioningErrorCodes: ChannelSendMessageError")
            }
            ProvisioningErrorCodes::ChannelReceiveMessageError => {
                write!(f, "ProvisioningErrorCodes: ChannelReceiveMessageError")
            }
            ProvisioningErrorCodes::MachineIdError => {
                write!(f, "ProvisioningErrorCodes: MachineIdError")
            }
            ProvisioningErrorCodes::ExtractMessagePayloadError => {
                write!(f, "ProvisioningErrorCodes: ExtractMessagePayloadError")
            }
        }
    }
}

#[derive(Debug)]
pub struct ProvisioningError {
    pub code: ProvisioningErrorCodes,
    pub message: String,
}

impl std::fmt::Display for ProvisioningError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ProvisioningErrorCodes:(code: {:?}, message: {})",
            self.code, self.message
        )
    }
}

impl ProvisioningError {
    pub fn new(code: ProvisioningErrorCodes, message: String, capture_error: bool) -> Self {
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

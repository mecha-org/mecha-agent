use std::fmt;

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
    SettingsDatabaseDeleteError,
    ParseResponseError,
    ChannelSendMessageError,
    ChannelReceiveMessageError,
    MachineMismatchError,
    ExtractMessagePayloadError,
    DeprovisioningError,
    SubscribeToNatsError,
    PayloadDeserializationError,
    InvalidMachineIdError,
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
            ProvisioningErrorCodes::SettingsDatabaseDeleteError => {
                write!(f, "ProvisioningErrorCodes: SettingsDatabaseDeleteError")
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
            ProvisioningErrorCodes::MachineMismatchError => {
                write!(f, "ProvisioningErrorCodes: MachineMismatchError")
            }
            ProvisioningErrorCodes::ExtractMessagePayloadError => {
                write!(f, "ProvisioningErrorCodes: ExtractMessagePayloadError")
            }
            ProvisioningErrorCodes::DeprovisioningError => {
                write!(f, "ProvisioningErrorCodes: DeprovisioningError")
            }
            ProvisioningErrorCodes::SubscribeToNatsError => {
                write!(f, "ProvisioningErrorCodes: SubscribeToNatsError")
            }
            ProvisioningErrorCodes::PayloadDeserializationError => {
                write!(f, "ProvisioningErrorCodes: PayloadDeserializationError")
            }
            ProvisioningErrorCodes::InvalidMachineIdError => {
                write!(f, "ProvisioningErrorCodes: InvalidMachineIdError")
            }
        }
    }
}

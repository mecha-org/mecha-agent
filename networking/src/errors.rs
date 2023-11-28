use sentry_anyhow::capture_anyhow;
use std::fmt;
use tracing::error;
use tracing_opentelemetry_instrumentation_sdk::find_current_trace_id;

#[derive(Debug, Default, Clone, Copy)]
pub enum NetworkingErrorCodes {
    #[default]
    NetworkingError,
    ExtractMessagePayloadError,
    MachineSettingsProviderNameNotFoundError,
    ProviderMetadataPayloadCreateError,
    MachineSettingsMachineIdNotFoundError,
    SystemArchNotFoundError,
    SystemOsNotFoundError,
    GetProviderConfigsError,
    MessagingClientConnectError,
    MessagingRequestProviderMetadataError,
    MessagingRequestProviderConfigError,
    ProviderDirectoryCreateError,
    ProviderDownloadError,
    ProviderFileCreateError,
    ProviderFileWriteError,
    ProviderFileExtractError,
    InvalidProviderFileType,
    ProviderBinariesSaveError,
    CertsValidateOrCreateError,
    CertsDirectoryCreateError,
    CertsGenerateError,
    MachineSettingsEnrollmentUrlFoundError,
    CertReadFileError,
    SignCertError,
    SignCertDecodeError,
    SignCertConvertStringError,
    SignCertFileCreateError,
    SignCertFileSaveError,
    CaCertDecodeError,
    CaCertConvertStringError,
    CaCertFileCreateError,
    CaCertFileSaveError,
    NebulaBaseConfigParseError,
    NebulaConfigDeSerializeError,
    NebulaConfigSerializeError,
    NebulaConfigFileCreateError,
    NebulaConfigFileGenerateError,
    NebulaStartError,
    SudoCheckFailed,
    CommandRunError,
    CertNotFoundError,
    KeyNotFoundError,
    CertValidityCheckError,
    CertExpiredError,
    CertVerificationCheckError,
    CertVerifyError,
}

impl fmt::Display for NetworkingErrorCodes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            NetworkingErrorCodes::NetworkingError => {
                write!(f, "NetworkingErrorCodes: NetworkingError")
            }
            NetworkingErrorCodes::ExtractMessagePayloadError => {
                write!(f, "NetworkingErrorCodes: ExtractMessagePayloadError")
            }
            //new
            NetworkingErrorCodes::MachineSettingsProviderNameNotFoundError => {
                write!(
                    f,
                    "NetworkingErrorCodes: MachineSettingsProviderNameNotFoundError"
                )
            }
            NetworkingErrorCodes::SystemArchNotFoundError => {
                write!(f, "NetworkingErrorCodes: SystemArchNotFoundError")
            }
            NetworkingErrorCodes::SystemOsNotFoundError => {
                write!(f, "NetworkingErrorCodes: SystemOsNotFoundError")
            }
            NetworkingErrorCodes::ProviderMetadataPayloadCreateError => {
                write!(
                    f,
                    "NetworkingErrorCodes: ProviderMetadataPayloadCreateError"
                )
            }
            NetworkingErrorCodes::MachineSettingsMachineIdNotFoundError => {
                write!(
                    f,
                    "NetworkingErrorCodes: MachineSettingsMachineIdNotFoundError"
                )
            }
            NetworkingErrorCodes::GetProviderConfigsError => {
                write!(f, "NetworkingErrorCodes: GetProviderConfigsError")
            }
            NetworkingErrorCodes::MessagingClientConnectError => {
                write!(f, "NetworkingErrorCodes: MessagingClientConnectError")
            }
            NetworkingErrorCodes::MessagingRequestProviderMetadataError => {
                write!(
                    f,
                    "NetworkingErrorCodes: MessagingRequestProviderMetadataError"
                )
            }
            NetworkingErrorCodes::MessagingRequestProviderConfigError => {
                write!(
                    f,
                    "NetworkingErrorCodes: MessagingRequestProviderConfigError"
                )
            }
            NetworkingErrorCodes::ProviderDirectoryCreateError => {
                write!(f, "NetworkingErrorCodes: ProviderDirectoryCreateError")
            }
            NetworkingErrorCodes::ProviderDownloadError => {
                write!(f, "NetworkingErrorCodes: ProviderDownloadError")
            }
            NetworkingErrorCodes::ProviderFileCreateError => {
                write!(f, "NetworkingErrorCodes: ProviderFileCreateError")
            }
            NetworkingErrorCodes::ProviderFileWriteError => {
                write!(f, "NetworkingErrorCodes: ProviderFileWriteError")
            }
            NetworkingErrorCodes::ProviderFileExtractError => {
                write!(f, "NetworkingErrorCodes: ProviderFileExtractError")
            }
            NetworkingErrorCodes::ProviderBinariesSaveError => {
                write!(f, "NetworkingErrorCodes: ProviderBinariesSaveError")
            }
            NetworkingErrorCodes::CertsValidateOrCreateError => {
                write!(f, "NetworkingErrorCodes: CertsValidateOrCreateError")
            }
            NetworkingErrorCodes::CertsDirectoryCreateError => {
                write!(f, "NetworkingErrorCodes: CertsDirectoryCreateError")
            }
            NetworkingErrorCodes::CertsGenerateError => {
                write!(f, "NetworkingErrorCodes: CertsGenerateError")
            }
            NetworkingErrorCodes::MachineSettingsEnrollmentUrlFoundError => {
                write!(
                    f,
                    "NetworkingErrorCodes: MachineSettingsEnrollmentUrlFoundError"
                )
            }
            NetworkingErrorCodes::CertReadFileError => {
                write!(f, "NetworkingErrorCodes: CertReadFileError")
            }
            NetworkingErrorCodes::SignCertError => {
                write!(f, "NetworkingErrorCodes: SignCertError")
            }
            NetworkingErrorCodes::SignCertDecodeError => {
                write!(f, "NetworkingErrorCodes: SignCertDecodeError")
            }
            NetworkingErrorCodes::SignCertConvertStringError => {
                write!(f, "NetworkingErrorCodes: SignCertConvertStringError")
            }
            NetworkingErrorCodes::SignCertFileCreateError => {
                write!(f, "NetworkingErrorCodes: SignCertFileCreateError")
            }
            NetworkingErrorCodes::SignCertFileSaveError => {
                write!(f, "NetworkingErrorCodes: SignCertFileSaveError")
            }
            NetworkingErrorCodes::NebulaBaseConfigParseError => {
                write!(f, "NetworkingErrorCodes: NebulaBaseConfigParseError")
            }
            NetworkingErrorCodes::NebulaConfigSerializeError => {
                write!(f, "NetworkingErrorCodes: NebulaConfigSerializeError")
            }
            NetworkingErrorCodes::NebulaConfigDeSerializeError => {
                write!(f, "NetworkingErrorCodes: NebulaConfigDeSerializeError")
            }
            NetworkingErrorCodes::NebulaConfigFileCreateError => {
                write!(f, "NetworkingErrorCodes: NebulaConfigFileCreateError")
            }
            NetworkingErrorCodes::NebulaConfigFileGenerateError => {
                write!(f, "NetworkingErrorCodes: NebulaConfigFileGenerateError")
            }
            NetworkingErrorCodes::NebulaStartError => {
                write!(f, "NetworkingErrorCodes: NebulaStartError")
            }
            NetworkingErrorCodes::SudoCheckFailed => {
                write!(f, "NetworkingErrorCodes: SudoCheckFailed")
            }
            NetworkingErrorCodes::CommandRunError => {
                write!(f, "NetworkingErrorCodes: CommandRunError")
            }
            NetworkingErrorCodes::CertNotFoundError => {
                write!(f, "NetworkingErrorCodes: CertNotFoundError")
            }
            NetworkingErrorCodes::CertExpiredError => {
                write!(f, "NetworkingErrorCodes: CertExpiredError")
            }
            NetworkingErrorCodes::KeyNotFoundError => {
                write!(f, "NetworkingErrorCodes: KeyNotFoundError")
            }
            NetworkingErrorCodes::CertValidityCheckError => {
                write!(f, "NetworkingErrorCodes: CertValidityCheckError")
            }
            NetworkingErrorCodes::CertVerificationCheckError => {
                write!(f, "NetworkingErrorCodes: CertVerificationCheckError")
            }
            NetworkingErrorCodes::CertVerifyError => {
                write!(f, "NetworkingErrorCodes: CertVerifyError")
            }
            NetworkingErrorCodes::InvalidProviderFileType => {
                write!(f, "NetworkingErrorCodes: InvalidProviderFileType")
            }
            NetworkingErrorCodes::CaCertDecodeError => {
                write!(f, "NetworkingErrorCodes: CaCertDecodeError")
            }
            NetworkingErrorCodes::CaCertConvertStringError => {
                write!(f, "NetworkingErrorCodes: CaCertConvertStringError")
            }
            NetworkingErrorCodes::CaCertFileCreateError => {
                write!(f, "NetworkingErrorCodes: CaCertFileCreateError")
            }
            NetworkingErrorCodes::CaCertFileSaveError => {
                write!(f, "NetworkingErrorCodes: CaCertFileSaveError")
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
        let trace_id = find_current_trace_id();
        error!(
            target = "networking",
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

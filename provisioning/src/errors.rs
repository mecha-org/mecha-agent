use sentry_anyhow::capture_anyhow;
use std::fmt;
use tracing::error;
use tracing_opentelemetry_instrumentation_sdk::find_current_trace_id;

#[derive(Debug, Default, Clone, Copy)]
pub enum ProvisioningErrorCodes {
    #[default]
    ManifestLookupUnknownError,
    ManifestLookupServerError,
    ManifestLookupNotFoundError,
    ManifestLookupBadRequestError,
    ManifestParseResponseError,
    CSRSignReadFileError,
    CSRSignUnknownError,
    CSRSignServerError,
    CSRSignNotFoundError,
    CSRSignBadRequestError,
    CSRSignResponseParseError,
    CertificateWriteError,
}

impl fmt::Display for ProvisioningErrorCodes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ProvisioningErrorCodes::ManifestLookupUnknownError => {
                write!(f, "ProvisioningErrorCodes: ManifestLookupUnknownError")
            }
            ProvisioningErrorCodes::ManifestLookupServerError => {
                write!(f, "ProvisioningErrorCodes: ManifestLookupServerError")
            }
            ProvisioningErrorCodes::ManifestLookupNotFoundError => {
                write!(f, "ProvisioningErrorCodes: ManifestLookupNotFoundError")
            }
            ProvisioningErrorCodes::ManifestLookupBadRequestError => {
                write!(f, "ProvisioningErrorCodes: ManifestLookupBadRequestError")
            }
            ProvisioningErrorCodes::ManifestParseResponseError => {
                write!(f, "ProvisioningErrorCodes: ManifestParseResponseError")
            }
            ProvisioningErrorCodes::CSRSignReadFileError => {
                write!(f, "ProvisioningErrorCodes: CSRSignReadFileError")
            }
            ProvisioningErrorCodes::CSRSignUnknownError => {
                write!(f, "ProvisioningErrorCodes: CSRSignUnknownError")
            }
            ProvisioningErrorCodes::CSRSignServerError => {
                write!(f, "ProvisioningErrorCodes: CSRSignServerError")
            }
            ProvisioningErrorCodes::CSRSignNotFoundError => {
                write!(f, "ProvisioningErrorCodes: CSRSignNotFoundError")
            }
            ProvisioningErrorCodes::CSRSignBadRequestError => {
                write!(f, "ProvisioningErrorCodes: CSRSignBadRequestError")
            }
            ProvisioningErrorCodes::CSRSignResponseParseError => {
                write!(f, "ProvisioningErrorCodes: CSRSignResponseParseError")
            }
            ProvisioningErrorCodes::CertificateWriteError => {
                write!(f, "ProvisioningErrorCodes: CertificateWriteError")
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
        let trace_id = find_current_trace_id();
        error!(
            target = "provisioning",
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

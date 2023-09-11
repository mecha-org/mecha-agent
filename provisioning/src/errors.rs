use std::fmt;
use sentry_anyhow::capture_anyhow;
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
    CryptoGeneratePrivateKeyError,
    CryptoGenerateCSRError,
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
            ProvisioningErrorCodes::ManifestLookupUnknownError => write!(f, "ManifestLookupUnknownError"),
            ProvisioningErrorCodes::ManifestLookupServerError => write!(f, "ManifestLookupServerError"),
            ProvisioningErrorCodes::ManifestLookupNotFoundError => write!(f, "ManifestLookupNotFoundError"),
            ProvisioningErrorCodes::ManifestLookupBadRequestError => write!(f, "ManifestLookupBadRequestError"),
            ProvisioningErrorCodes::ManifestParseResponseError => write!(f, "ManifestParseResponseError"),
            ProvisioningErrorCodes::CryptoGeneratePrivateKeyError => write!(f, "CryptoGeneratePrivateKeyError"),
            ProvisioningErrorCodes::CryptoGenerateCSRError => write!(f, "CryptoGenerateCSRError"),
            ProvisioningErrorCodes::CSRSignReadFileError => write!(f, "CSRSignReadFileError"),
            ProvisioningErrorCodes::CSRSignUnknownError => write!(f, "CSRSignUnknownError"),
            ProvisioningErrorCodes::CSRSignServerError => write!(f, "CSRSignServerError"),
            ProvisioningErrorCodes::CSRSignNotFoundError => write!(f, "CSRSignNotFoundError"),
            ProvisioningErrorCodes::CSRSignBadRequestError => write!(f, "CSRSignBadRequestError"),
            ProvisioningErrorCodes::CSRSignResponseParseError => write!(f, "CSRSignResponseParseError"),
            ProvisioningErrorCodes::CertificateWriteError => write!(f, "CertificateWriteError"),
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
        write!(f, "(code: {:?}, message: {})", self.code, self.message)
    }
}

impl ProvisioningError {
    pub fn new(code: ProvisioningErrorCodes, message: String, capture_error: bool) -> Self {
        let trace_id = find_current_trace_id();
        error!(
            target = "server",
            "error: (code: {:?}, message: {})", code, message
        );
        if capture_error {
            let error = &anyhow::anyhow!(code).context(format!(
                "error: (code: {:?}, messages: {} trace:{:?})",
                code, message, trace_id
            ));
            capture_anyhow(error);
        }
        Self { code, message }
    }
}


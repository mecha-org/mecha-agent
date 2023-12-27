use sentry_anyhow::capture_anyhow;
use std::fmt;
use tracing::error;

#[derive(Debug, Default, Clone, Copy)]
pub enum CryptoErrorCodes {
    GenerateCSRError,
    GeneratePrivateKeyError,
    OpenPrivateKeyError,
    ReadPrivateKeyError,
    LoadSignerError,
    UpdateSignerError,
    PemDeserializeError,
    ExtractSubjectNameError,
    FilePathError,
    GenerateFingerprintError,
    ReadCertFileError,
    #[default]
    UnknownError,
}

impl fmt::Display for CryptoErrorCodes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CryptoErrorCodes::GenerateCSRError => write!(f, "CryptoErrorCodes: GenerateCSRError"),
            CryptoErrorCodes::GeneratePrivateKeyError => {
                write!(f, "CryptoErrorCodes: GeneratePrivateKeyError")
            }
            CryptoErrorCodes::OpenPrivateKeyError => {
                write!(f, "CryptoErrorCodes: OpenPrivateKeyError")
            }
            CryptoErrorCodes::ReadPrivateKeyError => {
                write!(f, "CryptoErrorCodes: ReadPrivateKeyError")
            }
            CryptoErrorCodes::LoadSignerError => write!(f, "CryptoErrorCodes: LoadSignerError"),
            CryptoErrorCodes::UpdateSignerError => write!(f, "CryptoErrorCodes: UpdateSignerError"),
            CryptoErrorCodes::PemDeserializeError => {
                write!(f, "CryptoErrorCodes: PemDeserializeError")
            }
            CryptoErrorCodes::UnknownError => write!(f, "CryptoErrorCodes: UnknownError"),
            CryptoErrorCodes::ExtractSubjectNameError => {
                write!(f, "CryptoErrorCodes: ExtractSubjectNameError")
            }
            CryptoErrorCodes::FilePathError => write!(f, "CryptoErrorCodes: FilePathError"),
            CryptoErrorCodes::GenerateFingerprintError => {
                write!(f, "CryptoErrorCodes: GenerateFingerprintError")
            }
            CryptoErrorCodes::ReadCertFileError => write!(f, "CryptoErrorCodes: ReadCertFileError"),
        }
    }
}

#[derive(Debug)]
pub struct CryptoError {
    pub code: CryptoErrorCodes,
    pub message: String,
}

impl std::fmt::Display for CryptoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CryptoErrorCodes:(code: {:?}, message: {})",
            self.code, self.message
        )
    }
}

impl CryptoError {
    pub fn new(code: CryptoErrorCodes, message: String, capture_error: bool) -> Self {
        error!(
            target = "Crypto",
            "error: (code: {:?}, message: {})", code, message
        );
        if capture_error {
            let error = &anyhow::anyhow!(code)
                .context(format!("error: (code: {:?}, message: {})", code, message));
            capture_anyhow(error);
        }
        Self { code, message }
    }
}

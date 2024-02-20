use sentry_anyhow::capture_anyhow;
use std::fmt;

#[derive(Debug, Default, Clone, Copy)]
pub enum FsErrorCodes {
    #[default]
    UnknownError,
    InvalidFileNameError,
    InvalidFilePathError,
    FileRemoveError,
    FileCreateError,
    FileOpenError,
    FileWriteError,
    JoinPathError,
}

impl fmt::Display for FsErrorCodes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FsErrorCodes::UnknownError => {
                write!(f, "FsErrorCodes: UnknownError")
            }
            FsErrorCodes::InvalidFileNameError => {
                write!(f, "FsErrorCodes: InvalidFileNameError")
            }
            FsErrorCodes::InvalidFilePathError => {
                write!(f, "FsErrorCodes: InvalidFilePathError")
            }
            FsErrorCodes::FileRemoveError => {
                write!(f, "FsErrorCodes: FileRemoveError")
            }
            FsErrorCodes::FileCreateError => {
                write!(f, "FsErrorCodes: FileCreateError")
            }
            FsErrorCodes::FileOpenError => {
                write!(f, "FsErrorCodes: FileOpenError")
            }
            FsErrorCodes::JoinPathError => {
                write!(f, "FsErrorCodes: JoinPathError")
            }
            FsErrorCodes::FileWriteError => {
                write!(f, "FsErrorCodes: FileWriteError")
            }
        }
    }
}

#[derive(Debug)]
pub struct FsError {
    pub code: FsErrorCodes,
    pub message: String,
}

impl std::fmt::Display for FsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "FsErrorCodes:(code: {:?}, message: {})",
            self.code, self.message
        )
    }
}

impl FsError {
    pub fn new(code: FsErrorCodes, message: String, capture_error: bool) -> Self {
        if capture_error {
            let error = &anyhow::anyhow!(code)
                .context(format!("error: (code: {:?}, message: {})", code, message));
            capture_anyhow(error);
        }
        Self { code, message }
    }
}

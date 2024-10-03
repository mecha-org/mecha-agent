use std::fmt;

#[derive(Debug, Default)]
pub enum IdentityErrorCodes {
    #[default]
    UnknownError,
    GetMachineIdError,
    GetMachineCertError,
}

impl fmt::Display for IdentityErrorCodes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            IdentityErrorCodes::UnknownError => write!(f, "IdentityErrorCodes: UnknownError"),
            IdentityErrorCodes::GetMachineIdError => {
                write!(f, "IdentityErrorCodes: GetMachineIdError")
            }
            IdentityErrorCodes::GetMachineCertError => {
                write!(f, "IdentityErrorCodes: GetMachineCertError")
            }
        }
    }
}

#[derive(Debug)]
pub struct IdentityError {
    pub code: IdentityErrorCodes,
    pub message: String,
}

impl std::fmt::Display for IdentityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "IdentityErrorCodes:(code: {:?}, message: {})",
            self.code, self.message
        )
    }
}

impl IdentityError {
    pub fn new(code: IdentityErrorCodes, message: String) -> Self {
        Self { code, message }
    }
}

use std::fmt;

#[derive(Debug, Default, Clone, Copy)]
pub enum KeyValueStoreErrorCodes {
    #[default]
    UnknownError,
    DbInitializationError,
    RetrieveValueError,
    DbAcquireLockError,
    InsertError,
    DbNotInitialized,
}

impl fmt::Display for KeyValueStoreErrorCodes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            KeyValueStoreErrorCodes::UnknownError => {
                write!(f, "KeyValueStoreErrorCodes: UnknownError")
            }
            KeyValueStoreErrorCodes::DbInitializationError => {
                write!(f, "KeyValueStoreErrorCodes: DbInitializationError")
            }
            KeyValueStoreErrorCodes::RetrieveValueError => {
                write!(f, "KeyValueStoreErrorCodes: RetrieveValueError")
            }
            KeyValueStoreErrorCodes::DbAcquireLockError => {
                write!(f, "KeyValueStoreErrorCodes: DbAcquireLockError")
            }
            KeyValueStoreErrorCodes::InsertError => {
                write!(f, "KeyValueStoreErrorCodes: InsertError")
            }
            KeyValueStoreErrorCodes::DbNotInitialized => {
                write!(f, "KeyValueStoreErrorCodes: DbNotInitialized")
            }
        }
    }
}

#[derive(Debug)]
pub struct KeyValueStoreError {
    pub code: KeyValueStoreErrorCodes,
    pub message: String,
}

impl std::fmt::Display for KeyValueStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "KeyValueStoreErrorCodes:(code: {:?}, message: {})",
            self.code, self.message
        )
    }
}

impl KeyValueStoreError {
    pub fn new(code: KeyValueStoreErrorCodes, message: String) -> Self {
        Self { code, message }
    }
}

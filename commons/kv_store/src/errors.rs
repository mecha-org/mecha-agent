use sentry_anyhow::capture_anyhow;
use std::fmt;
use tracing::error;
use tracing_opentelemetry_instrumentation_sdk::find_current_trace_id;

#[derive(Debug, Default, Clone, Copy)]
pub enum KeyValueStoreErrorCodes {
    #[default]
    UnknownError,
    DbInitializationError,
    RetrieveValueError,
    DbAcquireLockError,
    InsertError,
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
    pub fn new(code: KeyValueStoreErrorCodes, message: String, capture_error: bool) -> Self {
        let trace_id = find_current_trace_id();
        error!(
            target = "key_value_store",
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

pub mod errors;
extern crate sled;
use anyhow::{bail, Result};

use crate::errors::{KeyValueStoreError, KeyValueStoreErrorCodes};

#[derive(Clone)]
pub struct KevValueStoreClient {
    db: sled::Db,
}

impl KevValueStoreClient {
    pub fn new(storage_file_path: String) -> Result<Self> {
        Ok(Self {
            db: match sled::open(storage_file_path) {
                Ok(s) => s,
                Err(e) => bail!(KeyValueStoreError::new(
                    KeyValueStoreErrorCodes::DbInitializationError,
                    format!("database initialization failed error - {}", e),
                    true
                )),
            },
        })
    }
    pub fn set(&self, key: &str, value: &str) -> Result<bool> {
        // insert and get
        let _last_inserted = self.db.insert(key, value);
        Ok(true)
    }
    pub fn get(&self, key: &str) -> Result<Option<String>> {
        let last_inserted = self.db.get(key);
        match last_inserted {
            Ok(s) => Ok(s.map(|s| String::from_utf8(s.to_vec()).unwrap())),
            Err(e) => Err(anyhow::Error::new(e)),
        }
    }
}

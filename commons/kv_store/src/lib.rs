pub mod errors;
extern crate sled;
use anyhow::{bail, Result};
use fs::construct_dir_path;
use lazy_static::lazy_static;
use sled::Db;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tracing::info;

use crate::errors::{KeyValueStoreError, KeyValueStoreErrorCodes};
static DATABASE_STORE_FIILE_PATH: &str = "~/.mecha/agent/db";
// Singleton database connection
lazy_static! {
    static ref DATABASE: Arc<Mutex<Db>> = {
        let file_path = fs::construct_dir_path(DATABASE_STORE_FIILE_PATH.clone()).unwrap();
        Arc::new(Mutex::new(sled::open(&file_path).unwrap()))
    };
}

#[derive(Clone)]
pub struct KeyValueStoreClient;

impl KeyValueStoreClient {
    pub fn new() -> Self {
        KeyValueStoreClient
    }
    pub fn set(&mut self, settings: HashMap<String, String>) -> Result<bool> {
        info!(target = "key_value_store", task = "set", "init");
        let db = match DATABASE.lock() {
            Ok(d) => d,
            Err(e) => bail!(KeyValueStoreError::new(
                KeyValueStoreErrorCodes::DbAcquireLockError,
                format!("Error acquiring lock on set - {}", e),
                true
            )),
        };

        for (key, value) in settings {
            match db.insert(key, value.as_str()) {
                Ok(_) => {}
                Err(e) => bail!(KeyValueStoreError::new(
                    KeyValueStoreErrorCodes::InsertError,
                    format!("Error inserting value into db - {}", e),
                    true
                )),
            };
        }
        info!(target = "key_value_store", task = "set", "completed");
        Ok(true)
    }
    pub fn get(&self, key: &str) -> Result<Option<String>> {
        info!(target = "key_value_store", task = "get", "init");
        let db = match DATABASE.lock() {
            Ok(d) => d,
            Err(e) => bail!(KeyValueStoreError::new(
                KeyValueStoreErrorCodes::DbAcquireLockError,
                format!("Error acquiring lock on get - {}", e),
                true
            )),
        };
        let last_inserted = db.get(key);
        match last_inserted {
            Ok(s) => {
                info!(target = "key_value_store", task = "get", "completed");
                Ok(s.map(|s| String::from_utf8(s.to_vec()).unwrap()))
            }
            Err(e) => bail!(KeyValueStoreError::new(
                KeyValueStoreErrorCodes::RetrieveValueError,
                format!("Error retrieving value from db - {}", e),
                true
            )),
        }
    }
}

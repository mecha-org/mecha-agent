pub mod errors;
extern crate sled;
use anyhow::{bail, Result};
use lazy_static::lazy_static;
use sled::Db;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tracing::{debug, error, info, trace};

use crate::errors::{KeyValueStoreError, KeyValueStoreErrorCodes};
const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");
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
        trace!(
            func = "set",
            package = PACKAGE_NAME,
            "init - settings: {:?}",
            settings
        );
        let db = match DATABASE.lock() {
            Ok(d) => d,
            Err(e) => {
                error!(
                    func = "set",
                    package = PACKAGE_NAME,
                    "failed to acquire lock on db - {}",
                    e
                );
                bail!(KeyValueStoreError::new(
                    KeyValueStoreErrorCodes::DbAcquireLockError,
                    format!("error acquiring lock on set - {}", e),
                    true
                ))
            }
        };

        for (key, value) in settings {
            debug!(
                func = "set",
                package = PACKAGE_NAME,
                "inserting key: {}, value: {}",
                key,
                value
            );
            match db.insert(key, value.as_str()) {
                Ok(_) => {}
                Err(e) => {
                    error!(
                        func = "set",
                        package = PACKAGE_NAME,
                        "failed to insert value into db - {}",
                        e
                    );
                    bail!(KeyValueStoreError::new(
                        KeyValueStoreErrorCodes::InsertError,
                        format!("error inserting value into db - {}", e),
                        true
                    ))
                }
            };
        }
        info!(
            func = "set",
            package = PACKAGE_NAME,
            "settings stored to database!",
        );
        Ok(true)
    }
    pub fn get(&self, key: &str) -> Result<Option<String>> {
        trace!(func = "get", package = PACKAGE_NAME, " key: {}", key);
        let db = match DATABASE.lock() {
            Ok(d) => d,
            Err(e) => {
                error!(
                    func = "get",
                    package = PACKAGE_NAME,
                    "failed to acquire lock on db - {}",
                    e
                );
                bail!(KeyValueStoreError::new(
                    KeyValueStoreErrorCodes::DbAcquireLockError,
                    format!("error acquiring lock on get - {}", e),
                    true
                ))
            }
        };
        let last_inserted = db.get(key);
        match last_inserted {
            Ok(s) => {
                debug!(
                    func = "get",
                    package = PACKAGE_NAME,
                    "retrieved value from db key - {}",
                    key
                );
                Ok(s.map(|s| String::from_utf8(s.to_vec()).unwrap()))
            }
            Err(e) => bail!(KeyValueStoreError::new(
                KeyValueStoreErrorCodes::RetrieveValueError,
                format!(
                    "Error retrieving value from db key - {}, error - {}",
                    key, e
                ),
                true
            )),
        }
    }
}

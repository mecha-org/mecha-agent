pub mod errors;
extern crate sled;
use std::path::PathBuf;

use anyhow::{bail, Result};

use crate::errors::{KeyValueStoreError, KeyValueStoreErrorCodes};

#[derive(Clone)]
pub struct KevValueStoreClient {
    db: sled::Db,
}

impl KevValueStoreClient {
    pub fn new(storage_file_path: String) -> Result<Self> {
        // Create dir if not exist
        let _r = check_dir_if_exist(storage_file_path.clone());
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
        let _last_inserted = self.db.insert(key, value);
        Ok(true)
    }
    pub fn get(&self, key: &str) -> Result<Option<String>> {
        let last_inserted = self.db.get(key);
        match last_inserted {
            Ok(s) => Ok(s.map(|s| String::from_utf8(s.to_vec()).unwrap())),
            Err(e) => bail!(KeyValueStoreError::new(
                KeyValueStoreErrorCodes::RetrieveValueError,
                format!("Error retrieving value from db - {}", e),
                true
            )),
        }
    }
}

fn check_dir_if_exist(storage_file_path: String) -> Result<()> {
    println!("check_dir_if_exist: {:?}", storage_file_path);
    let path = PathBuf::from(&storage_file_path);
    if !path.exists() {
        println!("path not exist");
        match mkdirp::mkdirp(&storage_file_path) {
            Ok(p) => {
                println!("path created {:?}", p);
                p
            }
            Err(err) => bail!(err),
        };
    }
    Ok(())
}

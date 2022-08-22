// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::rocks_db::database_schema::{SecureStorageKey, SecureStorageSchema, SecureStorageValue};
use crate::{CryptoKVStorage, Error, GetResponse, KVStorage};
use anyhow::{anyhow, Result};
use aptos_crypto::_once_cell::sync::Lazy;
use aptos_infallible::Mutex;
use aptos_logger::prelude::*;
use aptos_time_service::{TimeService, TimeServiceTrait};
use schemadb::{
    define_schema,
    schema::{KeyCodec, ValueCodec},
    ColumnFamilyName, Options, SchemaBatch, DB,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::{path::Path, sync::Arc, time::Instant};

/// The name of the secure storage db file
pub const SECURE_STORAGE_DB_NAME: &str = "secure_storage_db";

/// The name of the column family
const SECURE_STORAGE_CF_NAME: ColumnFamilyName = "secure_storage";

/// We place a global lock around the database to prevent
/// concurrent threads from creating the same RocksDB file.
static DATABASE: Lazy<Mutex<Option<Arc<DB>>>> = Lazy::new(|| Mutex::new(None));

/// A secure storage implementation that uses a RocksDB backend to persist data
#[derive(Clone)]
pub struct RocksDbStorage {
    database: Option<Arc<DB>>,
    time_service: TimeService,

    #[allow(unused)]
    secure_storage_db_path: PathBuf, // Required for the test method reset_and_clear()
}

impl RocksDbStorage {
    pub fn new<P: AsRef<Path> + Clone>(secure_storage_db_path: P) -> Self {
        Self::new_with_time_service(secure_storage_db_path, TimeService::real())
    }

    fn new_with_time_service<P: AsRef<Path> + Clone>(
        secure_storage_db_path: P,
        time_service: TimeService,
    ) -> Self {
        // Open the database
        let secure_storage_db_path = secure_storage_db_path.as_ref().to_path_buf();
        let database = Some(open_database(&secure_storage_db_path));

        Self {
            database,
            secure_storage_db_path,
            time_service,
        }
    }

    /// Returns the value for the given key in the database (if it exists)
    fn get_value(
        &self,
        secure_storage_key: SecureStorageKey,
    ) -> Result<Option<SecureStorageValue>> {
        let maybe_value = self
            .database
            .as_ref()
            .unwrap()
            .get::<SecureStorageSchema>(&secure_storage_key)
            .map_err(|error| {
                Error::InternalError(format!(
                    "Failed to read secure storage value for key: {:?}. Error: {:?}",
                    secure_storage_key, error
                ))
            })?;
        Ok(maybe_value)
    }

    /// Write the key value pair to the database
    fn set_key_value(
        &self,
        secure_storage_key: SecureStorageKey,
        secure_storage_value: SecureStorageValue,
    ) -> Result<(), Error> {
        // Create the schema batch
        let batch = SchemaBatch::new();
        batch
            .put::<SecureStorageSchema>(&secure_storage_key, &secure_storage_value)
            .map_err(|error| {
                Error::InternalError(format!(
                    "Failed to batch put the key and value. Key: {:?}, Value: {:?}. Error: {:?}",
                    secure_storage_key, secure_storage_value, error
                ))
            })?;

        // Write the schema batch to the database
        self.database
            .as_ref()
            .unwrap()
            .write_schemas(batch)
            .map_err(|error| {
                Error::InternalError(format!(
                    "Failed to write the secure storage schema. Error: {:?}",
                    error
                ))
            })?;

        Ok(())
    }
}

impl KVStorage for RocksDbStorage {
    fn available(&self) -> Result<(), Error> {
        Ok(())
    }

    fn get<V: DeserializeOwned>(&self, key: &str) -> Result<GetResponse<V>, Error> {
        // Create the key
        let serialized_key = serde_json::to_vec(key).unwrap();
        let secure_storage_key = SecureStorageKey::SerializedKey(serialized_key);

        // Fetch the value
        let secure_storage_value = self
            .get_value(secure_storage_key.clone())
            .map_err(|error| {
                Error::InternalError(format!(
                    "Failed to get value for key: {:?}. Error: {:?}",
                    secure_storage_key,
                    error.to_string()
                ))
            })?;
        match secure_storage_value {
            Some(secure_storage_value) => {
                let SecureStorageValue::SerializedValue(value) = secure_storage_value;
                Ok(serde_json::from_slice(&value)?)
            }
            None => Err(Error::KeyNotSet(key.to_string())),
        }
    }

    fn set<V: Serialize>(&mut self, key: &str, value: V) -> Result<(), Error> {
        // Create the key
        let serialized_key = serde_json::to_vec(key).unwrap();
        let secure_storage_key = SecureStorageKey::SerializedKey(serialized_key);

        // Create the value
        let now = self.time_service.now_secs();
        let serialized_value = serde_json::to_vec(&GetResponse::new(value, now))?;
        let secure_storage_value = SecureStorageValue::SerializedValue(serialized_value);

        // Insert the key value pair into the database
        self.set_key_value(secure_storage_key, secure_storage_value)
    }

    #[cfg(any(test, feature = "testing"))]
    fn reset_and_clear(&mut self) -> Result<(), Error> {
        // Drop all references to the database
        self.database = None;
        *DATABASE.lock() = None;

        // Delete the database
        assert!(self.secure_storage_db_path.exists());
        std::fs::remove_dir_all(self.secure_storage_db_path.clone()).unwrap();

        // Open a new database
        let database = open_database(&self.secure_storage_db_path);
        self.database = Some(database);

        Ok(())
    }
}

impl CryptoKVStorage for RocksDbStorage {}

/// The raw schema format used by the database
pub mod database_schema {
    use super::*;

    // This defines a physical storage schema for the secure storage.
    //
    // The key will be a bcs serialized SecureStorageKey type.
    // The value will be a bcs serialized SecureStorageValue type.
    //
    // |<-------key-------->|<--------value------->|
    // | secure storage key | secure storage value |
    define_schema!(
        SecureStorageSchema,
        SecureStorageKey,
        SecureStorageValue,
        SECURE_STORAGE_CF_NAME
    );

    /// A secure storage key that can be inserted into the database
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub enum SecureStorageKey {
        SerializedKey(Vec<u8>),
    }

    /// A secure storage value that can be inserted into the database
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub enum SecureStorageValue {
        SerializedValue(Vec<u8>),
    }

    impl KeyCodec<SecureStorageSchema> for SecureStorageKey {
        fn encode_key(&self) -> Result<Vec<u8>> {
            bcs::to_bytes(self).map_err(|error| {
                anyhow!(
                    "Failed to encode secure storage key: {:?}. Error: {:?}",
                    self,
                    error
                )
            })
        }

        fn decode_key(data: &[u8]) -> Result<Self> {
            bcs::from_bytes::<SecureStorageKey>(data).map_err(|error| {
                anyhow!(
                    "Failed to decode secure storage key: {:?}. Error: {:?}",
                    data,
                    error
                )
            })
        }
    }

    impl ValueCodec<SecureStorageSchema> for SecureStorageValue {
        fn encode_value(&self) -> Result<Vec<u8>> {
            bcs::to_bytes(self).map_err(|error| {
                anyhow!(
                    "Failed to encode secure storage value: {:?}. Error: {:?}",
                    self,
                    error
                )
            })
        }

        fn decode_value(data: &[u8]) -> Result<Self> {
            bcs::from_bytes::<SecureStorageValue>(data).map_err(|error| {
                anyhow!(
                    "Failed to decode secure storage value: {:?}. Error: {:?}",
                    data,
                    error
                )
            })
        }
    }
}

/// Opens the database at the given path. This function uses
/// a global lock to ensure that only a single database file exists
/// at any given time (i.e., it's possible that concurrent threads
/// might try and create the database concurrently).
fn open_database(database_path: &PathBuf) -> Arc<DB> {
    // If the database has already been created, return a new reference
    let mut global_database = DATABASE.lock();
    if let Some(database) = global_database.as_ref() {
        return database.clone();
    }

    // Otherwise, create the database
    let mut options = Options::default();
    options.create_if_missing(true);
    options.create_missing_column_families(true);

    // Open the database file
    let instant = Instant::now();
    let database = Arc::new(
        DB::open(
            database_path.clone(),
            "secure_storage",
            vec![SECURE_STORAGE_CF_NAME],
            &options,
        )
        .unwrap_or_else(|error| {
            panic!(
                "Failed to open/create the secure storage database at: {:?}. Error: {:?}",
                database_path, error
            )
        }),
    );
    info!(
        "Opened the secure storage database at: {:?}, in {:?} ms",
        database_path,
        instant.elapsed().as_millis()
    );

    // Update the global reference and return a clone
    global_database.insert(database).clone()
}

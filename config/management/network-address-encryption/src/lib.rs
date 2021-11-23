// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use diem_global_constants::VALIDATOR_NETWORK_ADDRESS_KEYS;
use diem_infallible::RwLock;
use diem_secure_storage::{Error as StorageError, KVStorage, Storage};
use diem_types::{
    account_address::AccountAddress,
    network_address::{
        self,
        encrypted::{
            Key, KeyVersion, TEST_SHARED_VAL_NETADDR_KEY, TEST_SHARED_VAL_NETADDR_KEY_VERSION,
        },
    },
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Unable to deserialize address for account {0}: {1}")]
    AddressDeserialization(AccountAddress, String),
    #[error("Unable to decrypt address for account {0}: {1}")]
    DecryptionError(AccountAddress, String),
    #[error("Failed (de)serializing validator_network_address_keys")]
    BCSError(#[from] bcs::Error),
    #[error("NetworkAddress parse error {0}")]
    ParseError(#[from] network_address::ParseError),
    #[error("Failed reading validator_network_address_keys from storage")]
    StorageError(#[from] StorageError),
    #[error("The specified version does not exist in validator_network_address_keys: {0}")]
    VersionNotFound(KeyVersion),
}

pub struct Encryptor<S> {
    storage: S,
    cached_keys: RwLock<Option<ValidatorKeys>>,
}

impl<S> Encryptor<S> {
    pub fn new(storage: S) -> Self {
        Self {
            storage,
            cached_keys: RwLock::new(None),
        }
    }
}

impl<S> Encryptor<S>
where
    S: KVStorage,
{
    pub fn add_key(&mut self, version: KeyVersion, key: Key) -> Result<(), Error> {
        let mut keys = self.read()?;
        keys.keys.insert(version, StorageKey(key));
        self.write(&keys)
    }

    pub fn set_current_version(&mut self, version: KeyVersion) -> Result<(), Error> {
        let mut keys = self.read()?;
        if keys.keys.get(&version).is_some() {
            keys.current = version;
            self.write(&keys)
        } else {
            Err(Error::VersionNotFound(version))
        }
    }

    pub fn current_version(&self) -> Result<KeyVersion, Error> {
        self.read().map(|keys| keys.current)
    }

    fn read(&self) -> Result<ValidatorKeys, Error> {
        let result = self
            .storage
            .get::<ValidatorKeys>(VALIDATOR_NETWORK_ADDRESS_KEYS)
            .map(|v| v.value)
            .map_err(|e| e.into());

        match &result {
            Ok(keys) => {
                *self.cached_keys.write() = Some(keys.clone());
            }
            Err(err) => diem_logger::error!(
                "Unable to read {} from storage: {}",
                VALIDATOR_NETWORK_ADDRESS_KEYS,
                err
            ),
        }

        let keys = self.cached_keys.read();
        keys.as_ref().map_or(result, |v| Ok(v.clone()))
    }

    fn write(&mut self, keys: &ValidatorKeys) -> Result<(), Error> {
        self.storage
            .set(VALIDATOR_NETWORK_ADDRESS_KEYS, keys)
            .map_err(|e| e.into())
    }

    pub fn initialize(&mut self) -> Result<(), Error> {
        self.write(&ValidatorKeys::default())
    }
}

impl Encryptor<Storage> {
    /// This generates an empty encryptor for use in scenarios where encryption is not necessary.
    /// Any encryption operations (e.g., encrypt / decrypt) will return errors.
    pub fn empty() -> Self {
        let storage = Storage::InMemoryStorage(diem_secure_storage::InMemoryStorage::new());
        Encryptor::new(storage)
    }

    /// This generates an encryptor for use in testing scenarios. The encryptor is
    /// initialized with a test network encryption key.
    pub fn for_testing() -> Self {
        let mut encryptor = Self::empty();
        encryptor.initialize_for_testing().unwrap();
        encryptor
    }

    pub fn initialize_for_testing(&mut self) -> Result<(), Error> {
        self.initialize()?;
        self.add_key(
            TEST_SHARED_VAL_NETADDR_KEY_VERSION,
            TEST_SHARED_VAL_NETADDR_KEY,
        )?;
        self.set_current_version(TEST_SHARED_VAL_NETADDR_KEY_VERSION)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct StorageKey(
    #[serde(
        serialize_with = "diem_secure_storage::to_base64",
        deserialize_with = "from_base64"
    )]
    Key,
);

pub fn to_base64<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&base64::encode(bytes))
}

pub fn from_base64<'de, D>(deserializer: D) -> Result<Key, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = serde::Deserialize::deserialize(deserializer)?;
    base64::decode(s)
        .map_err(serde::de::Error::custom)
        .and_then(|v| {
            std::convert::TryInto::try_into(v.as_slice()).map_err(serde::de::Error::custom)
        })
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ValidatorKeys {
    keys: HashMap<KeyVersion, StorageKey>,
    current: KeyVersion,
}

impl Default for ValidatorKeys {
    fn default() -> Self {
        ValidatorKeys {
            current: 0,
            keys: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use diem_secure_storage::{InMemoryStorage, Namespaced};
    use rand::{rngs::OsRng, Rng, RngCore, SeedableRng};

    #[test]
    fn e2e() {
        let storage = Storage::InMemoryStorage(InMemoryStorage::new());
        let mut encryptor = Encryptor::new(storage);
        encryptor.initialize().unwrap();

        let mut rng = rand::rngs::StdRng::from_seed(OsRng.gen());
        let mut key = [0; network_address::encrypted::KEY_LEN];
        rng.fill_bytes(&mut key);
        encryptor.add_key(0, key).unwrap();
        encryptor.set_current_version(0).unwrap();
        rng.fill_bytes(&mut key);
        encryptor.add_key(1, key).unwrap();
        encryptor.set_current_version(1).unwrap();
        rng.fill_bytes(&mut key);
        encryptor.add_key(4, key).unwrap();
        encryptor.set_current_version(4).unwrap();

        encryptor.set_current_version(5).unwrap_err();
    }

    // The only purpose of this test is to generate a baseline for vault
    #[ignore]
    #[test]
    fn initializer() {
        let storage = Storage::from(Namespaced::new(
            "network_address_encryption_keys",
            Box::new(Storage::VaultStorage(
                diem_secure_storage::VaultStorage::new(
                    "http://127.0.0.1:8200".to_string(),
                    "root_token".to_string(),
                    None,
                    None,
                    true,
                    None,
                    None,
                ),
            )),
        ));
        let mut encryptor = Encryptor::new(storage);
        encryptor.initialize().unwrap();
        let mut rng = rand::rngs::StdRng::from_seed(OsRng.gen());
        let mut key = [0; network_address::encrypted::KEY_LEN];
        rng.fill_bytes(&mut key);
        encryptor.add_key(0, key).unwrap();
        encryptor.set_current_version(0).unwrap();
    }
}

// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::{
    storage_key, LoadedSecretShare, Result, SecretShareKey, SecretShareStorage,
    SecretShareStorageError,
};
use aptos_infallible::Mutex;
use aptos_types::secret_sharing::SecretShare;
use std::collections::HashMap;

pub struct InMemorySecretShareStorage {
    shares: Mutex<HashMap<SecretShareKey, Vec<u8>>>,
}

impl InMemorySecretShareStorage {
    pub fn new() -> Self {
        Self {
            shares: Mutex::new(HashMap::new()),
        }
    }

    #[cfg(test)]
    pub(crate) fn insert_raw(&self, key: SecretShareKey, value: Vec<u8>) {
        self.shares.lock().insert(key, value);
    }
}

impl Default for InMemorySecretShareStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretShareStorage for InMemorySecretShareStorage {
    fn save_self_share(&self, share: &SecretShare) -> Result<()> {
        let key = storage_key(share.metadata());
        let serialized = bcs::to_bytes(share)
            .map_err(|error| SecretShareStorageError::Corruption(error.to_string()))?;
        let mut shares = self.shares.lock();
        match shares.get(&key) {
            Some(existing) if existing == &serialized => Ok(()),
            Some(_) => Err(SecretShareStorageError::Conflict {
                epoch: key.0,
                block_id: key.1,
            }),
            None => {
                shares.insert(key, serialized);
                Ok(())
            },
        }
    }

    fn load_self_shares(&self, epoch: u64) -> Result<Vec<LoadedSecretShare>> {
        Ok(self
            .shares
            .lock()
            .iter()
            .filter(|((stored_epoch, _), _)| *stored_epoch == epoch)
            .map(|(key, serialized)| {
                bcs::from_bytes::<SecretShare>(serialized)
                    .map_err(|error| SecretShareStorageError::Corruption(error.to_string()))
                    .and_then(|share| {
                        if storage_key(share.metadata()) == *key {
                            Ok(share)
                        } else {
                            Err(SecretShareStorageError::Corruption(format!(
                                "stored key does not match secret share metadata for epoch {}, block {}",
                                key.0, key.1
                            )))
                        }
                    })
            })
            .collect())
    }

    fn prune_before_epoch(&self, epoch: u64) -> Result<()> {
        self.shares
            .lock()
            .retain(|(stored_epoch, _), _| *stored_epoch >= epoch);
        Ok(())
    }
}

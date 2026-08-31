// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::{storage_key, LoadedSecretShare, SecretShareKey, SecretShareStorage};
use anyhow::{bail, ensure, Result};
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
        let serialized = bcs::to_bytes(share)?;
        let mut shares = self.shares.lock();
        match shares.get(&key) {
            Some(existing) if existing == &serialized => Ok(()),
            Some(_) => bail!(
                "conflicting secret share for epoch {}, block {}",
                key.0,
                key.1
            ),
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
                let share = bcs::from_bytes::<SecretShare>(serialized)?;
                ensure!(
                    storage_key(share.metadata()) == *key,
                    "stored key does not match secret share metadata for epoch {}, block {}",
                    key.0,
                    key.1
                );
                Ok(share)
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

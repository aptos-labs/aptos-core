// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::{storage_key, SecretShareKey, SecretShareStorage};
use anyhow::Result;
use aptos_infallible::Mutex;
use aptos_types::secret_sharing::SecretShare;
use std::collections::HashMap;

pub struct InMemorySecretShareStorage {
    shares: Mutex<HashMap<SecretShareKey, SecretShare>>,
}

impl InMemorySecretShareStorage {
    pub fn new() -> Self {
        Self {
            shares: Mutex::new(HashMap::new()),
        }
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
        self.shares.lock().insert(key, share.clone());
        Ok(())
    }

    fn get_all_self_shares(&self) -> Result<Vec<SecretShare>> {
        Ok(self.shares.lock().values().cloned().collect())
    }

    fn prune_self_shares(&self, keys: &[SecretShareKey]) -> Result<()> {
        let mut shares = self.shares.lock();
        for key in keys {
            shares.remove(key);
        }
        Ok(())
    }
}

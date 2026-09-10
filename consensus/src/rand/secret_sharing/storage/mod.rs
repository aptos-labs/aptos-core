// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

mod db;
#[cfg(test)]
mod in_memory;
mod schema;

use anyhow::Result;
use aptos_crypto::HashValue;
use aptos_types::secret_sharing::{SecretShare, SecretShareMetadata};
pub use db::SecretShareDb;
#[cfg(test)]
pub use in_memory::InMemorySecretShareStorage;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct SecretShareKey {
    pub epoch: u64,
    pub block_id: HashValue,
}

pub trait SecretShareStorage: Send + Sync + 'static {
    fn save_self_share(&self, share: &SecretShare) -> Result<()>;

    fn get_all_self_shares(&self) -> Result<Vec<SecretShare>>;

    fn prune_self_shares(&self, keys: &[SecretShareKey]) -> Result<()>;
}

pub(crate) fn storage_key(metadata: &SecretShareMetadata) -> SecretShareKey {
    SecretShareKey {
        epoch: metadata.epoch,
        block_id: metadata.block_id,
    }
}

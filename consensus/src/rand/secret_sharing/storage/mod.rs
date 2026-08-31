// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

mod db;
#[cfg(test)]
mod in_memory;
mod schema;

use aptos_crypto::HashValue;
use aptos_types::secret_sharing::{SecretShare, SecretShareMetadata};
pub use db::SecretShareDb;
#[cfg(test)]
pub use in_memory::InMemorySecretShareStorage;
use thiserror::Error;

pub type SecretShareKey = (u64, HashValue);

#[derive(Debug, Error)]
pub enum SecretShareStorageError {
    #[error("conflicting secret share for epoch {epoch}, block {block_id}")]
    Conflict { epoch: u64, block_id: HashValue },
    #[error("corrupt secret share storage record: {0}")]
    Corruption(String),
    #[error("secret share storage I/O failure: {0}")]
    Io(String),
}

pub type Result<T> = std::result::Result<T, SecretShareStorageError>;
pub type LoadedSecretShare = Result<SecretShare>;

pub trait SecretShareStorage: Send + Sync + 'static {
    fn save_self_share(&self, share: &SecretShare) -> Result<()>;

    fn load_self_shares(&self, epoch: u64) -> Result<Vec<LoadedSecretShare>>;

    fn prune_before_epoch(&self, epoch: u64) -> Result<()>;
}

pub(crate) fn storage_key(metadata: &SecretShareMetadata) -> SecretShareKey {
    (metadata.epoch, metadata.block_id)
}

// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

mod db;
#[cfg(test)]
mod in_memory;
mod schema;

use anyhow::Result;
use aptos_consensus_types::common::Round;
use aptos_crypto::HashValue;
use aptos_types::secret_sharing::{SecretShare, SecretShareMetadata};
pub use db::SecretShareDb;
#[cfg(test)]
pub use in_memory::InMemorySecretShareStorage;

pub type SecretShareKey = (u64, HashValue);
pub type LoadedSecretShare = Result<SecretShare>;

pub trait SecretShareStorage: Send + Sync + 'static {
    fn save_self_share(&self, share: &SecretShare) -> Result<()>;

    fn load_self_shares(&self, epoch: u64) -> Result<Vec<LoadedSecretShare>>;

    fn prune_before_epoch(&self, epoch: u64) -> Result<()>;

    fn prune_before_round(&self, epoch: u64, round: Round) -> Result<()>;
}

pub(crate) fn storage_key(metadata: &SecretShareMetadata) -> SecretShareKey {
    (metadata.epoch, metadata.block_id)
}

// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![forbid(unsafe_code)]

use aptos_crypto::HashValue;
use aptos_schemadb::DB;
use aptos_types::state_store::{state_key::StateKey, NUM_STATE_SHARDS};
use std::sync::Arc;

#[derive(Debug)]
pub struct ShardedKvDb {
    metadata_db: Arc<DB>,
    shards: [Arc<DB>; NUM_STATE_SHARDS],
}

impl ShardedKvDb {
    pub fn new(metadata_db: Arc<DB>, shards: [Arc<DB>; NUM_STATE_SHARDS]) -> Self {
        Self {
            metadata_db,
            shards,
        }
    }

    pub fn metadata_db(&self) -> &Arc<DB> {
        &self.metadata_db
    }

    pub fn shard(&self, idx: usize) -> &Arc<DB> {
        &self.shards[idx]
    }

    pub fn shards(&self) -> &[Arc<DB>; NUM_STATE_SHARDS] {
        &self.shards
    }

    pub fn shard_of_state_key(state_key: &StateKey) -> usize {
        state_key.get_shard_id()
    }

    pub fn shard_of_hash(state_key_hash: HashValue) -> usize {
        usize::from(state_key_hash.nibble(0))
    }
}

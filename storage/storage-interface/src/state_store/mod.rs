// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

mod hot_state;
pub mod state;
pub mod state_delta;
pub mod state_summary;
pub mod state_update_refs;
pub mod state_view;
pub mod state_with_summary;
pub mod versioned_state_value;

use aptos_crypto::HashValue;
use aptos_types::{
    state_store::{hot_state::HotStateValue, NUM_STATE_SHARDS},
    transaction::Version,
};
use std::collections::HashMap;

#[derive(Debug)]
pub struct HotInsertionOp {
    pub value: HotStateValue,
    /// `Some(version)` for occupied entries and `None` for vacant.
    pub value_version: Option<Version>,
    /// The `hot_since_version` of the DB entry being superseded.
    /// `None` means this is a first write (creation or promotion).
    pub superseded_version: Option<Version>,
}

#[derive(Debug)]
pub struct HotEvictionOp {
    pub eviction_version: Version,
    /// The `hot_since_version` of the DB entry being superseded. `None` if the key was never
    /// persisted to hot DB (e.g. promoted and evicted in the same batch, unlikely though).
    pub superseded_version: Option<Version>,
}

#[derive(Debug)]
pub struct HotStateShardUpdates {
    pub insertions: HashMap<HashValue, HotInsertionOp>,
    // TODO(HotState): per-block eviction tracking will be needed for cold-write elimination.
    pub evictions: HashMap<HashValue, HotEvictionOp>,
}

impl HotStateShardUpdates {
    pub fn new(
        insertions: HashMap<HashValue, HotInsertionOp>,
        evictions: HashMap<HashValue, HotEvictionOp>,
    ) -> Self {
        Self {
            insertions,
            evictions,
        }
    }
}

#[derive(Debug)]
pub struct HotStateUpdates {
    pub for_last_checkpoint: Option<[HotStateShardUpdates; NUM_STATE_SHARDS]>,
    pub for_latest: Option<[HotStateShardUpdates; NUM_STATE_SHARDS]>,
}

impl HotStateUpdates {
    pub fn new_empty() -> Self {
        Self {
            for_last_checkpoint: None,
            for_latest: None,
        }
    }
}

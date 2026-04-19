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
    state_store::{hot_state::HotStateValue, state_key::StateKey, NUM_STATE_SHARDS},
    transaction::Version,
};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct HotInsertionOp {
    /// The key associated with this insertion. Carried alongside the hash so that downstream
    /// consumers (hot JMT commit, etc.) don't need to look it up from a keyless source.
    pub state_key: StateKey,
    pub value: HotStateValue,
    /// `Some(version)` for occupied entries and `None` for vacant.
    pub value_version: Option<Version>,
    /// The `hot_since_version` of the DB entry being superseded.
    /// `None` means this is a first write (creation or promotion).
    pub superseded_version: Option<Version>,
}

#[derive(Clone, Debug)]
pub struct HotEvictionOp {
    pub eviction_version: Version,
    /// The `hot_since_version` of the DB entry being superseded. `None` if the key was never
    /// persisted to hot DB (e.g. promoted and evicted in the same batch, unlikely though).
    pub superseded_version: Option<Version>,
}

#[derive(Clone, Debug, Default)]
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

    /// Merges `other` into `self`, treating `other` as logically later in time.
    ///
    /// Semantics mirror the within-batch logic in `LedgerState::update`: when the same key
    /// appears in both, the earlier DB-level `superseded_version` is preserved so that
    /// pruner targeting stays correct across insert/evict/reinsert chains.
    pub fn merge(&mut self, other: HotStateShardUpdates) {
        for (key_hash, mut op) in other.insertions {
            if let Some(evicted) = self.evictions.remove(&key_hash) {
                // Earlier eviction → later insertion: carry forward the eviction's
                // superseded_version so the insertion still targets the original DB entry.
                op.superseded_version = evicted.superseded_version;
                self.insertions.insert(key_hash, op);
            } else if let Some(prev) = self.insertions.get(&key_hash) {
                // Earlier insertion → later insertion: keep the first insertion's
                // superseded_version (the DB-level predecessor).
                op.superseded_version = prev.superseded_version;
                self.insertions.insert(key_hash, op);
            } else {
                self.insertions.insert(key_hash, op);
            }
        }
        for (key_hash, mut evict) in other.evictions {
            if let Some(prev) = self.insertions.remove(&key_hash) {
                // Earlier insertion → later eviction: eviction wins but inherits the
                // insertion's DB-level superseded_version.
                evict.superseded_version = prev.superseded_version;
                self.evictions.insert(key_hash, evict);
            } else {
                assert!(
                    !self.evictions.contains_key(&key_hash),
                    "Key {key_hash} cannot be evicted twice."
                );
                self.evictions.insert(key_hash, evict);
            }
        }
    }
}

/// Creates an empty `[HotStateShardUpdates; NUM_STATE_SHARDS]`, used as the seed for accumulating
/// hot state updates across chunks.
pub fn empty_hot_state_shard_updates() -> [HotStateShardUpdates; NUM_STATE_SHARDS] {
    std::array::from_fn(|_| HotStateShardUpdates::default())
}

#[derive(Clone, Debug)]
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

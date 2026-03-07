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

use aptos_types::{
    state_store::{hot_state::HotStateValue, state_key::StateKey, NUM_STATE_SHARDS},
    transaction::Version,
};
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub struct HotStateShardUpdates {
    insertions: HashMap<StateKey, HotStateValue>,
    // TODO(HotState): only keys are needed for now, since evictions do not affect cold state.
    evictions: HashSet<StateKey>,
    /// `value_version` for occupied entries — needed for KV persistence, not for Merkle tree.
    /// Only present for insertions that are occupied (not vacant).
    value_versions: HashMap<StateKey, Version>,
}

impl HotStateShardUpdates {
    pub fn new(
        insertions: HashMap<StateKey, HotStateValue>,
        evictions: HashSet<StateKey>,
        value_versions: HashMap<StateKey, Version>,
    ) -> Self {
        Self {
            insertions,
            evictions,
            value_versions,
        }
    }

    pub fn insertions(&self) -> &HashMap<StateKey, HotStateValue> {
        &self.insertions
    }

    pub fn evictions(&self) -> &HashSet<StateKey> {
        &self.evictions
    }

    pub fn value_versions(&self) -> &HashMap<StateKey, Version> {
        &self.value_versions
    }
}

#[derive(Debug)]
pub struct HotStateUpdates {
    pub(crate) for_last_checkpoint: Option<[HotStateShardUpdates; NUM_STATE_SHARDS]>,
    pub(crate) for_latest: Option<[HotStateShardUpdates; NUM_STATE_SHARDS]>,
}

impl HotStateUpdates {
    pub fn new_empty() -> Self {
        Self {
            for_last_checkpoint: None,
            for_latest: None,
        }
    }

    pub fn for_last_checkpoint(&self) -> Option<&[HotStateShardUpdates; NUM_STATE_SHARDS]> {
        self.for_last_checkpoint.as_ref()
    }
}

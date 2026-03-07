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
use std::collections::HashMap;

#[derive(Debug)]
pub struct HotStateShardUpdates {
    /// Each insertion carries the `HotStateValue` and an optional `value_version`.
    /// `value_version` is `Some(version)` for occupied entries and `None` for vacant.
    insertions: HashMap<StateKey, (HotStateValue, Option<Version>)>,
    /// Each eviction carries the checkpoint version at which eviction happened.
    // TODO(HotState): per-block eviction tracking will be needed for cold-write elimination.
    evictions: HashMap<StateKey, Version>,
}

impl HotStateShardUpdates {
    pub fn new(
        insertions: HashMap<StateKey, (HotStateValue, Option<Version>)>,
        evictions: HashMap<StateKey, Version>,
    ) -> Self {
        Self {
            insertions,
            evictions,
        }
    }

    pub fn insertions(&self) -> &HashMap<StateKey, (HotStateValue, Option<Version>)> {
        &self.insertions
    }

    pub fn evictions(&self) -> &HashMap<StateKey, Version> {
        &self.evictions
    }
}

#[derive(Debug)]
pub struct HotStateUpdates {
    for_last_checkpoint: Option<[HotStateShardUpdates; NUM_STATE_SHARDS]>,
    for_latest: Option<[HotStateShardUpdates; NUM_STATE_SHARDS]>,
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

    pub fn for_latest(&self) -> Option<&[HotStateShardUpdates; NUM_STATE_SHARDS]> {
        self.for_latest.as_ref()
    }
}

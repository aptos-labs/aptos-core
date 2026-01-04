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

use aptos_types::state_store::{state_key::StateKey, state_value::StateValue, NUM_STATE_SHARDS};
use std::collections::HashMap;

#[derive(Debug)]
pub(crate) struct HotStateShardUpdates {
    insertions: HashMap<StateKey, Option<StateValue>>,
    evictions: HashMap<StateKey, Option<StateValue>>,
}

impl HotStateShardUpdates {
    pub fn new(
        insertions: HashMap<StateKey, Option<StateValue>>,
        evictions: HashMap<StateKey, Option<StateValue>>,
    ) -> Self {
        Self {
            insertions,
            evictions,
        }
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
}

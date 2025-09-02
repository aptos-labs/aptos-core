// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::state::State;
use aptos_experimental_layered_map::LayeredMap;
use aptos_types::{
    state_store::{
        hot_state::HOT_STATE_MAX_ITEMS_PER_SHARD, state_key::StateKey, state_slot::StateSlot,
        NUM_STATE_SHARDS,
    },
    transaction::Version,
};
use std::sync::Arc;

/// This represents two state sparse merkle trees at their versions in memory with the updates
/// reflecting the difference of `current` on top of `base`.
///
/// The `base` is the state SMT that current is based on.
/// The `current` is the state SMT that results from applying updates_since_base on top of `base`.
/// `updates_since_base` tracks all those key-value pairs that's changed since `base`, useful
///  when the next checkpoint is calculated.
#[derive(Clone, Debug)]
pub struct StateDelta {
    pub base: State,
    pub current: State,
    pub shards: Arc<[LayeredMap<StateKey, StateSlot>; NUM_STATE_SHARDS]>,
}

impl StateDelta {
    pub fn new(base: State, current: State) -> Self {
        assert!(current.is_descendant_of(&base));

        let shards = Arc::new(std::array::from_fn(|shard_id| {
            current.shards()[shard_id].view_layers_after(&base.shards()[shard_id])
        }));

        Self {
            base,
            current,
            shards,
        }
    }

    pub fn next_version(&self) -> Version {
        self.current.next_version()
    }

    pub fn base_version(&self) -> Option<Version> {
        self.base.version()
    }

    /// Get the state update for a given state key.
    /// `None` indicates the key is not updated in the delta.
    pub fn get_state_slot(&self, state_key: &StateKey) -> Option<StateSlot> {
        self.shards[state_key.get_shard_id()].get(state_key)
    }

    pub(crate) fn num_free_hot_slots(&self) -> [usize; NUM_STATE_SHARDS] {
        std::array::from_fn(|shard_id| {
            let num_items = self.current.num_hot_items(shard_id);
            assert!(
                num_items <= HOT_STATE_MAX_ITEMS_PER_SHARD,
                "Number of hot state items {} exceeded max size {} in shard {}.",
                num_items,
                HOT_STATE_MAX_ITEMS_PER_SHARD,
                shard_id,
            );
            HOT_STATE_MAX_ITEMS_PER_SHARD - num_items
        })
    }

    pub fn latest_hot_key(&self, shard_id: usize) -> Option<StateKey> {
        self.current.latest_hot_key(shard_id)
    }

    pub fn oldest_hot_key(&self, shard_id: usize) -> Option<StateKey> {
        self.current.oldest_hot_key(shard_id)
    }
}

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::{state::State, NUM_STATE_SHARDS};
use aptos_experimental_layered_map::LayeredMap;
use aptos_types::{
    state_store::{
        hot_state::{LRUEntry, SpeculativeLRUEntry},
        state_key::StateKey,
        state_slot::{StateSlot, HOT_STATE_MAX_ITEMS},
    },
    transaction::Version,
};
use itertools::Itertools;
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
    pub lru: Arc<LayeredMap<StateKey, SpeculativeLRUEntry<StateKey>>>,
}

impl StateDelta {
    pub fn new(base: State, current: State) -> Self {
        assert!(current.is_descendant_of(&base));

        let shards = Arc::new(
            current
                .shards()
                .iter()
                .zip_eq(base.shards().iter())
                .map(|(current, base)| current.view_layers_after(base))
                .collect_vec()
                .try_into()
                .expect("Known to be 16 shards."),
        );

        let lru = Arc::new(
            current
                .hot_state_meta
                .lru_state
                .view_layers_after(&base.hot_state_meta.lru_state),
        );

        Self {
            base,
            current,
            shards,
            lru,
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

    pub fn num_hot_items(&self) -> usize {
        self.current.hot_state_meta.num_items
    }

    pub fn num_free_hot_slots(&self) -> usize {
        HOT_STATE_MAX_ITEMS - self.num_hot_items()
    }

    pub fn hot_state_contains(&self, state_key: &StateKey) -> bool {
        match self.lru.get(state_key) {
            Some(SpeculativeLRUEntry::Existing(_)) => true,
            _ => false,
        }
    }

    pub(crate) fn get_lru_entry(
        &self,
        state_key: &StateKey,
    ) -> Option<SpeculativeLRUEntry<StateKey>> {
        self.lru.get(state_key)
    }

    pub fn get_oldest_key(&self) -> Option<StateKey> {
        self.current.hot_state_meta.oldest.clone()
    }

    pub fn get_newest_key(&self) -> Option<StateKey> {
        self.current.hot_state_meta.latest.clone()
    }
}

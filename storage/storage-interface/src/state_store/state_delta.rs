// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::{state::State, versioned_state_value::StateUpdate, NUM_STATE_SHARDS};
use aptos_experimental_layered_map::LayeredMap;
use aptos_types::{state_store::state_key::StateKey, transaction::Version};
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
    pub shards: Arc<[LayeredMap<StateKey, StateUpdate>; NUM_STATE_SHARDS]>,
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
    pub fn get_state_update(&self, state_key: &StateKey) -> Option<StateUpdate> {
        self.shards[state_key.get_shard_id() as usize].get(state_key)
    }
}

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    metrics::TIMER,
    state_store::{state::State, state_update::StateUpdate, NUM_STATE_SHARDS},
};
use aptos_experimental_layered_map::LayeredMap;
use aptos_metrics_core::TimerHelper;
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
                .shards
                .iter()
                .zip_eq(base.shards.iter())
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

    pub fn new_empty_with_version(_version: Option<u64>) -> StateDelta {
        /* FIXME(aldenhu):
        let smt = SparseMerkleTree::new_empty();
        Self::new(
            smt.clone(),
            version,
            smt,
            version,
            ShardedStateUpdates::new_empty(),
        )
         */
        todo!()
    }

    pub fn new_empty() -> Self {
        Self::new_empty_with_version(None)
    }

    pub fn next_version(&self) -> Version {
        self.current.next_version()
    }

    pub fn parent_version(&self) -> Option<Version> {
        self.base.next_version().checked_sub(1)
    }

    /// Get the state update for a given state key.
    /// `None` indicates the key is not updated in the delta.
    pub fn get_state_update(&self, state_key: &StateKey) -> Option<StateUpdate> {
        self.shards[state_key.get_shard_id() as usize].get(state_key)
    }

    pub fn count_updates_costly(&self) -> usize {
        let _timer = TIMER.timer_with(&["state_delta__count_items_heavy"]);

        self.shards.iter().map(|shard| shard.iter().count()).sum()
    }
}

// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{metrics::TIMER, state_store::NUM_STATE_SHARDS};
use aptos_metrics_core::TimerHelper;
use aptos_types::{
    state_store::{state_key::StateKey, state_value::StateValue},
    write_set::WriteSet,
};
use arr_macro::arr;

pub type StateUpdateRefWithOffset<'kv> = (usize, &'kv StateKey, Option<&'kv StateValue>);

pub struct ShardedStateUpdateRefs<'kv> {
    /// (idx, key, value) tuple per update, sharded
    ///    Converting to Box<[]> to release over-allocated memory during construction
    pub shards: [Box<[StateUpdateRefWithOffset<'kv>]>; NUM_STATE_SHARDS],
    pub num_versions: usize,
}

impl<'kv> ShardedStateUpdateRefs<'kv> {
    pub fn index_write_sets(
        write_sets: impl IntoIterator<Item = &'kv WriteSet>,
        num_write_sets: usize,
    ) -> Self {
        Self::index_per_version_updates(
            write_sets
                .into_iter()
                .map(|write_set| write_set.state_update_refs()),
            num_write_sets,
        )
    }

    pub fn index_per_version_updates<
        UpdateIter: IntoIterator<Item = (&'kv StateKey, Option<&'kv StateValue>)>,
        VersionIter: IntoIterator<Item = UpdateIter>,
    >(
        updates_by_version: VersionIter,
        num_versions: usize,
    ) -> Self {
        let _timer = TIMER.timer_with(&["index_state_updates"]);

        // Over-allocate a bit to minimize re-allocation.
        let mut shards = arr![Vec::with_capacity(num_versions / 8); 16];

        let mut versions_seen = 0;
        for (idx, update_iter) in updates_by_version.into_iter().enumerate() {
            versions_seen += 1;

            for (key, value) in update_iter.into_iter() {
                shards[key.get_shard_id() as usize].push((idx, key, value));
            }
        }
        assert_eq!(versions_seen, num_versions);

        Self {
            shards: shards.map(|shard| shard.into_boxed_slice()),
            num_versions,
        }
    }
}

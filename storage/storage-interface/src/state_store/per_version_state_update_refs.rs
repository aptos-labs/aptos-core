// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    metrics::TIMER,
    state_store::{state_update::StateUpdateRef, NUM_STATE_SHARDS},
};
use aptos_metrics_core::TimerHelper;
use aptos_types::{
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::Version,
    write_set::WriteSet,
};
use arr_macro::arr;

pub struct PerVersionStateUpdateRefs<'kv> {
    pub first_version: Version,
    pub num_versions: usize,
    /// Converting to Box<[]> to release over-allocated memory during construction
    pub shards: [Box<[(&'kv StateKey, StateUpdateRef<'kv>)]>; NUM_STATE_SHARDS],
}

impl<'kv> PerVersionStateUpdateRefs<'kv> {
    pub fn index_write_sets(
        first_version: Version,
        write_sets: impl IntoIterator<Item = &'kv WriteSet>,
        num_write_sets: usize,
    ) -> Self {
        Self::index(
            first_version,
            write_sets
                .into_iter()
                .map(|write_set| write_set.state_update_refs()),
            num_write_sets,
        )
    }

    pub fn index<
        UpdateIter: IntoIterator<Item = (&'kv StateKey, Option<&'kv StateValue>)>,
        VersionIter: IntoIterator<Item = UpdateIter>,
    >(
        first_version: Version,
        updates_by_version: VersionIter,
        num_versions: usize,
    ) -> Self {
        let _timer = TIMER.timer_with(&["index_state_updates__per_version"]);

        // Over-allocate a bit to minimize re-allocation.
        let mut shards = arr![Vec::with_capacity(num_versions / 8); 16];

        let mut versions_seen = 0;
        for update_iter in updates_by_version.into_iter() {
            let version = first_version + versions_seen as Version;
            versions_seen += 1;

            for (key, value) in update_iter.into_iter() {
                shards[key.get_shard_id() as usize].push((key, StateUpdateRef { version, value }));
            }
        }
        assert_eq!(versions_seen, num_versions);

        Self {
            first_version,
            shards: shards.map(|shard| shard.into_boxed_slice()),
            num_versions,
        }
    }
}

// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{metrics::TIMER, state_store::versioned_state_value::StateUpdateRef};
use aptos_metrics_core::TimerHelper;
use aptos_types::{
    state_store::{state_key::StateKey, NUM_STATE_SHARDS},
    transaction::Version,
    write_set::{BaseStateOp, WriteSet},
};
use arr_macro::arr;
use itertools::Itertools;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefMutIterator, ParallelIterator};
use std::collections::HashMap;

pub struct PerVersionStateUpdateRefs<'kv> {
    pub first_version: Version,
    pub num_versions: usize,
    /// Converting to Vec to Box<[]> to release over-allocated memory during construction
    /// TODO(HotState): let WriteOp always carry StateSlot, so we can use &'kv StateSlot here
    pub shards: [Box<[(&'kv StateKey, StateUpdateRef<'kv>)]>; NUM_STATE_SHARDS],
}

impl<'kv> PerVersionStateUpdateRefs<'kv> {
    pub fn index<
        UpdateIter: IntoIterator<Item = (&'kv StateKey, &'kv BaseStateOp)>,
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

            for (key, write_op) in update_iter.into_iter() {
                shards[key.get_shard_id()].push((key, StateUpdateRef {
                    version,
                    state_op: write_op,
                }));
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

#[derive(Clone, Debug)]
pub struct BatchedStateUpdateRefs<'kv> {
    pub first_version: Version,
    pub num_versions: usize,
    pub shards: [HashMap<&'kv StateKey, StateUpdateRef<'kv>>; NUM_STATE_SHARDS],
}

impl BatchedStateUpdateRefs<'_> {
    pub fn new_empty(first_version: Version, num_versions: usize) -> Self {
        Self {
            first_version,
            num_versions,
            shards: arr![HashMap::new(); 16],
        }
    }

    pub fn first_version(&self) -> Version {
        self.first_version
    }

    pub fn next_version(&self) -> Version {
        self.first_version + self.num_versions as Version
    }

    pub fn last_version(&self) -> Option<Version> {
        self.next_version().checked_sub(1)
    }

    pub fn len(&self) -> usize {
        self.shards.iter().map(|shard| shard.len()).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

pub struct StateUpdateRefs<'kv> {
    pub per_version: PerVersionStateUpdateRefs<'kv>,
    /// Batched updates (updates for the same keys are merged) from the
    /// beginning of the block/chunk to the last checkpoint (if it exists).
    pub for_last_checkpoint: Option<BatchedStateUpdateRefs<'kv>>,
    /// Batched updates from the version after last check point to last version
    /// (`None` if the last version is a checkpoint, e.g. in a regular block).
    pub for_latest: Option<BatchedStateUpdateRefs<'kv>>,
}

impl<'kv> StateUpdateRefs<'kv> {
    pub fn index_write_sets(
        first_version: Version,
        write_sets: impl IntoIterator<Item = &'kv WriteSet>,
        num_write_sets: usize,
        last_checkpoint_index: Option<usize>,
    ) -> Self {
        Self::index(
            first_version,
            write_sets
                .into_iter()
                .map(|write_set| write_set.base_op_iter()),
            num_write_sets,
            last_checkpoint_index,
        )
    }

    pub fn index<
        UpdateIter: IntoIterator<Item = (&'kv StateKey, &'kv BaseStateOp)>,
        VersionIter: IntoIterator<Item = UpdateIter>,
    >(
        first_version: Version,
        updates_by_version: VersionIter,
        num_versions: usize,
        last_checkpoint_index: Option<usize>,
    ) -> Self {
        let per_version =
            PerVersionStateUpdateRefs::index(first_version, updates_by_version, num_versions);

        let (for_last_checkpoint, for_latest) =
            Self::collect_updates(&per_version, last_checkpoint_index);
        Self {
            per_version,
            for_last_checkpoint,
            for_latest,
        }
    }

    pub fn last_inner_checkpoint_index(&self) -> Option<usize> {
        self.for_last_checkpoint
            .as_ref()
            .map(|updates| updates.num_versions - 1)
    }

    fn collect_updates(
        per_version_updates: &PerVersionStateUpdateRefs<'kv>,
        last_checkpoint_index: Option<usize>,
    ) -> (
        Option<BatchedStateUpdateRefs<'kv>>,
        Option<BatchedStateUpdateRefs<'kv>>,
    ) {
        let _timer = TIMER.timer_with(&["index_state_updates__collect_batch"]);

        let mut shard_iters = per_version_updates
            .shards
            .iter()
            .map(|shard| shard.iter().cloned())
            .collect::<Vec<_>>();

        let mut first_version_to_collect = per_version_updates.first_version;
        let mut remaining_versions = per_version_updates.num_versions;
        let updates_for_last_checkpoint = last_checkpoint_index.map(|idx| {
            let num_versions = idx + 1;
            let ret = Self::collect_some_updates(
                first_version_to_collect,
                num_versions,
                &mut shard_iters,
            );
            first_version_to_collect += num_versions as Version;
            remaining_versions -= num_versions;
            ret
        });
        let updates_for_latest = if remaining_versions == 0 {
            None
        } else {
            Some(Self::collect_some_updates(
                first_version_to_collect,
                remaining_versions,
                &mut shard_iters,
            ))
        };

        // Assert that all updates are consumed.
        assert!(shard_iters.iter_mut().all(|iter| iter.next().is_none()));

        (updates_for_last_checkpoint, updates_for_latest)
    }

    fn collect_some_updates(
        first_version: Version,
        num_versions: usize,
        shard_iters: &mut [impl Iterator<Item = (&'kv StateKey, StateUpdateRef<'kv>)> + Clone + Send],
    ) -> BatchedStateUpdateRefs<'kv> {
        let mut ret = BatchedStateUpdateRefs::new_empty(first_version, num_versions);
        // exclusive
        let end_version = first_version + num_versions as Version;
        shard_iters
            .par_iter_mut()
            .zip_eq(ret.shards.par_iter_mut())
            .for_each(|(shard_iter, dedupped)| {
                dedupped.extend(
                    shard_iter
                        // n.b. take_while_ref so that in the next step we can process the rest of the entries from the iters.
                        .take_while_ref(|(_k, u)| u.version < end_version),
                )
            });
        ret
    }
}

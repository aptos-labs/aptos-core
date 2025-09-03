// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{metrics::TIMER, state_store::versioned_state_value::StateUpdateRef};
use aptos_logger::{sample, sample::SampleRate, warn};
use aptos_metrics_core::TimerHelper;
use aptos_types::{
    state_store::{state_key::StateKey, NUM_STATE_SHARDS},
    transaction::Version,
    write_set::{BaseStateOp, WriteSet},
};
use arr_macro::arr;
use itertools::Itertools;
use rayon::prelude::*;
use std::{
    collections::{hash_map::Entry, HashMap},
    time::Duration,
};

#[derive(Debug)]
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
                shards[key.get_shard_id()].push((
                    key,
                    StateUpdateRef {
                        version,
                        state_op: write_op,
                    },
                ));
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

#[derive(Debug)]
pub struct StateUpdateRefs<'kv> {
    pub per_version1: Option<PerVersionStateUpdateRefs<'kv>>,
    pub per_version2: Option<PerVersionStateUpdateRefs<'kv>>,
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
        let mut updates_by_version = updates_by_version.into_iter();
        let per_version1 = last_checkpoint_index.map(|index| {
            PerVersionStateUpdateRefs::index(
                first_version,
                updates_by_version.by_ref().take(index + 1),
                index + 1,
            )
        });
        let per_version2 = match last_checkpoint_index {
            Some(num) if num == num_versions => None,
            _ => Some(PerVersionStateUpdateRefs::index(
                first_version,
                updates_by_version,
                num_versions - last_checkpoint_index.map_or(0, |index| index + 1),
            )),
        };

        let for_last_checkpoint = per_version1.as_ref().map(|pv| Self::collect_updates(pv));
        let for_latest = per_version2.as_ref().map(|pv| Self::collect_updates(pv));
        Self {
            per_version1,
            per_version2,
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
    ) -> BatchedStateUpdateRefs<'kv> {
        let _timer = TIMER.timer_with(&["index_state_updates__collect_batch"]);

        let first_version = per_version_updates.first_version;
        let num_versions = per_version_updates.num_versions;

        let mut ret = BatchedStateUpdateRefs::new_empty(first_version, num_versions);
        // exclusive
        let end_version = first_version + num_versions as Version;
        per_version_updates
            .shards
            .par_iter()
            .map(|shard| shard.iter().cloned())
            .zip_eq(ret.shards.par_iter_mut())
            .for_each(|(mut shard_iter, dedupped)| {
                // n.b. take_while_ref so that in the next step we can process the rest of the
                // entries from the iters.
                for (k, u) in shard_iter.take_while_ref(|(_k, u)| u.version < end_version) {
                    // If it's a value write op (Creation/Modification/Deletion), just insert and
                    // overwrite the previous op.
                    if u.state_op.is_value_write_op() {
                        dedupped.insert(k, u);
                        continue;
                    }

                    // If we see a hotness op, we check if there is a value write op with the same
                    // key before. This is unlikely, but if it does happen (e.g. if the write
                    // summary used to compute MakeHot is missing keys), we must discard the
                    // hotness op to avoid overwriting the value write op.
                    // TODO(HotState): also double check this logic for state sync later. For now
                    // we do not output hotness ops for state sync.
                    match dedupped.entry(k) {
                        Entry::Occupied(mut entry) => {
                            let prev_op = &entry.get().state_op;
                            sample!(
                                SampleRate::Duration(Duration::from_secs(10)),
                                warn!(
                                    "Key: {:?}. Previous write op: {}. Current write op: {}",
                                    k,
                                    prev_op.as_ref(),
                                    u.state_op.as_ref()
                                )
                            );
                            if !prev_op.is_value_write_op() {
                                entry.insert(u);
                            }
                        },
                        Entry::Vacant(entry) => {
                            entry.insert(u);
                        },
                    }
                }
            });
        ret
    }
}

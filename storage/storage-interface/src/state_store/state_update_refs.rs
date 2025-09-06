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

#[derive(Clone, Debug)]
pub struct PerVersionStateUpdateRefs<'kv> {
    pub first_version: Version,
    pub num_versions: usize,
    /// TODO(HotState): let WriteOp always carry StateSlot, so we can use &'kv StateSlot here
    /// TODO(wqfish): check if this is deterministic, i.e. if the order within one
    /// version/transaction is deterministic.
    /// Note(wqfish): this is the flattened write sets.
    pub shards: [Vec<(&'kv StateKey, StateUpdateRef<'kv>)>; NUM_STATE_SHARDS],
}

impl<'kv> PerVersionStateUpdateRefs<'kv> {
    fn new_empty(first_version: Version) -> Self {
        Self {
            first_version,
            num_versions: 0,
            shards: arr![Vec::new(); 16],
        }
    }

    fn index<
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
            shards: shards.map(|mut shard| {
                // Release over-allocated memory during construction.
                shard.shrink_to_fit();
                shard
            }),
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

pub fn batched_updates_to_debug_str<'kv>(
    shard: &HashMap<&'kv StateKey, StateUpdateRef<'kv>>,
) -> String {
    let mut out = "\n".to_string();
    for (key, update) in shard {
        out += &format!("\t{:?}: {:?}\n", key, update);
    }
    out
}

impl BatchedStateUpdateRefs<'_> {
    fn new_empty(first_version: Version, num_versions: usize) -> Self {
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

    #[cfg(test)]
    fn get(&self, key: &str) -> Option<&StateUpdateRef> {
        let state_key = StateKey::raw(key.as_bytes());
        let shard_id = state_key.get_shard_id();
        self.shards[shard_id].get(&state_key)
    }
}

#[derive(Debug)]
pub struct StateUpdateRefs<'kv> {
    pub per_version: PerVersionStateUpdateRefs<'kv>,
    /// Updates from the beginning of the block/chunk to the last checkpoint (if it exists).
    for_last_checkpoint: Option<(PerVersionStateUpdateRefs<'kv>, BatchedStateUpdateRefs<'kv>)>,
    /// Updates from the version after last checkpoint to last version (`None` if the last version
    /// is a checkpoint, e.g. in a regular block).
    for_latest: Option<(PerVersionStateUpdateRefs<'kv>, BatchedStateUpdateRefs<'kv>)>,
}

impl<'kv> StateUpdateRefs<'kv> {
    pub(crate) fn for_last_checkpoint_per_version(&self) -> Option<&PerVersionStateUpdateRefs> {
        self.for_last_checkpoint.as_ref().map(|x| &x.0)
    }

    pub(crate) fn for_last_checkpoint_batched(&self) -> Option<&BatchedStateUpdateRefs> {
        self.for_last_checkpoint.as_ref().map(|x| &x.1)
    }

    pub(crate) fn for_latest_per_version(&self) -> Option<&PerVersionStateUpdateRefs> {
        self.for_latest.as_ref().map(|x| &x.0)
    }

    pub(crate) fn for_latest_batched(&self) -> Option<&BatchedStateUpdateRefs> {
        self.for_latest.as_ref().map(|x| &x.1)
    }

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
        if num_versions == 0 {
            return Self {
                per_version: PerVersionStateUpdateRefs::new_empty(first_version),
                for_last_checkpoint: None,
                for_latest: None,
            };
        }

        let mut updates_by_version = updates_by_version.into_iter();
        let mut num_versions_for_last_checkpoint = 0;

        let for_last_checkpoint = last_checkpoint_index.map(|index| {
            num_versions_for_last_checkpoint = index + 1;
            let per_version = PerVersionStateUpdateRefs::index(
                first_version,
                updates_by_version
                    .by_ref()
                    .take(num_versions_for_last_checkpoint),
                num_versions_for_last_checkpoint,
            );
            let batched = Self::batch_updates(&per_version);
            (per_version, batched)
        });

        let for_latest = match last_checkpoint_index {
            Some(index) if index + 1 == num_versions => None,
            _ => {
                assert!(num_versions_for_last_checkpoint < num_versions);
                let per_version = PerVersionStateUpdateRefs::index(
                    first_version + num_versions_for_last_checkpoint as Version,
                    updates_by_version,
                    num_versions - num_versions_for_last_checkpoint,
                );
                let batched = Self::batch_updates(&per_version);
                Some((per_version, batched))
            },
        };

        Self {
            per_version: Self::concat_per_version_updates(
                for_last_checkpoint.as_ref().map(|x| &x.0),
                for_latest.as_ref().map(|x| &x.0),
            ),
            for_last_checkpoint,
            for_latest,
        }
    }

    fn concat_per_version_updates(
        for_last_checkpoint: Option<&PerVersionStateUpdateRefs<'kv>>,
        for_latest: Option<&PerVersionStateUpdateRefs<'kv>>,
    ) -> PerVersionStateUpdateRefs<'kv> {
        match for_last_checkpoint {
            Some(for_last_checkpoint) => {
                let mut all = for_last_checkpoint.clone();
                if let Some(for_latest) = for_latest {
                    all.num_versions += for_latest.num_versions;
                    for (dest, src) in all.shards.iter_mut().zip_eq(for_latest.shards.iter()) {
                        dest.extend_from_slice(src);
                    }
                }
                all
            },
            None => for_latest.cloned().expect("At least one should be Some."),
        }
    }

    pub fn last_inner_checkpoint_index(&self) -> Option<usize> {
        self.for_last_checkpoint.as_ref().map(|updates| {
            assert_eq!(updates.0.num_versions, updates.1.num_versions);
            updates.0.num_versions - 1
        })
    }

    fn batch_updates(
        per_version_updates: &PerVersionStateUpdateRefs<'kv>,
    ) -> BatchedStateUpdateRefs<'kv> {
        let _timer = TIMER.timer_with(&["index_state_updates__collect_batch"]);

        let mut ret = BatchedStateUpdateRefs::new_empty(
            per_version_updates.first_version,
            per_version_updates.num_versions,
        );
        per_version_updates
            .shards
            .par_iter()
            .map(|shard| shard.iter().cloned())
            .zip_eq(ret.shards.par_iter_mut())
            .for_each(|(shard_iter, dedupped)| {
                // TODO(wqfish): this is fine for now.
                // This is used to compute SMT and JMT, so we can simply discard all the hotness
                // ops.
                // Revisit when we compute the hot state root hash.
                dedupped.extend(shard_iter.filter(|(_k, u)| u.state_op.is_value_write_op()));
            });
        ret
    }
}

#[cfg(test)]
mod tests {
    use super::{BatchedStateUpdateRefs, StateUpdateRefs};
    use aptos_types::{
        state_store::{state_key::StateKey, state_value::StateValueMetadata},
        transaction::Version,
        write_set::{WriteOp, WriteSet, WriteSetMut},
    };
    use bytes::Bytes;

    fn write_set(kvs: &[(&str, &str)]) -> WriteSet {
        WriteSetMut::from_iter(kvs.iter().map(|(key, value)| {
            let key = StateKey::raw(key.as_bytes());
            let value = Bytes::copy_from_slice(value.as_bytes());
            let op = WriteOp::modification(value, StateValueMetadata::none());
            (key, op)
        }))
        .freeze()
        .unwrap()
    }

    fn verify_batching(
        res: &BatchedStateUpdateRefs,
        key: &str,
        expected_version: Version,
        expected_value: &str,
    ) {
        let u = res.get(key).unwrap();
        assert_eq!(u.version, expected_version);
        assert_eq!(
            u.state_op.as_state_value_opt().unwrap().bytes(),
            expected_value
        );
    }

    #[test]
    fn test_regular_block() {
        // A regular block, or a chunk that ends exactly at a block boundary.
        let v0 = write_set(&[("A", "A0")]);
        let v1 = write_set(&[("A", "A1"), ("B", "B1")]);
        let v2 = write_set(&[("C", "C2")]);
        let last_checkpoint_index = Some(2);
        let ret =
            StateUpdateRefs::index_write_sets(0, vec![&v0, &v1, &v2], 3, last_checkpoint_index);

        let for_last_checkpoint = ret.for_last_checkpoint_batched().unwrap();
        assert_eq!(for_last_checkpoint.first_version, 0);
        assert_eq!(for_last_checkpoint.num_versions, 3);
        verify_batching(for_last_checkpoint, "A", 1, "A1");
        verify_batching(for_last_checkpoint, "B", 1, "B1");
        verify_batching(for_last_checkpoint, "C", 2, "C2");

        assert!(ret.for_latest_batched().is_none());
    }

    #[test]
    fn test_chunk_with_checkpoint() {
        // For example, the second half of a previous block and the first half of the current
        // block.
        let v0 = write_set(&[("A", "A0")]);
        let v1 = write_set(&[("A", "A1"), ("B", "B1")]);
        let v2 = write_set(&[("A", "A2"), ("B", "B2")]);
        let v3 = write_set(&[("B", "B3"), ("C", "C3")]);
        let last_checkpoint_index = Some(1);
        let ret = StateUpdateRefs::index_write_sets(
            0,
            vec![&v0, &v1, &v2, &v3],
            4,
            last_checkpoint_index,
        );

        let for_last_checkpoint = ret.for_last_checkpoint_batched().unwrap();
        assert_eq!(for_last_checkpoint.first_version, 0);
        assert_eq!(for_last_checkpoint.num_versions, 2);
        verify_batching(for_last_checkpoint, "A", 1, "A1");
        verify_batching(for_last_checkpoint, "B", 1, "B1");

        let for_latest = ret.for_latest_batched().unwrap();
        assert_eq!(for_latest.first_version, 2);
        assert_eq!(for_latest.num_versions, 2);
        verify_batching(for_latest, "A", 2, "A2");
        verify_batching(for_latest, "B", 3, "B3");
        verify_batching(for_latest, "C", 3, "C3");
    }

    #[test]
    fn test_chunk_with_no_checkpoint() {
        // A chunk that is the middle of a large block.
        let v0 = write_set(&[("A", "A0"), ("B", "B0")]);
        let v1 = write_set(&[("A", "A1")]);
        let v2 = write_set(&[("C", "C2")]);
        let last_checkpoint = None;
        let ret = StateUpdateRefs::index_write_sets(10, vec![&v0, &v1, &v2], 3, last_checkpoint);

        assert!(ret.for_last_checkpoint_batched().is_none());

        let for_latest = ret.for_latest_batched().unwrap();
        assert_eq!(for_latest.first_version, 10);
        assert_eq!(for_latest.num_versions, 3);
        verify_batching(for_latest, "A", 11, "A1");
        verify_batching(for_latest, "B", 10, "B0");
        verify_batching(for_latest, "C", 12, "C2");
    }
}

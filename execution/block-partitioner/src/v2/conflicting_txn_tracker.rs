// Copyright © Aptos Foundation

use crate::v2::{OriginalTxnIdx, ShardedTxnIndex2};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::collections::btree_set::BTreeSet;
use std::fmt::{Display, Formatter};
use itertools::Itertools;
use aptos_types::block_executor::partitioner::{RoundId, ShardId};
use aptos_types::state_store::state_key::StateKey;
use aptos_types::transaction::analyzed_transaction::StorageLocation;

/// This structure is only used in `V2Partitioner`.
/// For txns that claimed to access the same storage location,
/// it caches some metadata about the location and also keeps track of their status (pending or position finalized) throughout the partitioning process.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ConflictingTxnTracker {
    pub storage_location: StorageLocation,
    pub anchor_shard_id: ShardId,
    pending_reads: BTreeSet<OriginalTxnIdx>,
    pending_writes: BTreeSet<OriginalTxnIdx>,
    pub writer_set: HashSet<OriginalTxnIdx>,
    pub finalized_all: BTreeSet<ShardedTxnIndex2>,
    pub finalized_writes: BTreeSet<ShardedTxnIndex2>,
}

impl ConflictingTxnTracker {
    pub fn new(storage_location: StorageLocation, anchor_shard_id: ShardId) -> Self {
        Self {
            storage_location,
            anchor_shard_id,
            pending_reads: Default::default(),
            pending_writes: Default::default(),
            writer_set: Default::default(),
            finalized_all: Default::default(),
            finalized_writes: Default::default(),
        }
    }

    pub fn add_candidate(&mut self, txn_id: OriginalTxnIdx, is_write: bool) {
        if is_write {
            self.pending_writes.insert(txn_id);
            self.writer_set.insert(txn_id);
        } else {
            self.pending_reads.insert(txn_id);
        }
    }

    /// Partitioner has finalized the position of a txn. Remove it from the pending txn list.
    pub fn mark_txn_ordered(&mut self, txn_id: OriginalTxnIdx, round_id: RoundId, shard_id: ShardId) {
        let txn_fat_id = ShardedTxnIndex2 {
            round_id,
            shard_id,
            ori_txn_idx: txn_id,
        };
        if self.pending_writes.remove(&txn_id) {
            self.finalized_writes.insert(txn_fat_id);
        } else {
            assert!(self.pending_reads.remove(&txn_id));
        }
        self.finalized_all.insert(txn_fat_id);

    }

    /// Check if there is a txn writing to the current storage location and its txn_id in the given range.
    /// The txn list is considered a ring, and the range can be wrapped.
    /// Below are some examples.
    ///
    /// candidates=[T4(W), T7(W), T8(R), T9(W), T11(W)], start=3, end=4 => false
    /// candidates=[T4(W), T7(W), T8(R), T9(W), T11(W)], start=3, end=5 => true
    /// candidates=[T4(W), T7(W), T8(R), T9(W), T11(W)], start=3, end=6 => true
    /// candidates=[T4(W), T7(W), T8(R), T9(W), T11(W)], start=4, end=4 => false
    /// candidates=[T4(W), T7(W), T8(R), T9(W), T11(W)], start=4, end=6 => true
    /// candidates=[T4(W), T7(W), T8(R), T9(W), T11(W)], start=5, end=6 => false
    /// candidates=[T4(W), T7(W), T8(R), T9(W), T11(W)], start=7, end=9 => true
    /// candidates=[T4(W), T7(W), T8(R), T9(W), T11(W)], start=8, end=9 => false
    /// candidates=[T4(W), T7(W), T8(R), T9(W), T11(W)], start=12, end=4 => false
    /// candidates=[T4(W), T7(W), T8(R), T9(W), T11(W)], start=12, end=5 => true
    pub fn has_write_in_range(&self, start_txn_id: OriginalTxnIdx, end_txn_id: OriginalTxnIdx) -> bool {
        if start_txn_id <= end_txn_id {
            self.pending_writes.range(start_txn_id..end_txn_id).next().is_some()
        } else {
            self.pending_writes.range(start_txn_id..).next().is_some() || self.pending_writes.range(..end_txn_id).next().is_some()
        }
    }

    #[cfg(test)]
    pub fn is_writer(&self, old_txn_id: usize) -> bool {
        self.writer_set.contains(&old_txn_id)
    }

    pub fn brief(&self) -> String {
        let candidates: BTreeSet<(usize, bool)> = BTreeSet::from_iter(self.pending_reads.iter().map(|t|(*t, false)).chain(self.pending_writes.iter().map(|t|(*t, true))));
        let candidate_strs: Vec<String> = candidates.into_iter().map(|(txn_id, is_write)|{
            let flag = if is_write {"W"} else {"R"};
            format!("{txn_id}({flag})")
        }).collect();
        let candidates_str = candidate_strs.join(",");
        let promoteds: Vec<(ShardedTxnIndex2, bool)> = self.finalized_all.iter().map(|fat_id|(*fat_id, self.writer_set.contains(&fat_id.ori_txn_idx))).collect();
        let promoted_strs: Vec<String> = promoteds.into_iter().map(|(fat_id, is_write)|{
            let flag = if is_write {"W"} else {"R"};
            format!("({},{})/{}({})", fat_id.round_id, fat_id.shard_id, fat_id.ori_txn_idx, flag)
        }).collect();
        let promoteds_str = promoted_strs.join(",");
        format!("{{anchor={}, candidates=[{}], promoted=[{}]}}", self.anchor_shard_id, candidates_str, promoteds_str)
    }
}

#[test]
fn test_storage_location_helper() {
    let mut helper = ConflictingTxnTracker::new(StorageLocation::Specific(StateKey::raw(vec![])), 0);
    helper.add_candidate(4, true);
    helper.add_candidate(10, true);
    helper.add_candidate(7, true);
    helper.add_candidate(8, false);
    helper.add_candidate(9, true);
    // candidates: T4(W), T7(W), T8(R), T9(W), T10(W)
    // promoted: -
    assert!(!helper.has_write_in_range(4, 4)); // 0-length interval
    assert!(helper.has_write_in_range(4, 5)); // 0-length interval
    assert!(helper.has_write_in_range(5, 10));
    assert!(!helper.has_write_in_range(8, 9));
    assert!(helper.has_write_in_range(11, 5)); // wrapped range
    assert!(!helper.has_write_in_range(11, 4)); // wrapped range
    helper.mark_txn_ordered(9, 99, 10);
    // candidates: T4(W), T7(W), T8(R), T10(W)
    // promoted: (99,10)/T9(W)
    assert!(helper.has_write_in_range(5, 10));
    helper.mark_txn_ordered(7, 99, 20);
    // candidates: T4(W), T8(R), T10(W)
    // promoted: (99,10)/T9(W), (99,20)/T7(W)
    assert!(!helper.has_write_in_range(5, 10));
    helper.mark_txn_ordered(4, 99, 20);
    helper.mark_txn_ordered(8, 99, 30);
    helper.mark_txn_ordered(10, 99, 30);
    // candidates: -
    // promoted: (99,10)/T9(W), (99,20)/T4(W), (99,20)/T7(W), (99,30)/T8(R), (99,30)/T10(W)
    assert_eq!(
        vec![ShardedTxnIndex2::new(99, 10, 9), ShardedTxnIndex2::new(99, 20, 4), ShardedTxnIndex2::new(99, 20, 7)],
        helper.finalized_all.range(ShardedTxnIndex2::new(98, 0, 0)..ShardedTxnIndex2::new(99, 20, 8)).map(|fat_id|*fat_id).collect::<Vec<_>>()
    );
    assert_eq!(
        vec![ShardedTxnIndex2::new(99, 20, 7), ShardedTxnIndex2::new(99, 30, 8), ShardedTxnIndex2::new(99, 30, 10)],
        helper.finalized_all.range(ShardedTxnIndex2::new(99, 20, 7)..).map(|fat_id|*fat_id).collect::<Vec<_>>()
    );
    assert_eq!(
        vec![ShardedTxnIndex2::new(99, 20, 7), ShardedTxnIndex2::new(99, 30, 10)],
        helper.finalized_writes.range(ShardedTxnIndex2::new(99, 20, 7)..ShardedTxnIndex2::new(99, 40, 0)).map(|fat_id|*fat_id).collect::<Vec<_>>()
    );
}

// Copyright © Aptos Foundation

use crate::v2::{OriginalTxnIdx, ShardedTxnIndex2};
#[cfg(test)]
use aptos_types::state_store::state_key::StateKey;
use aptos_types::{
    block_executor::partitioner::{RoundId, ShardId},
    transaction::analyzed_transaction::StorageLocation,
};
use serde::{Deserialize, Serialize};
use std::collections::{btree_set::BTreeSet, HashSet};

/// This structure is only used in `V2Partitioner`.
/// For txns that claimed to access the same storage location,
/// it caches some metadata about the location and also keeps track of their status (pending or position finalized) throughout the partitioning process.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ConflictingTxnTracker {
    /// The storage location on which conflicting txns are being tracked by this tracker.
    pub storage_location: StorageLocation,
    /// A randomly chosen owner shard of the storage location, for conflict resolution purpose.
    pub anchor_shard_id: ShardId,
    /// Txns that (1) read the current storage location and (2) have not been accepted.
    pending_reads: BTreeSet<OriginalTxnIdx>,
    /// Txns that (1) write the current storage location and (2) have not been accepted.
    pending_writes: BTreeSet<OriginalTxnIdx>,
    /// Txns that write the current storage location.
    pub writer_set: HashSet<OriginalTxnIdx>,
    /// Txns that have been accepted.
    pub finalized_all: BTreeSet<ShardedTxnIndex2>,
    /// Txns that (1) write the current storage location and (2) have been accepted.
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

    pub fn add_read_candidate(&mut self, txn_id: OriginalTxnIdx) {
        self.pending_reads.insert(txn_id);
    }

    pub fn add_write_candidate(&mut self, txn_id: OriginalTxnIdx) {
        self.pending_writes.insert(txn_id);
        self.writer_set.insert(txn_id);
    }

    /// Partitioner has finalized the position of a txn. Remove it from the pending txn list.
    pub fn mark_txn_ordered(
        &mut self,
        txn_id: OriginalTxnIdx,
        round_id: RoundId,
        shard_id: ShardId,
    ) {
        let sharded_txn_idx = ShardedTxnIndex2::new(round_id, shard_id,txn_id);
        if self.pending_writes.remove(&txn_id) {
            self.finalized_writes.insert(sharded_txn_idx);
        } else {
            assert!(self.pending_reads.remove(&txn_id));
        }
        self.finalized_all.insert(sharded_txn_idx);
    }

    /// Check if there is a txn writing to the current storage location and its txn_id in the given wrapped range [start, end).
    pub fn has_write_in_range(
        &self,
        start_txn_id: OriginalTxnIdx,
        end_txn_id: OriginalTxnIdx,
    ) -> bool {
        if start_txn_id <= end_txn_id {
            self.pending_writes
                .range(start_txn_id..end_txn_id)
                .next()
                .is_some()
        } else {
            self.pending_writes.range(start_txn_id..).next().is_some()
                || self.pending_writes.range(..end_txn_id).next().is_some()
        }
    }
}

#[test]
fn test_conflicting_txn_tracker() {
    let mut tracker =
        ConflictingTxnTracker::new(StorageLocation::Specific(StateKey::raw(vec![])), 0);
    tracker.add_write_candidate(4);
    tracker.add_write_candidate(10);
    tracker.add_write_candidate(7);
    tracker.add_read_candidate(8);
    tracker.add_write_candidate(9);
    // candidates: T4(W), T7(W), T8(R), T9(W), T10(W)
    // promoted: -
    assert!(!tracker.has_write_in_range(4, 4)); // 0-length interval
    assert!(tracker.has_write_in_range(4, 5)); // 0-length interval
    assert!(tracker.has_write_in_range(5, 10));
    assert!(!tracker.has_write_in_range(8, 9));
    assert!(tracker.has_write_in_range(11, 5)); // wrapped range
    assert!(!tracker.has_write_in_range(11, 4)); // wrapped range
    tracker.mark_txn_ordered(9, 99, 10);
    // candidates: T4(W), T7(W), T8(R), T10(W)
    // promoted: (99,10)/T9(W)
    assert!(tracker.has_write_in_range(5, 10));
    tracker.mark_txn_ordered(7, 99, 20);
    // candidates: T4(W), T8(R), T10(W)
    // promoted: (99,10)/T9(W), (99,20)/T7(W)
    assert!(!tracker.has_write_in_range(5, 10));
    tracker.mark_txn_ordered(4, 99, 20);
    tracker.mark_txn_ordered(8, 99, 30);
    tracker.mark_txn_ordered(10, 99, 30);
    // candidates: -
    // promoted: (99,10)/T9(W), (99,20)/T4(W), (99,20)/T7(W), (99,30)/T8(R), (99,30)/T10(W)
    assert_eq!(
        vec![
            ShardedTxnIndex2::new(99, 10, 9),
            ShardedTxnIndex2::new(99, 20, 4),
            ShardedTxnIndex2::new(99, 20, 7)
        ],
        tracker
            .finalized_all
            .range(ShardedTxnIndex2::new(98, 0, 0)..ShardedTxnIndex2::new(99, 20, 8))
            .copied()
            .collect::<Vec<_>>()
    );
    assert_eq!(
        vec![
            ShardedTxnIndex2::new(99, 20, 7),
            ShardedTxnIndex2::new(99, 30, 8),
            ShardedTxnIndex2::new(99, 30, 10)
        ],
        tracker
            .finalized_all
            .range(ShardedTxnIndex2::new(99, 20, 7)..)
            .copied()
            .collect::<Vec<_>>()
    );
    assert_eq!(
        vec![
            ShardedTxnIndex2::new(99, 20, 7),
            ShardedTxnIndex2::new(99, 30, 10)
        ],
        tracker
            .finalized_writes
            .range(ShardedTxnIndex2::new(99, 20, 7)..ShardedTxnIndex2::new(99, 40, 0))
            .copied()
            .collect::<Vec<_>>()
    );
}

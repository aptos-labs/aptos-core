// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::v2::types::{PrePartitionedTxnIdx, ShardedTxnIndexV2};
#[cfg(test)]
use velor_types::state_store::state_key::StateKey;
use velor_types::{
    block_executor::partitioner::{RoundId, ShardId},
    transaction::analyzed_transaction::StorageLocation,
};
use serde::{Deserialize, Serialize};
use std::collections::btree_set::BTreeSet;

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
    pending_reads: BTreeSet<PrePartitionedTxnIdx>,
    /// Txns that (1) write the current storage location and (2) have not been accepted.
    pending_writes: BTreeSet<PrePartitionedTxnIdx>,
    /// Txns that have been accepted.
    pub finalized: BTreeSet<ShardedTxnIndexV2>,
    /// Txns that (1) write the current storage location and (2) have been accepted.
    pub finalized_writes: BTreeSet<ShardedTxnIndexV2>,
}

impl ConflictingTxnTracker {
    pub fn new(storage_location: StorageLocation, anchor_shard_id: ShardId) -> Self {
        Self {
            storage_location,
            anchor_shard_id,
            pending_reads: Default::default(),
            pending_writes: Default::default(),
            finalized: Default::default(),
            finalized_writes: Default::default(),
        }
    }

    pub fn add_read_candidate(&mut self, txn_id: PrePartitionedTxnIdx) {
        self.pending_reads.insert(txn_id);
    }

    pub fn add_write_candidate(&mut self, txn_id: PrePartitionedTxnIdx) {
        self.pending_writes.insert(txn_id);
    }

    /// Partitioner has finalized the position of a txn. Remove it from the pending txn list.
    pub fn mark_txn_ordered(
        &mut self,
        txn_id: PrePartitionedTxnIdx,
        round_id: RoundId,
        shard_id: ShardId,
    ) {
        let sharded_txn_idx = ShardedTxnIndexV2::new(round_id, shard_id, txn_id);
        if self.pending_writes.remove(&txn_id) {
            self.finalized_writes.insert(sharded_txn_idx);
        } else {
            assert!(self.pending_reads.remove(&txn_id));
        }
        self.finalized.insert(sharded_txn_idx);
    }

    /// Check if there is a txn writing to the current storage location and its txn_id in the given wrapped range [start, end).
    pub fn has_write_in_range(
        &self,
        start_txn_id: PrePartitionedTxnIdx,
        end_txn_id: PrePartitionedTxnIdx,
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
    let mut tracker = ConflictingTxnTracker::new(StorageLocation::Specific(StateKey::raw(&[])), 0);
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
            ShardedTxnIndexV2::new(99, 10, 9),
            ShardedTxnIndexV2::new(99, 20, 4),
            ShardedTxnIndexV2::new(99, 20, 7)
        ],
        tracker
            .finalized
            .range(ShardedTxnIndexV2::new(98, 0, 0)..ShardedTxnIndexV2::new(99, 20, 8))
            .copied()
            .collect::<Vec<_>>()
    );
    assert_eq!(
        vec![
            ShardedTxnIndexV2::new(99, 20, 7),
            ShardedTxnIndexV2::new(99, 30, 8),
            ShardedTxnIndexV2::new(99, 30, 10)
        ],
        tracker
            .finalized
            .range(ShardedTxnIndexV2::new(99, 20, 7)..)
            .copied()
            .collect::<Vec<_>>()
    );
    assert_eq!(
        vec![
            ShardedTxnIndexV2::new(99, 20, 7),
            ShardedTxnIndexV2::new(99, 30, 10)
        ],
        tracker
            .finalized_writes
            .range(ShardedTxnIndexV2::new(99, 20, 7)..ShardedTxnIndexV2::new(99, 40, 0))
            .copied()
            .collect::<Vec<_>>()
    );
}

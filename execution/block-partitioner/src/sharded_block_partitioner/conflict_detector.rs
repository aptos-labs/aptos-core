// Copyright © Aptos Foundation

use crate::sharded_block_partitioner::dependency_analysis::{RWSet, WriteSetWithTxnIndex};
use aptos_types::{
    block_executor::partitioner::{
        CrossShardDependencies, RoundId, ShardId, ShardedTxnIndex, SubBlock,
        TransactionWithDependencies, TxnIndex,
    },
    transaction::{
        analyzed_transaction::{AnalyzedTransaction, StorageLocation},
        Transaction,
    },
};
use std::{
    collections::{hash_map::DefaultHasher, HashSet},
    hash::{Hash, Hasher},
    sync::Arc,
};

pub struct CrossShardConflictDetector {
    shard_id: ShardId,
    num_shards: usize,
    round_id: RoundId,
}

impl CrossShardConflictDetector {
    pub fn new(shard_id: ShardId, num_shards: usize, round_id: RoundId) -> Self {
        Self {
            shard_id,
            num_shards,
            round_id,
        }
    }

    pub fn discard_txns_with_cross_shard_deps(
        &mut self,
        txns: Vec<AnalyzedTransaction>,
        cross_shard_rw_set: &[RWSet],
        prev_rounds_rw_set_with_index: Arc<Vec<WriteSetWithTxnIndex>>,
    ) -> (
        Vec<AnalyzedTransaction>,
        Vec<CrossShardDependencies>,
        Vec<AnalyzedTransaction>,
    ) {
        // Iterate through all the transactions and if any shard has taken read/write lock on the storage location
        // and has a higher priority than this shard id, then this transaction needs to be moved to the end of the block.
        let mut accepted_txns = Vec::new();
        let mut accepted_txn_dependencies = Vec::new();
        let mut rejected_txns = Vec::new();
        let mut discarded_senders = HashSet::new();
        for (_, txn) in txns.into_iter().enumerate() {
            let sender_was_discarded = txn
                .sender()
                .map_or(false, |sender| discarded_senders.contains(&sender));
            if sender_was_discarded
                || self.check_for_cross_shard_conflict(self.shard_id, &txn, cross_shard_rw_set)
            {
                if let Some(sender) = txn.sender() {
                    discarded_senders.insert(sender);
                }
                rejected_txns.push(txn);
            } else {
                let cross_shard_deps = if self.round_id == 0 {
                    // 1st-round txns always have 0 cross-shard dependencies.
                    CrossShardDependencies::default()
                } else {
                    self.get_deps_for_frozen_txn(
                        &txn,
                        Arc::new(vec![WriteSetWithTxnIndex::default(); self.num_shards]),
                        prev_rounds_rw_set_with_index.clone(),
                    )
                };
                accepted_txn_dependencies.push(cross_shard_deps);
                accepted_txns.push(txn);
            }
        }
        (accepted_txns, accepted_txn_dependencies, rejected_txns)
    }

    /// Adds a cross shard dependency for a transaction. This can be done by finding the maximum transaction index
    /// that has taken a read/write lock on the storage location the current transaction is trying to read/write.
    /// We traverse the current round read/write set in reverse order starting from shard id -1 and look for the first
    /// txn index that has taken a read/write lock on the storage location. If we can't find any such txn index, we
    /// traverse the previous rounds read/write set in reverse order and look for the first txn index that has taken
    /// a read/write lock on the storage location.
    fn get_deps_for_frozen_txn(
        &self,
        frozen_txn: &AnalyzedTransaction,
        current_round_rw_set_with_index: Arc<Vec<WriteSetWithTxnIndex>>,
        prev_rounds_rw_set_with_index: Arc<Vec<WriteSetWithTxnIndex>>,
    ) -> CrossShardDependencies {
        assert_eq!(self.num_shards, current_round_rw_set_with_index.len());
        assert_eq!(0, current_round_rw_set_with_index.len() % self.num_shards);
        // Iterate through the frozen dependencies and add the max transaction index for each storage location
        let mut cross_shard_dependencies = CrossShardDependencies::default();
        for storage_location in frozen_txn
            .read_hints()
            .iter()
            .chain(frozen_txn.write_hints().iter())
        {
            // For current round, iterate through all shards less than current shards in the reverse order and for previous rounds iterate through all shards in the reverse order
            // and find the first shard id that has taken a write lock on the storage location. This ensures that we find the highest txn index that is conflicting
            // with the current transaction. Please note that since we use a multi-version database, there is no conflict if any previous txn index has taken
            // a read lock on the storage location.
            let mut current_shard_id = self.shard_id;
            let mut current_round = self.round_id;
            for rw_set_with_index in current_round_rw_set_with_index
                .iter()
                .take(self.shard_id)
                .rev()
                .chain(prev_rounds_rw_set_with_index.iter().rev())
            {
                // Move the cursor backward.
                if current_shard_id == 0 {
                    current_round -= 1;
                    current_shard_id = self.num_shards - 1;
                } else {
                    current_shard_id -= 1;
                };

                if rw_set_with_index.has_write_lock(storage_location) {
                    cross_shard_dependencies.add_required_edge(
                        ShardedTxnIndex::new(
                            rw_set_with_index.get_write_lock_txn_index(storage_location),
                            current_shard_id,
                            current_round,
                        ),
                        storage_location.clone(),
                    );
                    break;
                }
            }
        }

        cross_shard_dependencies
    }

    pub fn add_deps_for_frozen_sub_block(
        &self,
        txns: Vec<AnalyzedTransaction>,
        current_round_rw_set_with_index: Arc<Vec<WriteSetWithTxnIndex>>,
        prev_round_rw_set_with_index: Arc<Vec<WriteSetWithTxnIndex>>,
        index_offset: TxnIndex,
    ) -> (SubBlock<Transaction>, Vec<CrossShardDependencies>) {
        let mut frozen_txns = Vec::new();
        let mut cross_shard_dependencies = Vec::new();
        for txn in txns.into_iter() {
            let dependency = self.get_deps_for_frozen_txn(
                &txn,
                current_round_rw_set_with_index.clone(),
                prev_round_rw_set_with_index.clone(),
            );
            cross_shard_dependencies.push(dependency.clone());
            frozen_txns.push(TransactionWithDependencies::new(txn.into_txn(), dependency));
        }
        (
            SubBlock::new(index_offset, frozen_txns),
            cross_shard_dependencies,
        )
    }

    fn check_for_cross_shard_conflict(
        &self,
        current_shard_id: ShardId,
        txn: &AnalyzedTransaction,
        cross_shard_rw_set: &[RWSet],
    ) -> bool {
        if self.check_for_read_conflict(current_shard_id, txn, cross_shard_rw_set) {
            return true;
        }
        if self.check_for_write_conflict(current_shard_id, txn, cross_shard_rw_set) {
            return true;
        }
        false
    }

    fn get_anchor_shard_id(&self, storage_location: &StorageLocation) -> ShardId {
        let mut hasher = DefaultHasher::new();
        storage_location.hash(&mut hasher);
        (hasher.finish() % self.num_shards as u64) as usize
    }

    fn check_for_read_conflict(
        &self,
        current_shard_id: ShardId,
        txn: &AnalyzedTransaction,
        cross_shard_rw_set: &[RWSet],
    ) -> bool {
        for read_location in txn.read_hints().iter() {
            // Each storage location is allocated an anchor shard id, which is used to conflict resolution deterministically across shards.
            // During conflict resolution, shards starts scanning from the anchor shard id and
            // first shard id that has taken a read/write lock on this storage location is the owner of this storage location.
            // Please note another alternative is scan from first shard id, but this will result in non-uniform load across shards in case of conflicts.
            let anchor_shard_id = self.get_anchor_shard_id(read_location);
            for offset in 0..self.num_shards {
                let shard_id = (anchor_shard_id + offset) % self.num_shards;
                // Ignore if this is from the same shard
                if shard_id == current_shard_id {
                    // We only need to check if any shard id < current shard id has taken a write lock on the storage location
                    break;
                }
                if cross_shard_rw_set[shard_id].has_write_lock(read_location) {
                    return true;
                }
            }
        }
        false
    }

    fn check_for_write_conflict(
        &self,
        current_shard_id: usize,
        txn: &AnalyzedTransaction,
        cross_shard_rw_set: &[RWSet],
    ) -> bool {
        for write_location in txn.write_hints().iter() {
            let anchor_shard_id = self.get_anchor_shard_id(write_location);
            for offset in 0..self.num_shards {
                let shard_id = (anchor_shard_id + offset) % self.num_shards;
                // Ignore if this is from the same shard
                if shard_id == current_shard_id {
                    // We only need to check if any shard id < current shard id has taken a write lock on the storage location
                    break;
                }
                if cross_shard_rw_set[shard_id].has_read_or_write_lock(write_location) {
                    return true;
                }
            }
        }
        false
    }
}

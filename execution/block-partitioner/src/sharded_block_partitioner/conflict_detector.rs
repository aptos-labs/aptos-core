// Copyright © Aptos Foundation

use crate::{
    sharded_block_partitioner::dependency_analysis::{RWSet, RWSetWithTxnIndex},
    types::{
        CrossShardDependencies, ShardId, TransactionWithDependencies, TransactionsChunk, TxnIndex,
    },
};
use aptos_crypto::hash::CryptoHash;
use aptos_types::transaction::analyzed_transaction::AnalyzedTransaction;
use std::sync::Arc;

pub struct CrossShardConflictDetector {
    shard_id: ShardId,
    num_shards: usize,
}

impl CrossShardConflictDetector {
    pub fn new(shard_id: ShardId, num_shards: usize) -> Self {
        Self {
            shard_id,
            num_shards,
        }
    }

    pub fn discard_txns_with_cross_shard_deps(
        &mut self,
        txns: Vec<AnalyzedTransaction>,
        cross_shard_rw_set: &[RWSet],
        prev_rounds_rw_set_with_index: Arc<Vec<RWSetWithTxnIndex>>,
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
        for (_, txn) in txns.into_iter().enumerate() {
            if self.check_for_cross_shard_conflict(self.shard_id, &txn, cross_shard_rw_set) {
                rejected_txns.push(txn);
            } else {
                accepted_txn_dependencies.push(self.get_dependencies_for_frozen_txn(
                    &txn,
                    Arc::new(vec![]),
                    prev_rounds_rw_set_with_index.clone(),
                ));
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
    fn get_dependencies_for_frozen_txn(
        &self,
        frozen_txn: &AnalyzedTransaction,
        current_round_rw_set_with_index: Arc<Vec<RWSetWithTxnIndex>>,
        prev_rounds_rw_set_with_index: Arc<Vec<RWSetWithTxnIndex>>,
    ) -> CrossShardDependencies {
        if current_round_rw_set_with_index.is_empty() && prev_rounds_rw_set_with_index.is_empty() {
            return CrossShardDependencies::default();
        }
        // Iterate through the frozen dependencies and add the max transaction index for each storage location
        let mut cross_shard_dependencies = CrossShardDependencies::default();
        for read_location in frozen_txn.read_set().iter() {
            // For current round, iterate through all shards less than current shards in the reverse order and for previous rounds iterate through all shards in the reverse order
            // and find the first shard id that has taken a write lock on the storage location. This ensures that we find the highest txn index that is conflicting
            // with the current transaction.
            for rw_set_with_index in current_round_rw_set_with_index
                .iter()
                .take(self.shard_id)
                .chain(prev_rounds_rw_set_with_index.iter())
            {
                if rw_set_with_index.has_write_lock(read_location) {
                    cross_shard_dependencies.add_depends_on_txn(
                        rw_set_with_index.get_write_lock_txn_index(read_location),
                    );
                    break;
                }
            }
        }

        for write_location in frozen_txn.write_set().iter() {
            for rw_set_with_index in current_round_rw_set_with_index
                .iter()
                .take(self.shard_id)
                .chain(prev_rounds_rw_set_with_index.iter())
            {
                if rw_set_with_index.has_read_lock(write_location) {
                    cross_shard_dependencies.add_depends_on_txn(
                        rw_set_with_index.get_read_lock_txn_index(write_location),
                    );
                    break;
                }
                if rw_set_with_index.has_write_lock(write_location) {
                    cross_shard_dependencies.add_depends_on_txn(
                        rw_set_with_index.get_write_lock_txn_index(write_location),
                    );
                    break;
                }
            }
        }

        cross_shard_dependencies
    }

    pub fn get_frozen_chunk(
        &self,
        txns: Vec<AnalyzedTransaction>,
        current_round_rw_set_with_index: Arc<Vec<RWSetWithTxnIndex>>,
        prev_round_rw_set_with_index: Arc<Vec<RWSetWithTxnIndex>>,
        index_offset: TxnIndex,
    ) -> TransactionsChunk {
        let mut frozen_txns = Vec::new();
        for txn in txns.into_iter() {
            let dependency = self.get_dependencies_for_frozen_txn(
                &txn,
                current_round_rw_set_with_index.clone(),
                prev_round_rw_set_with_index.clone(),
            );
            frozen_txns.push(TransactionWithDependencies::new(txn, dependency));
        }
        TransactionsChunk::new(index_offset, frozen_txns)
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

    fn check_for_read_conflict(
        &self,
        current_shard_id: ShardId,
        txn: &AnalyzedTransaction,
        cross_shard_rw_set: &[RWSet],
    ) -> bool {
        for read_location in txn.read_set().iter() {
            // Each storage location is allocated an anchor shard id, which is used to conflict resolution deterministically across shards.
            // During conflict resolution, shards starts scanning from the anchor shard id and
            // first shard id that has taken a read/write lock on this storage location is the owner of this storage location.
            // Please note another alternative is scan from first shard id, but this will result in non-uniform load across shards in case of conflicts.
            let anchor_shard_id = read_location.hash().byte(0) as usize % self.num_shards;
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
        for write_location in txn.write_set().iter() {
            let anchor_shard_id = write_location.hash().byte(0) as usize % self.num_shards;
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

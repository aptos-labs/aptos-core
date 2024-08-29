// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Copyright © Aptos Foundation

use crate::{BlockPartitioner, PartitionerConfig};
use aptos_types::{
    block_executor::partitioner::{
        PartitionV3, PartitionedTransactions, PartitionedTransactionsV3,
    },
    state_store::state_key::StateKey,
    transaction::analyzed_transaction::AnalyzedTransaction,
};
use std::collections::HashMap;

/// A partitioner that does not reorder and assign txns to shards in a round-robin way.
/// Only for testing the correctness or sharded execution V3.
pub struct V3NaivePartitioner {}

impl BlockPartitioner for V3NaivePartitioner {
    fn partition(
        &self,
        transactions: Vec<AnalyzedTransaction>,
        num_shards: usize,
    ) -> PartitionedTransactions {
        let shard_idx_of_txn = |txn_idx: u32| txn_idx as usize % num_shards; // Naive Round-Robin.
        let mut partitions = vec![PartitionV3::default(); num_shards];
        let mut owners_by_key: HashMap<StateKey, u32> = HashMap::new();
        for (cur_txn_idx, transaction) in transactions.into_iter().enumerate() {
            let cur_shard_idx = shard_idx_of_txn(cur_txn_idx as u32);

            // Find remote dependencies with reads + writes.
            for loc in transaction
                .read_hints
                .iter()
                .chain(transaction.write_hints.iter())
            {
                if let Some(owner_txn_idx) = owners_by_key.get(loc.state_key()) {
                    let owner_shard_idx = shard_idx_of_txn(*owner_txn_idx);
                    if owner_shard_idx == cur_shard_idx {
                        continue;
                    }
                    partitions[owner_shard_idx]
                        .insert_follower_shard(*owner_txn_idx, cur_shard_idx);
                    partitions[cur_shard_idx]
                        .insert_remote_dependency(*owner_txn_idx, loc.state_key().clone());
                }
            }

            // Update owner table with writes.
            for loc in transaction.write_hints.iter() {
                owners_by_key.insert(loc.state_key().clone(), cur_txn_idx as u32);
            }

            partitions[cur_shard_idx].append_txn(cur_txn_idx as u32, transaction);
        }

        let global_idx_lists_by_shard = partitions.iter().map(|p| p.global_idxs.clone()).collect();

        PartitionedTransactions::V3(PartitionedTransactionsV3 {
            block_id: [0; 32],
            partitions,
            global_idx_sets_by_shard: global_idx_lists_by_shard,
        })
    }
}

#[derive(Debug, Default)]
pub struct V3NaivePartitionerConfig {}

impl PartitionerConfig for V3NaivePartitionerConfig {
    fn build(&self) -> Box<dyn BlockPartitioner> {
        Box::new(V3NaivePartitioner {})
    }
}

pub fn build_partitioning_result(num_shards: usize, transactions: Vec<AnalyzedTransaction>, shard_idxs: Vec<usize>) -> PartitionedTransactionsV3 {
    let mut partitions = vec![PartitionV3::default(); num_shards];
    let mut owners_by_key: HashMap<StateKey, u32> = HashMap::new();
    for (cur_txn_idx, transaction) in transactions.into_iter().enumerate() {
        let cur_shard_idx = shard_idxs[cur_txn_idx];

        // Find remote dependencies with reads + writes.
        for loc in transaction
            .read_hints
            .iter()
            .chain(transaction.write_hints.iter())
        {
            if let Some(owner_txn_idx) = owners_by_key.get(loc.state_key()) {
                let owner_shard_idx = shard_idxs[*owner_txn_idx as usize];
                if owner_shard_idx == cur_shard_idx {
                    continue;
                }
                partitions[owner_shard_idx]
                    .insert_follower_shard(*owner_txn_idx, cur_shard_idx);
                partitions[cur_shard_idx]
                    .insert_remote_dependency(*owner_txn_idx, loc.state_key().clone());
            }
        }

        // Update owner table with writes.
        for loc in transaction.write_hints.iter() {
            owners_by_key.insert(loc.state_key().clone(), cur_txn_idx as u32);
        }

        partitions[cur_shard_idx].append_txn(cur_txn_idx as u32, transaction);
    }

    let global_idx_lists_by_shard = partitions.iter().map(|p| p.global_idxs.clone()).collect();

    PartitionedTransactionsV3 {
        block_id: [0; 32],
        partitions,
        global_idx_sets_by_shard: global_idx_lists_by_shard,
    }
}

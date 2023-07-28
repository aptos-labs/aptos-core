// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

pub mod sharded_block_partitioner;
pub mod omega_partitioner;

pub mod test_utils;

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use aptos_types::block_executor::partitioner::{CrossShardDependencies, ShardId, SubBlock, SubBlocksForShard, TransactionWithDependencies};
use aptos_types::transaction::analyzed_transaction::{AnalyzedTransaction, StorageLocation};
use aptos_types::transaction::Transaction;
use crate::omega_partitioner::OmegaPartitioner;
use crate::sharded_block_partitioner::ShardedBlockPartitioner;

pub trait BlockPartitioner: Send + Sync {
    fn partition(&self, transactions: Vec<AnalyzedTransaction>, num_shards: usize)
        -> Vec<SubBlocksForShard<AnalyzedTransaction>>;
}

// pub fn build_partitioner() -> Box<dyn BlockPartitioner> {
//     match std::env::var("APTOS_PARTITIONER_IMPL").ok() {
//         Some(v) if v.as_str() == "v2" => {
//             Box::new(OmegaPartitioner::new())
//         }
//         _ => {
//             ShardedBlockPartitioner::new()
//         }
//     }
// }

/// An implementation of partitioner that splits the transactions into equal-sized chunks.
pub struct UniformPartitioner {}

impl BlockPartitioner for UniformPartitioner {
    fn partition(
        &self,
        transactions: Vec<AnalyzedTransaction>,
        num_shards: usize,
    ) -> Vec<SubBlocksForShard<AnalyzedTransaction>> {
        let total_txns = transactions.len();
        if total_txns == 0 {
            return vec![];
        }
        let txns_per_shard = (total_txns as f64 / num_shards as f64).ceil() as usize;

        let mut result: Vec<SubBlocksForShard<AnalyzedTransaction>> = Vec::new();
        let mut global_txn_counter: usize = 0;
        for (shard_id, chunk) in transactions.chunks(txns_per_shard).enumerate() {
            let twds: Vec<TransactionWithDependencies<AnalyzedTransaction>> = chunk.iter().map(|t|TransactionWithDependencies::new(t.clone(), CrossShardDependencies::default())).collect();
            let sub_block = SubBlock::new(global_txn_counter, twds);
            global_txn_counter += sub_block.num_txns();
            result.push(SubBlocksForShard::new(shard_id, vec![sub_block]));
        }
        result
    }
}

fn get_anchor_shard_id(storage_location: &StorageLocation, num_shards: usize) -> ShardId {
    let mut hasher = DefaultHasher::new();
    storage_location.hash(&mut hasher);
    (hasher.finish() % num_shards as u64) as usize
}

// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

pub mod sharded_block_partitioner;
pub mod test_utils;

use aptos_types::transaction::Transaction;

pub trait BlockPartitioner: Send + Sync {
    fn partition(&self, transactions: Vec<Transaction>, num_shards: usize)
        -> Vec<Vec<Transaction>>;
}

/// An implementation of partitioner that splits the transactions into equal-sized chunks.
pub struct UniformPartitioner {}

impl BlockPartitioner for UniformPartitioner {
    fn partition(
        &self,
        transactions: Vec<Transaction>,
        num_shards: usize,
    ) -> Vec<Vec<Transaction>> {
        let total_txns = transactions.len();
        if total_txns == 0 {
            return vec![];
        }
        let txns_per_shard = (total_txns as f64 / num_shards as f64).ceil() as usize;

        let mut result = Vec::new();
        for chunk in transactions.chunks(txns_per_shard) {
            result.push(chunk.to_vec());
        }
        result
    }
}

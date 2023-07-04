// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

pub mod sharded_block_partitioner;
pub mod simple_partitioner;
pub mod no_op;

pub mod test_utils;

use aptos_types::block_executor::partitioner::SubBlocksForShard;
use aptos_types::transaction::Transaction;

pub trait BlockPartitioner: Send {
    fn partition(&self, transactions: Vec<Transaction>, num_executor_shards: usize)
        -> Vec<SubBlocksForShard<Transaction>>;
}

/// An implementation of partitioner that splits the transactions into equal-sized chunks.
pub struct UniformPartitioner {}

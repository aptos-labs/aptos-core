// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

pub mod sharded_block_partitioner;
pub mod test_utils;
pub mod types;

use crate::types::ExecutableTransactions;
use aptos_types::transaction::Transaction;

pub trait BlockPartitioner {
    fn partition(&self, transactions: Vec<Transaction>) -> ExecutableTransactions;
}
pub struct NoOpBlockPartitioner {}

impl BlockPartitioner for NoOpBlockPartitioner {
    fn partition(&self, transactions: Vec<Transaction>) -> ExecutableTransactions {
        partition_no_op(transactions)
    }
}

fn partition_no_op(transactions: Vec<Transaction>) -> ExecutableTransactions {
    ExecutableTransactions::Unsharded(transactions)
}

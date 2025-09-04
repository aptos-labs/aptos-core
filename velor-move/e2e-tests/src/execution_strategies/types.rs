// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]
use velor_types::transaction::{SignedTransaction, TransactionOutput};

pub type Block<Txn> = Vec<Txn>;
pub type ExecutorResult<T> = Result<Vec<TransactionOutput>, T>;

pub trait Executor {
    type Txn;
    type BlockResult: std::error::Error;
    fn execute_block(&mut self, txns: Block<Self::Txn>) -> ExecutorResult<Self::BlockResult>;
}

pub trait PartitionStrategy {
    type Txn;
    fn partition(&mut self, block: Block<Self::Txn>) -> Vec<Block<SignedTransaction>>;
}

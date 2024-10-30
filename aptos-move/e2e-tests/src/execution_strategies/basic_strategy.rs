// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{
    execution_strategies::types::{Block, Executor, ExecutorResult, PartitionStrategy},
    executor::FakeExecutor,
};
use aptos_types::{transaction::SignedTransaction, vm_status::VMStatus};

#[derive(Debug)]
pub struct BasicStrategy;

impl PartitionStrategy for BasicStrategy {
    type Txn = SignedTransaction;

    fn partition(&mut self, block: Block<Self::Txn>) -> Vec<Block<SignedTransaction>> {
        vec![block]
    }
}

pub struct BasicExecutor {
    pub executor: FakeExecutor,
    strategy: BasicStrategy,
}

impl Default for BasicExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl BasicExecutor {
    pub fn new() -> Self {
        Self {
            executor: FakeExecutor::from_head_genesis(),
            strategy: BasicStrategy,
        }
    }
}

impl Executor for BasicExecutor {
    type BlockResult = VMStatus;
    type Txn = <BasicStrategy as PartitionStrategy>::Txn;

    fn execute_block(&mut self, txns: Block<Self::Txn>) -> ExecutorResult<Self::BlockResult> {
        let mut block = self.strategy.partition(txns);
        let outputs = self.executor.execute_block(block.remove(0))?;
        for output in &outputs {
            self.executor.apply_write_set(output.write_set())
        }
        Ok(outputs)
    }
}

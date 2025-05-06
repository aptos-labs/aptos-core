// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::Transaction;
use std::fmt::Debug;

#[derive(Debug)]
pub struct BlockOutput<Output: Debug> {
    transaction_outputs: Vec<Output>,
    block_epilogue_txn: Option<Transaction>,
}

impl<Output: Debug> BlockOutput<Output> {
    pub fn new(transaction_outputs: Vec<Output>, block_epilogue_txn: Option<Transaction>) -> Self {
        Self {
            transaction_outputs,
            block_epilogue_txn,
        }
    }

    fn has_block_epilogue_txn(&self) -> bool {
        self.block_epilogue_txn.is_some()
    }

    pub fn into_transaction_outputs_forced(self) -> Vec<Output> {
        assert!(!self.has_block_epilogue_txn());
        self.transaction_outputs
    }

    pub fn into_inner(self) -> (Vec<Output>, Option<Transaction>) {
        (self.transaction_outputs, self.block_epilogue_txn)
    }

    pub fn get_transaction_outputs_forced(&self) -> &[Output] {
        assert!(!self.has_block_epilogue_txn());
        &self.transaction_outputs
    }
}

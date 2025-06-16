// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::Transaction;
use std::fmt::Debug;

#[derive(Debug)]
pub struct BlockOutput<Output: Debug> {
    transaction_outputs: Vec<Output>,
    // A BlockEpilogueTxn might be appended to the block.
    // This field will be None iff the input is not a block, or an epoch change is triggered.
    block_epilogue_txn: Option<Transaction>,
}

impl<Output: Debug> BlockOutput<Output> {
    pub fn new(transaction_outputs: Vec<Output>, block_epilogue_txn: Option<Transaction>) -> Self {
        Self {
            transaction_outputs,
            block_epilogue_txn,
        }
    }

    pub fn into_transaction_outputs_forced(self) -> Vec<Output> {
        self.transaction_outputs
    }

    pub fn into_inner(self) -> (Vec<Output>, Option<Transaction>) {
        (self.transaction_outputs, self.block_epilogue_txn)
    }

    pub fn get_transaction_outputs_forced(&self) -> &[Output] {
        &self.transaction_outputs
    }
}

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::fmt::Debug;

#[derive(Debug)]
pub struct BlockOutput<Output: Debug> {
    transaction_outputs: Vec<Output>,
    // TODO add block_limit_info
}

impl<Output: Debug> BlockOutput<Output> {
    pub fn new(transaction_outputs: Vec<Output>) -> Self {
        Self {
            transaction_outputs,
        }
    }

    /// If block limit is not set (i.e. in tests), we can safely unwrap here
    pub fn into_transaction_outputs_forced(self) -> Vec<Output> {
        // TODO assert there is no block limit info?
        // assert!(self.block_limit_info_transaction.is_none());
        self.transaction_outputs
    }

    // TODO add block_limit_info
    pub fn into_inner(self) -> Vec<Output> {
        self.transaction_outputs
    }

    pub fn get_transaction_outputs_forced(&self) -> &[Output] {
        // TODO assert there is no block limit info?
        // assert!(self.block_limit_info_transaction.is_none());
        &self.transaction_outputs
    }
}

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::block_epilogue::BlockEndInfo;
use std::fmt::Debug;

#[derive(Debug)]
pub struct BlockOutput<Output: Debug> {
    transaction_outputs: Vec<Output>,
    block_end_info: Option<BlockEndInfo>,
}

impl<Output: Debug> BlockOutput<Output> {
    pub fn new(transaction_outputs: Vec<Output>, block_end_info: Option<BlockEndInfo>) -> Self {
        Self {
            transaction_outputs,
            block_end_info,
        }
    }

    fn is_block_limit_reached(&self) -> bool {
        self.block_end_info
            .as_ref()
            .is_some_and(BlockEndInfo::limit_reached)
    }

    /// If block limit is not set (i.e. in tests), we can safely unwrap here
    pub fn into_transaction_outputs_forced(self) -> Vec<Output> {
        assert!(!self.is_block_limit_reached());
        self.transaction_outputs
    }

    pub fn into_inner(self) -> (Vec<Output>, Option<BlockEndInfo>) {
        (self.transaction_outputs, self.block_end_info)
    }

    pub fn get_transaction_outputs_forced(&self) -> &[Output] {
        assert!(!self.is_block_limit_reached());
        &self.transaction_outputs
    }
}

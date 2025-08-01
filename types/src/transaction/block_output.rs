// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::BlockExecutableTransaction;
use crate::state_store::state_slot::StateSlot;
use std::{collections::BTreeMap, fmt::Debug};

#[derive(Debug)]
pub struct BlockOutput<T, Output: Debug>
where
    T: BlockExecutableTransaction,
{
    transaction_outputs: Vec<Output>,
    // A BlockEpilogueTxn might be appended to the block.
    // This field will be None iff the input is not a block, or an epoch change is triggered.
    block_epilogue_txn: Option<T>,
    to_make_hot: BTreeMap<T::Key, StateSlot>,
}

impl<T, Output: Debug> BlockOutput<T, Output>
where
    T: BlockExecutableTransaction,
{
    pub fn new(
        transaction_outputs: Vec<Output>,
        block_epilogue_txn: Option<T>,
        to_make_hot: BTreeMap<T::Key, StateSlot>,
    ) -> Self {
        Self {
            transaction_outputs,
            block_epilogue_txn,
            to_make_hot,
        }
    }

    pub fn into_transaction_outputs_forced(self) -> Vec<Output> {
        self.transaction_outputs
    }

    pub fn into_inner(self) -> (Vec<Output>, Option<T>, BTreeMap<T::Key, StateSlot>) {
        (
            self.transaction_outputs,
            self.block_epilogue_txn,
            self.to_make_hot,
        )
    }

    pub fn get_transaction_outputs_forced(&self) -> &[Output] {
        &self.transaction_outputs
    }
}

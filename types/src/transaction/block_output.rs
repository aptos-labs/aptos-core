// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::Transaction;
use crate::state_store::state_slot::StateSlot;
use std::collections::BTreeMap;
use std::fmt::Debug;

#[derive(Debug)]
pub struct BlockOutput<Key, Output: Debug> {
    transaction_outputs: Vec<Output>,
    // A BlockEpilogueTxn might be appended to the block.
    // This field will be None iff the input is not a block, or an epoch change is triggered.
    block_epilogue_txn: Option<Transaction>,
    slots_to_make_hot: BTreeMap<Key, StateSlot>,
}

impl<Key, Output: Debug> BlockOutput<Key, Output> {
    pub fn new(
        transaction_outputs: Vec<Output>,
        block_epilogue_txn: Option<Transaction>,
        slots_to_make_hot: BTreeMap<Key, StateSlot>,
    ) -> Self {
        Self {
            transaction_outputs,
            block_epilogue_txn,
            slots_to_make_hot,
        }
    }

    pub fn into_transaction_outputs_forced(self) -> Vec<Output> {
        self.transaction_outputs
    }

    pub fn into_inner(self) -> (Vec<Output>, Option<Transaction>, BTreeMap<Key, StateSlot>) {
        (self.transaction_outputs, self.block_epilogue_txn, self.slots_to_make_hot)
    }

    pub fn get_transaction_outputs_forced(&self) -> &[Output] {
        &self.transaction_outputs
    }
}

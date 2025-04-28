// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::types::ReadWriteSummary;
use aptos_types::transaction::{block_epilogue::THotStateOp, BlockExecutableTransaction};
use std::marker::PhantomData;

pub struct BlockHotStateOpAccumulator<Txn> {
    _phantom: PhantomData<Txn>,
}

impl<Txn: BlockExecutableTransaction> BlockHotStateOpAccumulator<Txn> {
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }

    pub fn add_transaction(&mut self, _read_write_summary: &ReadWriteSummary<Txn>) {
        // TODO
    }

    pub fn get_hot_state_ops(&self) -> Vec<THotStateOp<Txn::Key>> {
        vec![]
    }
}

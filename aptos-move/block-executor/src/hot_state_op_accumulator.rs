// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::types::ReadWriteSummary;
use aptos_types::transaction::BlockExecutableTransaction;
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

    pub fn get_keys_to_make_hot(&self) -> Vec<Txn::Key> {
        todo!()
    }
}

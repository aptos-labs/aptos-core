// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::txn_provider::TxnProvider;
use aptos_mvhashmap::types::TxnIndex;
use aptos_types::transaction::{AuxiliaryInfoTrait, BlockExecutableTransaction as Transaction};
use once_cell::sync::OnceCell;

pub struct BlockingTxnProvider<T: Transaction + std::fmt::Debug> {
    txns: Vec<OnceCell<T>>,
}

#[allow(dead_code)]
impl<T: Transaction + std::fmt::Debug> BlockingTxnProvider<T> {
    pub fn new(num_txns: usize) -> Self {
        let txns = vec![OnceCell::new(); num_txns];
        Self { txns }
    }

    pub fn set_txn(&self, idx: TxnIndex, txn: T) {
        self.txns[idx as usize]
            .set(txn)
            .expect("Trying to set a txn that is already present");
    }
}

impl<T: Transaction + std::fmt::Debug, A: AuxiliaryInfoTrait> TxnProvider<T, A>
    for BlockingTxnProvider<T>
{
    fn num_txns(&self) -> usize {
        self.txns.len()
    }

    fn get_txn(&self, idx: TxnIndex) -> &T {
        self.txns[idx as usize].wait()
    }

    fn get_auxiliary_info(&self, _idx: TxnIndex) -> A {
        // TODO: The whole struct seems to be dead code for now, implement this when necessary.
        unimplemented!()
    }
}

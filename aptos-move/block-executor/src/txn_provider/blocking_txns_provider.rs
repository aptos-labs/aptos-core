// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::txn_provider::TxnProvider;
use aptos_mvhashmap::types::TxnIndex;
use aptos_types::transaction::BlockExecutableTransaction as Transaction;
use once_cell::sync::OnceCell;

pub struct BlockingTransaction<T: Transaction + std::fmt::Debug> {
    pub txn: OnceCell<T>,
}

#[allow(dead_code)]
impl<T: Transaction + std::fmt::Debug> BlockingTransaction<T> {
    pub fn new() -> Self {
        Self {
            txn: OnceCell::new(),
        }
    }
}

pub struct BlockingTxnsProvider<T: Transaction + std::fmt::Debug> {
    txns: Vec<BlockingTransaction<T>>,
}

#[allow(dead_code)]
impl<T: Transaction + std::fmt::Debug> BlockingTxnsProvider<T> {
    pub fn new(txns: Vec<BlockingTransaction<T>>) -> Self {
        Self { txns }
    }

    pub fn set_txn(&self, idx: TxnIndex, txn: T) {
        self.txns[idx as usize]
            .txn
            .set(txn)
            .expect("Trying to set a txn that is already present");
    }
}

impl<T: Transaction + std::fmt::Debug> TxnProvider<T> for BlockingTxnsProvider<T> {
    fn num_txns(&self) -> usize {
        self.txns.len()
    }

    fn get_txn(&self, idx: TxnIndex) -> &T {
        self.txns[idx as usize].txn.wait()
    }
}

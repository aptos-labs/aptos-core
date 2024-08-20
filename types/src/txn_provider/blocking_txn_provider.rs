// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    transaction::BlockExecutableTransaction as Transaction,
    txn_provider::{TxnIndex, TxnProvider},
};
use once_cell::sync::OnceCell;
use std::sync::Arc;

#[derive(Clone)]
pub struct BlockingTxnProvider<T: Transaction + std::fmt::Debug> {
    txns: Arc<Vec<OnceCell<T>>>,
}

#[allow(dead_code)]
impl<T: Transaction + std::fmt::Debug> BlockingTxnProvider<T> {
    pub fn new(num_txns: usize) -> Self {
        let txns = Arc::new(vec![OnceCell::new(); num_txns]);
        Self { txns }
    }

    pub fn set_txn(&self, idx: TxnIndex, txn: T) {
        self.txns[idx as usize]
            .set(txn)
            .expect("Trying to set a txn that is already present");
    }
}

impl<T: Transaction + std::fmt::Debug> TxnProvider<T> for BlockingTxnProvider<T> {
    fn num_txns(&self) -> usize {
        self.txns.len()
    }

    fn get_txn(&self, idx: TxnIndex) -> &T {
        let res = self.txns[idx as usize].wait();
        res
    }

    fn to_vec(&self) -> Vec<T> {
        let mut txns = vec![];
        for i in 0..self.num_txns() as TxnIndex {
            let txn = self.get_txn(i).clone();
            txns.push(txn);
        }
        txns
    }
}

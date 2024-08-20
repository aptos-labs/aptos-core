// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    transaction::BlockExecutableTransaction as Transaction,
    txn_provider::{TxnIndex, TxnProvider},
};
use once_cell::sync::OnceCell;
use std::sync::{Arc, Condvar, Mutex};

#[allow(dead_code)]
pub enum BlockingTransactionStatus<T: Transaction> {
    Ready(Arc<T>),
    Waiting,
}

pub struct BlockingTransaction<T: Transaction> {
    pub txn: Mutex<BlockingTransactionStatus<T>>,
    pub cvar: Condvar,
}

#[allow(dead_code)]
impl<T: Transaction> BlockingTransaction<T> {
    pub fn new() -> Self {
        Self {
            txn: Mutex::new(BlockingTransactionStatus::Waiting),
            cvar: Condvar::new(),
        }
    }
}

impl<T: Transaction> Default for BlockingTransaction<T> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct BlockingTxnsProvider<T: Transaction> {
    txns: Vec<OnceCell<Arc<T>>>,
}

#[allow(dead_code)]
impl<T: Transaction + std::fmt::Debug> BlockingTxnsProvider<T> {
    pub fn new(num_txns: usize) -> Self {
        let txns: Vec<_> = (0..num_txns).map(|_| OnceCell::new()).collect();
        Self { txns }
    }

    pub fn set_txn(&self, idx: TxnIndex, txn: T) {
        let blocking_txn = &self.txns[idx as usize];
        blocking_txn.set(Arc::new(txn)).unwrap();
    }
}

impl<T: Transaction> TxnProvider<T> for BlockingTxnsProvider<T> {
    fn num_txns(&self) -> usize {
        self.txns.len()
    }

    fn get_txn(&self, idx: TxnIndex) -> Arc<T> {
        let txn = &self.txns[idx as usize];
        txn.wait().clone()
    }

    fn to_vec(&self) -> Vec<T> {
        let mut txns = vec![];
        for i in 0..self.num_txns() as TxnIndex {
            let txn = self.get_txn(i).as_ref().clone();
            txns.push(txn);
        }
        txns
    }
}

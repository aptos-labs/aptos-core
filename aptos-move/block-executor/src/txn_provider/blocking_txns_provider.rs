// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::txn_provider::TxnProvider;
use aptos_mvhashmap::types::TxnIndex;
use aptos_types::transaction::BlockExecutableTransaction as Transaction;
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

pub struct BlockingTxnsProvider<T: Transaction> {
    txns: Vec<BlockingTransaction<T>>,
}

#[allow(dead_code)]
impl<T: Transaction> BlockingTxnsProvider<T> {
    pub fn new(txns: Vec<BlockingTransaction<T>>) -> Self {
        Self { txns }
    }

    pub fn set_txn(&self, idx: TxnIndex, txn: T) {
        let blocking_txn = &self.txns[idx as usize];
        let (lock, cvar) = (&blocking_txn.txn, &blocking_txn.cvar);
        let mut status = lock.lock().unwrap();
        match &*status {
            BlockingTransactionStatus::Waiting => {
                *status = BlockingTransactionStatus::Ready(Arc::new(txn));
                cvar.notify_all();
            },
            BlockingTransactionStatus::Ready(_) => {
                panic!("Trying to add a txn that is already present");
            },
        }
    }
}

impl<T: Transaction> TxnProvider<T> for BlockingTxnsProvider<T> {
    fn num_txns(&self) -> usize {
        self.txns.len()
    }

    fn get_txn(&self, idx: TxnIndex) -> Arc<T> {
        let txn = &self.txns[idx as usize];
        let mut status = txn.txn.lock().unwrap();
        while let BlockingTransactionStatus::Waiting = *status {
            status = txn.cvar.wait(status).unwrap();
        }
        match *status {
            BlockingTransactionStatus::Ready(ref txn) => txn.clone(),
            BlockingTransactionStatus::Waiting => panic!("Unexpected status"),
        }
    }
}

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::txn_provider::TxnProvider;
use aptos_mvhashmap::types::TxnIndex;
use aptos_types::transaction::BlockExecutableTransaction as Transaction;
use std::sync::Arc;

pub struct DefaultTxnProvider<T: Transaction> {
    pub txns: Vec<Arc<T>>,
}

impl<T: Transaction> DefaultTxnProvider<T> {
    pub fn new(txns: Vec<T>) -> Self {
        let txns = txns.into_iter().map(|txn| Arc::new(txn)).collect();
        Self { txns }
    }

    pub fn get_txns(&self) -> &Vec<Arc<T>> {
        &self.txns
    }
}

impl<T: Transaction> TxnProvider<T> for DefaultTxnProvider<T> {
    fn num_txns(&self) -> usize {
        self.txns.len()
    }

    fn get_txn(&self, idx: TxnIndex) -> Arc<T> {
        self.txns[idx as usize].clone()
    }
}

impl<T: Transaction> Iterator for DefaultTxnProvider<T> {
    type Item = Arc<T>;

    fn next(&mut self) -> Option<Self::Item> {
        self.txns.pop()
    }
}

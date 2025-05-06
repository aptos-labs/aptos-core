// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::txn_provider::TxnProvider;
use aptos_mvhashmap::types::TxnIndex;
use aptos_types::transaction::{BlockExecutableTransaction as Transaction, ExtraInfo};

pub struct DefaultTxnProvider<T: Transaction> {
    pub txns: Vec<T>,
    extra_info: Vec<Option<ExtraInfo>>,
}

impl<T: Transaction> DefaultTxnProvider<T> {
    pub fn new(txns: Vec<T>, extra_info: Vec<Option<ExtraInfo>>) -> Self {
        assert!(txns.len() == extra_info.len());
        Self { txns, extra_info }
    }

    pub fn new_without_info(txns: Vec<T>) -> Self {
        let len = txns.len();
        let mut extra_info = Vec::with_capacity(len);
        extra_info.resize(len, None);
        Self { txns, extra_info }
    }

    pub fn get_txns(&self) -> &Vec<T> {
        &self.txns
    }
}

impl<T: Transaction> TxnProvider<T> for DefaultTxnProvider<T> {
    fn num_txns(&self) -> usize {
        self.txns.len()
    }

    fn get_txn(&self, idx: TxnIndex) -> &T {
        &self.txns[idx as usize]
    }

    fn get_extra_info(&self, idx: TxnIndex) -> Option<&ExtraInfo> {
        self.extra_info[idx as usize].as_ref()
    }
}

/*
impl<T: Transaction> Iterator for DefaultTxnProvider<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.txns.pop()
    }
}*/

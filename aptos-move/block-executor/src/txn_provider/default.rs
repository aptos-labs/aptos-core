// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::txn_provider::TxnProvider;
use aptos_mvhashmap::types::TxnIndex;
use aptos_types::transaction::{AuxiliaryInfo, BlockExecutableTransaction as Transaction};

pub struct DefaultTxnProvider<T: Transaction> {
    pub txns: Vec<T>,
    auxiliary_info: Vec<AuxiliaryInfo>,
}

impl<T: Transaction> DefaultTxnProvider<T> {
    pub fn new(txns: Vec<T>, auxiliary_info: Vec<AuxiliaryInfo>) -> Self {
        assert!(txns.len() == auxiliary_info.len());
        Self {
            txns,
            auxiliary_info,
        }
    }

    pub fn new_without_info(txns: Vec<T>) -> Self {
        let len = txns.len();
        let mut auxiliary_info = Vec::with_capacity(len);
        auxiliary_info.resize(len, AuxiliaryInfo::new_empty());
        Self {
            txns,
            auxiliary_info,
        }
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

    fn get_auxiliary_info(&self, idx: TxnIndex) -> &AuxiliaryInfo {
        &self.auxiliary_info[idx as usize]
    }
}

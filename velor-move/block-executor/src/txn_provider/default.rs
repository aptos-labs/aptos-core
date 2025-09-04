// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::txn_provider::TxnProvider;
use velor_mvhashmap::types::TxnIndex;
use velor_types::transaction::{AuxiliaryInfoTrait, BlockExecutableTransaction as Transaction};

pub struct DefaultTxnProvider<T: Transaction, A: AuxiliaryInfoTrait> {
    txns: Vec<T>,
    auxiliary_info: Vec<A>,
}

impl<T: Transaction, A: AuxiliaryInfoTrait> DefaultTxnProvider<T, A> {
    pub fn new(txns: Vec<T>, auxiliary_info: Vec<A>) -> Self {
        assert!(txns.len() == auxiliary_info.len());
        Self {
            txns,
            auxiliary_info,
        }
    }

    pub fn new_without_info(txns: Vec<T>) -> Self {
        let len = txns.len();
        let mut auxiliary_info = Vec::with_capacity(len);
        auxiliary_info.resize(len, A::new_empty());
        Self {
            txns,
            auxiliary_info,
        }
    }

    pub fn get_txns(&self) -> &Vec<T> {
        &self.txns
    }

    pub fn into_inner(self) -> (Vec<T>, Vec<A>) {
        (self.txns, self.auxiliary_info)
    }
}

impl<T: Transaction, A: AuxiliaryInfoTrait> TxnProvider<T, A> for DefaultTxnProvider<T, A> {
    fn num_txns(&self) -> usize {
        self.txns.len()
    }

    fn get_txn(&self, idx: TxnIndex) -> &T {
        &self.txns[idx as usize]
    }

    fn get_auxiliary_info(&self, txn_index: TxnIndex) -> A {
        if (txn_index as usize) < self.auxiliary_info.len() {
            self.auxiliary_info[txn_index as usize].clone()
        } else {
            // Check if existing auxiliary infos are None to maintain consistency
            if !self.auxiliary_info.is_empty() {
                // Sample existing auxiliary infos to check the pattern
                let all_auxiliary_infos_are_none = self
                    .auxiliary_info
                    .iter()
                    .all(|info| info.transaction_index().is_none());

                if all_auxiliary_infos_are_none {
                    // If existing auxiliary infos are None, use None for consistency (version 0 behavior)
                    A::new_empty()
                } else {
                    // Otherwise, use the standard function (version 1 behavior)
                    A::auxiliary_info_at_txn_index(txn_index)
                }
            } else {
                // Fallback if no existing auxiliary infos
                A::new_empty()
            }
        }
    }
}

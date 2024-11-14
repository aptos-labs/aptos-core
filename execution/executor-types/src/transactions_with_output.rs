// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::TIMER;
use aptos_metrics_core::TimerHelper;
use aptos_types::transaction::{Transaction, TransactionOutput};
use itertools::izip;

#[derive(Debug, Default)]
pub struct TransactionsWithOutput {
    pub transactions: Vec<Transaction>,
    pub transaction_outputs: Vec<TransactionOutput>,
    pub is_reconfig: bool,
}

impl TransactionsWithOutput {
    pub fn new(
        transactions: Vec<Transaction>,
        transaction_outputs: Vec<TransactionOutput>,
        is_reconfig: bool,
    ) -> Self {
        assert_eq!(transactions.len(), transaction_outputs.len());
        Self {
            transactions,
            transaction_outputs,
            is_reconfig,
        }
    }

    pub fn new_empty() -> Self {
        Self::default()
    }

    pub fn new_dummy_success(txns: Vec<Transaction>) -> Self {
        let txn_outputs = vec![TransactionOutput::new_empty_success(); txns.len()];
        Self::new(txns, txn_outputs, false)
    }

    pub fn push(
        &mut self,
        transaction: Transaction,
        transaction_output: TransactionOutput,
        is_reconfig: bool,
    ) {
        // can't add more txns after reconfig
        assert!(!self.is_reconfig);

        self.transactions.push(transaction);
        self.transaction_outputs.push(transaction_output);
        self.is_reconfig = is_reconfig;
    }

    pub fn len(&self) -> usize {
        self.transactions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.transactions.is_empty()
    }

    pub fn txns(&self) -> &Vec<Transaction> {
        &self.transactions
    }

    pub fn transaction_outputs(&self) -> &[TransactionOutput] {
        &self.transaction_outputs
    }

    pub fn get_last_checkpoint_index(&self) -> Option<usize> {
        if self.is_reconfig {
            return Some(self.len() - 1);
        }

        (0..self.len())
            .rev()
            .find(|&i| self.transactions[i].is_non_reconfig_block_ending())
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Transaction, &TransactionOutput)> {
        izip!(self.transactions.iter(), self.transaction_outputs.iter(),)
    }

    pub fn ends_with_sole_checkpoint(&self) -> bool {
        let _timer = TIMER.timer_with(&["ends_with_sole_checkpoint"]);
        if self.is_reconfig {
            !self
                .txns()
                .iter()
                .any(Transaction::is_non_reconfig_block_ending)
        } else {
            self.txns()
                .iter()
                .position(Transaction::is_non_reconfig_block_ending)
                == Some(self.len() - 1)
        }
    }
}

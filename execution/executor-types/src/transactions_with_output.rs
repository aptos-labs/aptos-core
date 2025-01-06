// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::TIMER;
use aptos_metrics_core::TimerHelper;
use aptos_storage_interface::state_store::sharded_state_update_refs::ShardedStateUpdateRefs;
use aptos_types::transaction::{Transaction, TransactionOutput};
use itertools::izip;
use std::{
    fmt::{Debug, Formatter},
    ops::Deref,
};

#[derive(Debug, Default)]
pub struct TransactionsWithOutput {
    pub transactions: Vec<Transaction>,
    pub transaction_outputs: Vec<TransactionOutput>,
}

impl TransactionsWithOutput {
    pub fn new(
        transactions: Vec<Transaction>,
        transaction_outputs: Vec<TransactionOutput>,
    ) -> Self {
        assert_eq!(transactions.len(), transaction_outputs.len());
        Self {
            transactions,
            transaction_outputs,
        }
    }

    pub fn new_empty() -> Self {
        Self::default()
    }

    pub fn push(&mut self, transaction: Transaction, transaction_output: TransactionOutput) {
        self.transactions.push(transaction);
        self.transaction_outputs.push(transaction_output);
    }

    pub fn len(&self) -> usize {
        self.transactions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.transactions.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Transaction, &TransactionOutput)> {
        izip!(self.transactions.iter(), self.transaction_outputs.iter(),)
    }
}

#[ouroboros::self_referencing]
pub struct TransactionsToKeep {
    transactions_with_output: TransactionsWithOutput,
    last_checkpoint_index: Option<usize>,
    is_reconfig: bool,
    #[borrows(transactions_with_output)]
    #[covariant]
    state_update_refs: ShardedStateUpdateRefs<'this>,
}

impl TransactionsToKeep {
    pub fn index(transactions_with_output: TransactionsWithOutput, is_reconfig: bool) -> Self {
        let _timer = TIMER.timer_with(&["transactions_to_keep__index"]);

        let num_write_sets = transactions_with_output.len();
        let last_checkpoint_index =
            Self::get_last_checkpoint_index(is_reconfig, &transactions_with_output.transactions);
        TransactionsToKeepBuilder {
            transactions_with_output,
            is_reconfig,
            last_checkpoint_index,
            state_update_refs_builder: |transactions_with_output| {
                let write_sets = transactions_with_output
                    .transaction_outputs
                    .iter()
                    .map(TransactionOutput::write_set);
                ShardedStateUpdateRefs::index_write_sets(write_sets, num_write_sets)
            },
        }
        .build()
    }

    pub fn make(
        transactions: Vec<Transaction>,
        transaction_outputs: Vec<TransactionOutput>,
        is_reconfig: bool,
    ) -> Self {
        let txns_with_output = TransactionsWithOutput::new(transactions, transaction_outputs);
        Self::index(txns_with_output, is_reconfig)
    }

    pub fn new_empty() -> Self {
        Self::make(vec![], vec![], false)
    }

    pub fn new_dummy_success(txns: Vec<Transaction>) -> Self {
        let txn_outputs = vec![TransactionOutput::new_empty_success(); txns.len()];
        Self::make(txns, txn_outputs, false)
    }

    pub fn state_update_refs(&self) -> &ShardedStateUpdateRefs {
        self.borrow_state_update_refs()
    }

    pub fn is_reconfig(&self) -> bool {
        *self.borrow_is_reconfig()
    }

    pub fn last_checkpoint_index(&self) -> Option<usize> {
        *self.borrow_last_checkpoint_index()
    }

    pub fn ends_with_sole_checkpoint(&self) -> bool {
        let _timer = TIMER.timer_with(&["ends_with_sole_checkpoint"]);

        if self.is_reconfig() {
            !self
                .transactions
                .iter()
                .any(Transaction::is_non_reconfig_block_ending)
        } else {
            self.transactions
                .iter()
                .position(Transaction::is_non_reconfig_block_ending)
                == Some(self.len() - 1)
        }
    }

    fn get_last_checkpoint_index(is_reconfig: bool, transactions: &[Transaction]) -> Option<usize> {
        let _timer = TIMER.timer_with(&["get_last_checkpoint_index"]);

        if is_reconfig {
            return Some(transactions.len() - 1);
        }

        transactions
            .iter()
            .rposition(Transaction::is_non_reconfig_block_ending)
    }
}

impl Deref for TransactionsToKeep {
    type Target = TransactionsWithOutput;

    fn deref(&self) -> &Self::Target {
        self.borrow_transactions_with_output()
    }
}

impl Debug for TransactionsToKeep {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TransactionsToKeep")
            .field(
                "transactions_with_output",
                self.borrow_transactions_with_output(),
            )
            .field("is_reconfig", &self.is_reconfig())
            .finish()
    }
}

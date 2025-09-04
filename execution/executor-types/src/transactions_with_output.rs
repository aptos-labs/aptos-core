// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::TIMER;
use anyhow::{Result, ensure};
use aptos_metrics_core::TimerHelper;
use aptos_storage_interface::state_store::state_update_refs::StateUpdateRefs;
use aptos_types::transaction::{PersistedAuxiliaryInfo, Transaction, TransactionOutput, Version};
use itertools::izip;
use std::{
    fmt::{Debug, Formatter},
    ops::Deref,
};

#[derive(Debug, Default)]
pub struct TransactionsWithOutput {
    pub transactions: Vec<Transaction>,
    pub transaction_outputs: Vec<TransactionOutput>,
    pub persisted_auxiliary_infos: Vec<PersistedAuxiliaryInfo>,
}

impl TransactionsWithOutput {
    pub fn new(
        transactions: Vec<Transaction>,
        transaction_outputs: Vec<TransactionOutput>,
        persisted_auxiliary_infos: Vec<PersistedAuxiliaryInfo>,
    ) -> Self {
        assert_eq!(transactions.len(), transaction_outputs.len());
        assert_eq!(transactions.len(), persisted_auxiliary_infos.len());
        Self {
            transactions,
            transaction_outputs,
            persisted_auxiliary_infos,
        }
    }

    pub fn new_empty() -> Self {
        Self::default()
    }

    pub fn push(
        &mut self,
        transaction: Transaction,
        transaction_output: TransactionOutput,
        persisted_auxiliary_info: PersistedAuxiliaryInfo,
    ) {
        self.transactions.push(transaction);
        self.transaction_outputs.push(transaction_output);
        self.persisted_auxiliary_infos
            .push(persisted_auxiliary_info);
    }

    pub fn len(&self) -> usize {
        self.transactions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.transactions.is_empty()
    }

    pub fn iter(
        &self,
    ) -> impl Iterator<Item = (&Transaction, &TransactionOutput, &PersistedAuxiliaryInfo)> {
        izip!(
            self.transactions.iter(),
            self.transaction_outputs.iter(),
            self.persisted_auxiliary_infos.iter()
        )
    }
}

#[ouroboros::self_referencing]
pub struct TransactionsToKeep {
    transactions_with_output: TransactionsWithOutput,
    is_reconfig: bool,
    #[borrows(transactions_with_output)]
    #[covariant]
    state_update_refs: StateUpdateRefs<'this>,
}

impl TransactionsToKeep {
    pub fn index(
        first_version: Version,
        transactions_with_output: TransactionsWithOutput,
        is_reconfig: bool,
    ) -> Self {
        let _timer = TIMER.timer_with(&["transactions_to_keep__index"]);

        TransactionsToKeepBuilder {
            transactions_with_output,
            is_reconfig,
            state_update_refs_builder: |transactions_with_output| {
                let write_sets = transactions_with_output
                    .transaction_outputs
                    .iter()
                    .map(TransactionOutput::write_set);
                let last_checkpoint_index = Self::get_last_checkpoint_index(
                    is_reconfig,
                    &transactions_with_output.transactions,
                );
                StateUpdateRefs::index_write_sets(
                    first_version,
                    write_sets,
                    transactions_with_output.len(),
                    last_checkpoint_index,
                )
            },
        }
        .build()
    }

    pub fn make(
        first_version: Version,
        transactions: Vec<Transaction>,
        transaction_outputs: Vec<TransactionOutput>,
        persisted_auxiliary_infos: Vec<PersistedAuxiliaryInfo>,
        is_reconfig: bool,
    ) -> Self {
        let txns_with_output = TransactionsWithOutput::new(
            transactions,
            transaction_outputs,
            persisted_auxiliary_infos,
        );
        Self::index(first_version, txns_with_output, is_reconfig)
    }

    pub fn new_empty() -> Self {
        Self::make(0, vec![], vec![], vec![], false)
    }

    pub fn new_dummy_success(txns: Vec<Transaction>) -> Self {
        let txn_outputs = vec![TransactionOutput::new_empty_success(); txns.len()];
        let persisted_auxiliary_infos = (0..txns.len())
            .map(|i| PersistedAuxiliaryInfo::V1 {
                transaction_index: i as u32,
            })
            .collect();
        Self::make(0, txns, txn_outputs, persisted_auxiliary_infos, false)
    }

    pub fn is_reconfig(&self) -> bool {
        *self.borrow_is_reconfig()
    }

    pub fn state_update_refs(&self) -> &StateUpdateRefs {
        self.borrow_state_update_refs()
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

    pub fn ensure_at_most_one_checkpoint(&self) -> Result<()> {
        let _timer = TIMER.timer_with(&["unexpected__ensure_at_most_one_checkpoint"]);

        let mut total = self
            .transactions
            .iter()
            .filter(|t| t.is_non_reconfig_block_ending())
            .count();
        if self.is_reconfig() {
            total += self
                .transactions
                .last()
                .map_or(0, |t| !t.is_non_reconfig_block_ending() as usize);
        }

        ensure!(
            total <= 1,
            "Expecting at most one checkpoint, found {}",
            total,
        );
        Ok(())
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

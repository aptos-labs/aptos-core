// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::transaction::{Transaction, TransactionOutput};
use itertools::izip;

#[derive(Debug, Default)]
pub struct TransactionsWithOutput {
    pub transactions: Vec<Transaction>,
    pub transaction_outputs: Vec<TransactionOutput>,
    pub epoch_ending_flags: Vec<bool>,
}

impl TransactionsWithOutput {
    pub fn new(
        transactions: Vec<Transaction>,
        transaction_outputs: Vec<TransactionOutput>,
        epoch_ending_flags: Vec<bool>,
    ) -> Self {
        assert_eq!(transactions.len(), transaction_outputs.len());
        assert_eq!(transactions.len(), epoch_ending_flags.len());
        Self {
            transactions,
            transaction_outputs,
            epoch_ending_flags,
        }
    }

    pub fn new_empty() -> Self {
        Self::default()
    }

    pub fn new_dummy_success(txns: Vec<Transaction>) -> Self {
        let txn_outputs = vec![TransactionOutput::new_empty_success(); txns.len()];
        let epoch_ending_flags = vec![false; txns.len()];
        Self::new(txns, txn_outputs, epoch_ending_flags)
    }

    pub fn push(
        &mut self,
        transaction: Transaction,
        transaction_output: TransactionOutput,
        is_reconfig: bool,
    ) {
        self.transactions.push(transaction);
        self.transaction_outputs.push(transaction_output);
        self.epoch_ending_flags.push(is_reconfig);
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
        (0..self.len())
            .rev()
            .find(|&i| Self::need_checkpoint(&self.transactions[i], self.epoch_ending_flags[i]))
    }

    pub fn need_checkpoint(txn: &Transaction, is_reconfig: bool) -> bool {
        if is_reconfig {
            return true;
        }
        match txn {
            Transaction::BlockMetadata(_)
            | Transaction::BlockMetadataExt(_)
            | Transaction::UserTransaction(_)
            | Transaction::ValidatorTransaction(_) => false,
            Transaction::GenesisTransaction(_)
            | Transaction::StateCheckpoint(_)
            | Transaction::BlockEpilogue(_) => true,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Transaction, &TransactionOutput, bool)> {
        izip!(
            self.transactions.iter(),
            self.transaction_outputs.iter(),
            self.epoch_ending_flags.iter().cloned()
        )
    }
}

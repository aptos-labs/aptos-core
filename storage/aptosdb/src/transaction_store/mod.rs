// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! This file defines transaction store APIs that are related to committed signed transactions.

use crate::ledger_db::LedgerDb;
use aptos_db_indexer_schemas::{
    schema::transaction_by_account::TransactionByAccountSchema,
    utils::AccountTransactionVersionIter,
};
use aptos_schemadb::batch::SchemaBatch;
use aptos_storage_interface::{AptosDbError, Result};
use aptos_types::{
    account_address::AccountAddress,
    transaction::{Transaction, Version},
};
use std::sync::Arc;

#[cfg(test)]
mod test;

#[derive(Clone, Debug)]
pub struct TransactionStore {
    ledger_db: Arc<LedgerDb>,
}

impl TransactionStore {
    pub fn new(ledger_db: Arc<LedgerDb>) -> Self {
        Self { ledger_db }
    }

    /// Gets the version of a transaction by the sender `address` and `sequence_number`.
    pub fn get_account_ordered_transaction_version(
        &self,
        address: AccountAddress,
        sequence_number: u64,
        ledger_version: Version,
    ) -> Result<Option<Version>> {
        if let Some(version) = self
            .ledger_db
            .transaction_db_raw()
            .get::<TransactionByAccountSchema>(&(address, sequence_number))?
        {
            if version <= ledger_version {
                return Ok(Some(version));
            }
        }

        Ok(None)
    }

    /// Gets an iterator that yields `(sequence_number, version)` for each
    /// transaction sent by an account, with minimum sequence number greater
    /// `min_seq_num`, and returning at most `num_versions` results with
    /// `version <= ledger_version`.
    /// Guarantees that the returned sequence numbers are sequential, i.e.,
    /// `seq_num_{i} + 1 = seq_num_{i+1}`.
    pub fn get_account_ordered_transactions_iter(
        &self,
        address: AccountAddress,
        min_seq_num: u64,
        num_versions: u64,
        ledger_version: Version,
    ) -> Result<AccountTransactionVersionIter> {
        let mut iter = self
            .ledger_db
            .transaction_db_raw()
            .iter::<TransactionByAccountSchema>()?;
        iter.seek(&(address, min_seq_num))?;
        Ok(AccountTransactionVersionIter::new(
            iter,
            address,
            min_seq_num
                .checked_add(num_versions)
                .ok_or(AptosDbError::TooManyRequested(min_seq_num, num_versions))?,
            ledger_version,
        ))
    }

    /// Prune the transaction by account store given a list of transaction
    pub fn prune_transaction_by_account(
        &self,
        transactions: &[Transaction],
        db_batch: &mut SchemaBatch,
    ) -> Result<()> {
        for transaction in transactions {
            if let Some(txn) = transaction.try_as_signed_user_txn() {
                db_batch
                    .delete::<TransactionByAccountSchema>(&(txn.sender(), txn.sequence_number()))?;
            }
        }
        Ok(())
    }
}

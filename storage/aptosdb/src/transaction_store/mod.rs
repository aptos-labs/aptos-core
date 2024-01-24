// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! This file defines transaction store APIs that are related to committed signed transactions.

use crate::{
    ledger_db::LedgerDb,
    schema::{transaction_by_account::TransactionByAccountSchema, write_set::WriteSetSchema},
    utils::iterators::{AccountTransactionVersionIter, ExpectContinuousVersions},
};
use aptos_schemadb::{ReadOptions, SchemaBatch};
use aptos_storage_interface::{db_ensure as ensure, AptosDbError, Result};
use aptos_types::{
    account_address::AccountAddress,
    transaction::{Transaction, Version},
    write_set::WriteSet,
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
    pub fn get_account_transaction_version(
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
    pub fn get_account_transaction_version_iter(
        &self,
        address: AccountAddress,
        min_seq_num: u64,
        num_versions: u64,
        ledger_version: Version,
    ) -> Result<AccountTransactionVersionIter> {
        let mut iter = self
            .ledger_db
            .transaction_db_raw()
            .iter::<TransactionByAccountSchema>(ReadOptions::default())?;
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

    /// Gets an iterator that yields `num_transactions` write sets starting from `start_version`.
    pub fn get_write_set_iter(
        &self,
        start_version: Version,
        num_transactions: usize,
    ) -> Result<impl Iterator<Item = Result<WriteSet>> + '_> {
        let mut iter = self
            .ledger_db
            .write_set_db()
            .iter::<WriteSetSchema>(ReadOptions::default())?;
        iter.seek(&start_version)?;
        iter.expect_continuous_versions(start_version, num_transactions)
    }

    /// Get executed transaction vm output given `version`
    pub fn get_write_set(&self, version: Version) -> Result<WriteSet> {
        self.ledger_db
            .write_set_db()
            .get::<WriteSetSchema>(&version)?
            .ok_or(AptosDbError::NotFound(format!(
                "WriteSet at version {}",
                version
            )))
    }

    /// Get write sets in `[begin_version, end_version)` half-open range.
    ///
    /// N.b. an empty `Vec` is returned when `begin_version == end_version`
    pub fn get_write_sets(
        &self,
        begin_version: Version,
        end_version: Version,
    ) -> Result<Vec<WriteSet>> {
        if begin_version == end_version {
            return Ok(Vec::new());
        }
        ensure!(
            begin_version < end_version,
            "begin_version {} >= end_version {}",
            begin_version,
            end_version
        );

        let mut iter = self
            .ledger_db
            .write_set_db()
            .iter::<WriteSetSchema>(Default::default())?;
        iter.seek(&begin_version)?;

        let mut ret = Vec::with_capacity((end_version - begin_version) as usize);
        for current_version in begin_version..end_version {
            let (version, write_set) = iter.next().transpose()?.ok_or_else(|| {
                AptosDbError::NotFound(format!("Write set missing for version {}", current_version))
            })?;
            ensure!(
                version == current_version,
                "Write set missing for version {}, got version {}",
                current_version,
                version,
            );
            ret.push(write_set);
        }

        Ok(ret)
    }

    /// Save executed transaction vm output given `version`
    pub fn put_write_set(
        &self,
        version: Version,
        write_set: &WriteSet,
        batch: &SchemaBatch,
    ) -> Result<()> {
        batch.put::<WriteSetSchema>(&version, write_set)
    }

    /// Prune the transaction by account store given a list of transaction
    pub fn prune_transaction_by_account(
        &self,
        transactions: &[Transaction],
        db_batch: &SchemaBatch,
    ) -> Result<()> {
        for transaction in transactions {
            if let Some(txn) = transaction.try_as_signed_user_txn() {
                db_batch
                    .delete::<TransactionByAccountSchema>(&(txn.sender(), txn.sequence_number()))?;
            }
        }
        Ok(())
    }

    /// Prune the transaction schema store between a range of version in [begin, end)
    pub fn prune_write_set(
        &self,
        begin: Version,
        end: Version,
        db_batch: &SchemaBatch,
    ) -> Result<()> {
        for version in begin..end {
            db_batch.delete::<WriteSetSchema>(&version)?;
        }
        Ok(())
    }
}

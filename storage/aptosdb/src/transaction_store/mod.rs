// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! This file defines transaction store APIs that are related to committed signed transactions.

use crate::transaction_accumulator::TransactionAccumulatorSchema;
use crate::utils::iterators::AccountTransactionVersionIter;
use crate::utils::iterators::ExpectContinuousVersions;
use crate::{
    errors::AptosDbError,
    schema::{
        transaction::TransactionSchema, transaction_by_account::TransactionByAccountSchema,
        transaction_by_hash::TransactionByHashSchema, write_set::WriteSetSchema,
    },
    transaction_info::TransactionInfoSchema,
};
use anyhow::{ensure, format_err, Result};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_types::{
    account_address::AccountAddress,
    proof::position::Position,
    transaction::{Transaction, Version},
    write_set::WriteSet,
};
use schemadb::{ReadOptions, SchemaBatch, DB};
use std::sync::Arc;

#[cfg(test)]
mod test;

#[derive(Clone, Debug)]
pub struct TransactionStore {
    db: Arc<DB>,
}

impl TransactionStore {
    pub fn new(db: Arc<DB>) -> Self {
        Self { db }
    }

    /// Gets the version of a transaction by the sender `address` and `sequence_number`.
    pub fn get_account_transaction_version(
        &self,
        address: AccountAddress,
        sequence_number: u64,
        ledger_version: Version,
    ) -> Result<Option<Version>> {
        if let Some(version) = self
            .db
            .get::<TransactionByAccountSchema>(&(address, sequence_number))?
        {
            if version <= ledger_version {
                return Ok(Some(version));
            }
        }

        Ok(None)
    }

    /// Gets the version of a transaction by its hash.
    pub fn get_transaction_version_by_hash(
        &self,
        hash: &HashValue,
        ledger_version: Version,
    ) -> Result<Option<Version>> {
        Ok(match self.db.get::<TransactionByHashSchema>(hash)? {
            Some(version) if version <= ledger_version => Some(version),
            _ => None,
        })
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
            .db
            .iter::<TransactionByAccountSchema>(ReadOptions::default())?;
        iter.seek(&(address, min_seq_num))?;
        Ok(AccountTransactionVersionIter::new(
            iter,
            address,
            min_seq_num
                .checked_add(num_versions)
                .ok_or_else(|| format_err!("too many transactions requested"))?,
            ledger_version,
        ))
    }

    /// Get signed transaction given `version`
    pub fn get_transaction(&self, version: Version) -> Result<Transaction> {
        self.db
            .get::<TransactionSchema>(&version)?
            .ok_or_else(|| AptosDbError::NotFound(format!("Txn {}", version)).into())
    }

    /// Gets an iterator that yields at most `num_transactions` transactions starting from `start_version`.
    pub fn get_transaction_iter(
        &self,
        start_version: Version,
        num_transactions: usize,
    ) -> Result<impl Iterator<Item = Result<Transaction>> + '_> {
        let mut iter = self.db.iter::<TransactionSchema>(ReadOptions::default())?;
        iter.seek(&start_version)?;
        iter.expect_continuous_versions(start_version, num_transactions)
    }

    /// Gets an iterator that yields `num_transactions` write sets starting from `start_version`.
    pub fn get_write_set_iter(
        &self,
        start_version: Version,
        num_transactions: usize,
    ) -> Result<impl Iterator<Item = Result<WriteSet>> + '_> {
        let mut iter = self.db.iter::<WriteSetSchema>(ReadOptions::default())?;
        iter.seek(&start_version)?;
        iter.expect_continuous_versions(start_version, num_transactions)
    }

    /// Save signed transaction at `version`
    pub fn put_transaction(
        &self,
        version: Version,
        transaction: &Transaction,
        batch: &mut SchemaBatch,
    ) -> Result<()> {
        if let Transaction::UserTransaction(txn) = transaction {
            batch.put::<TransactionByAccountSchema>(
                &(txn.sender(), txn.sequence_number()),
                &version,
            )?;
        }
        batch.put::<TransactionByHashSchema>(&transaction.hash(), &version)?;
        batch.put::<TransactionSchema>(&version, transaction)?;

        Ok(())
    }

    /// Get executed transaction vm output given `version`
    pub fn get_write_set(&self, version: Version) -> Result<WriteSet> {
        self.db.get::<WriteSetSchema>(&version)?.ok_or_else(|| {
            AptosDbError::NotFound(format!("WriteSet at version {}", version)).into()
        })
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

        let mut iter = self.db.iter::<WriteSetSchema>(Default::default())?;
        iter.seek(&begin_version)?;

        let mut ret = Vec::with_capacity((end_version - begin_version) as usize);
        for current_version in begin_version..end_version {
            let (version, write_set) = iter
                .next()
                .transpose()?
                .ok_or_else(|| format_err!("Write set missing for version {}", current_version))?;
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
        batch: &mut SchemaBatch,
    ) -> Result<()> {
        batch.put::<WriteSetSchema>(&version, write_set)
    }

    /// Prune the transaction by hash store given a list of transaction
    pub fn prune_transaction_by_hash(
        &self,
        transactions: &[Transaction],
        db_batch: &mut SchemaBatch,
    ) -> Result<()> {
        for transaction in transactions {
            db_batch.delete::<TransactionByHashSchema>(&transaction.hash())?;
        }
        Ok(())
    }

    /// Prune the transaction by account store given a list of transaction
    pub fn prune_transaction_by_account(
        &self,
        transactions: &[Transaction],
        db_batch: &mut SchemaBatch,
    ) -> Result<()> {
        for transaction in transactions {
            if let Transaction::UserTransaction(txn) = transaction {
                db_batch
                    .delete::<TransactionByAccountSchema>(&(txn.sender(), txn.sequence_number()))?;
            }
        }
        Ok(())
    }

    /// Prune the transaction schema store between a range of version in [begin, end)
    pub fn prune_transaction_schema(
        &self,
        begin: Version,
        end: Version,
        db_batch: &mut SchemaBatch,
    ) -> Result<()> {
        for version in begin..end {
            db_batch.delete::<TransactionSchema>(&version)?;
        }
        Ok(())
    }

    /// Prune the transaction schema store between a range of version in [begin, end)
    pub fn prune_transaction_info_schema(
        &self,
        begin: Version,
        end: Version,
        db_batch: &mut SchemaBatch,
    ) -> Result<()> {
        for version in begin..end {
            db_batch.delete::<TransactionInfoSchema>(&version)?;
        }
        Ok(())
    }

    /// Prune the transaction accumulator between a range of version in [begin, end).
    ///
    /// To avoid always pruning a full left subtree, we uses the following algorithm.
    /// For each leaf with an odd leaf index.
    /// 1. From the bottom upwards, find the first ancestor that's a left child of its parent.
    /// (the position of which can be got by popping "1"s from the right of the leaf address).
    /// Note that this node DOES NOT become non-useful.
    /// 2. From the node found from the previous step, delete both its children non-useful, and go
    /// to the right child to repeat the process until we reach a leaf node.
    /// More details are in this issue https://github.com/aptos-labs/aptos-core/issues/1288.
    pub fn prune_transaction_accumulator(
        &self,
        begin: Version,
        end: Version,
        db_batch: &mut SchemaBatch,
    ) -> Result<()> {
        for version_to_delete in begin..end {
            // The even version will be pruned in the iteration of version + 1.
            if version_to_delete % 2 == 0 {
                continue;
            }

            let first_ancestor_that_is_a_left_child =
                self.find_first_ancestor_that_is_a_left_child(version_to_delete);

            // This assertion is true because we skip the leaf nodes with address which is a
            // a multiple of 2.
            assert!(!first_ancestor_that_is_a_left_child.is_leaf());

            let mut current = first_ancestor_that_is_a_left_child;
            while !current.is_leaf() {
                db_batch.delete::<TransactionAccumulatorSchema>(&current.left_child())?;
                db_batch.delete::<TransactionAccumulatorSchema>(&current.right_child())?;
                current = current.right_child();
            }
        }
        Ok(())
    }

    /// Finds the first ancestor that is a child of its parent.
    fn find_first_ancestor_that_is_a_left_child(&self, version: Version) -> Position {
        // We can get the first ancestor's position based on the two observations:
        // - floor(level position of a node / 2) = level position of its parent.
        // - if a node is a left child of its parent, its level position should be a multiple of 2.
        // - level position means the position counted from 0 of a single tree level. For example,
        //                a (level position = 0)
        //         /                                \
        //    b (level position = 0)      c(level position = 1)
        //
        // To find the first ancestor which is a left child of its parent, we can keep diving the
        // version by 2 (to find the ancestor) until we get a number which is a multiple of 2
        // (to make sure the ancestor is a left child of its parent). The number of time we
        // divide the version is the level of the ancestor. The remainder is the level position
        // of the ancestor.
        let first_ancestor_that_is_a_left_child_level = version.trailing_ones();
        let index_in_level = version >> first_ancestor_that_is_a_left_child_level;
        Position::from_level_and_pos(first_ancestor_that_is_a_left_child_level, index_in_level)
    }

    /// Prune the transaction schema store between a range of version in [begin, end)
    pub fn prune_write_set(
        &self,
        begin: Version,
        end: Version,
        db_batch: &mut SchemaBatch,
    ) -> Result<()> {
        for version in begin..end {
            db_batch.delete::<WriteSetSchema>(&version)?;
        }
        Ok(())
    }
}

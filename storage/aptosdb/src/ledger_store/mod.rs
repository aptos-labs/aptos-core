// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! This file defines ledger store APIs that are related to the main ledger accumulator, from the
//! root(LedgerInfo) to leaf(TransactionInfo).

use crate::{
    ledger_db::LedgerDb,
    schema::{
        transaction_accumulator::TransactionAccumulatorSchema,
        transaction_info::TransactionInfoSchema,
    },
    utils::iterators::ExpectContinuousVersions,
};
use anyhow::anyhow;
use aptos_accumulator::{HashReader, MerkleAccumulator};
use aptos_crypto::{
    hash::{CryptoHash, TransactionAccumulatorHasher},
    HashValue,
};
use aptos_schemadb::{ReadOptions, SchemaBatch};
use aptos_storage_interface::{AptosDbError, Result};
use aptos_types::{
    proof::{
        definition::LeafCount, position::Position, AccumulatorConsistencyProof,
        TransactionAccumulatorProof, TransactionAccumulatorRangeProof, TransactionInfoWithProof,
    },
    transaction::{TransactionInfo, TransactionToCommit, Version},
};
use itertools::Itertools;
use std::{borrow::Borrow, sync::Arc};

#[derive(Debug)]
pub struct LedgerStore {
    pub ledger_db: Arc<LedgerDb>,
}

impl LedgerStore {
    pub fn new(ledger_db: Arc<LedgerDb>) -> Self {
        Self { ledger_db }
    }

    pub fn get_frozen_subtree_hashes(&self, num_transactions: LeafCount) -> Result<Vec<HashValue>> {
        Accumulator::get_frozen_subtree_hashes(self, num_transactions).map_err(Into::into)
    }

    /// Get transaction info given `version`
    pub fn get_transaction_info(&self, version: Version) -> Result<TransactionInfo> {
        self.ledger_db
            .transaction_info_db()
            .get::<TransactionInfoSchema>(&version)?
            .ok_or_else(|| {
                AptosDbError::NotFound(format!("No TransactionInfo at version {}", version))
            })
    }

    /// Gets an iterator that yields `num_transaction_infos` transaction infos starting from
    /// `start_version`.
    pub(crate) fn get_transaction_info_iter(
        &self,
        start_version: Version,
        num_transaction_infos: usize,
    ) -> Result<impl Iterator<Item = Result<TransactionInfo>> + '_> {
        let mut iter = self
            .ledger_db
            .transaction_info_db()
            .iter::<TransactionInfoSchema>(ReadOptions::default())?;
        iter.seek(&start_version)?;
        iter.expect_continuous_versions(start_version, num_transaction_infos)
    }

    /// Get transaction info at `version` with proof towards root of ledger at `ledger_version`.
    pub fn get_transaction_info_with_proof(
        &self,
        version: Version,
        ledger_version: Version,
    ) -> Result<TransactionInfoWithProof> {
        Ok(TransactionInfoWithProof::new(
            self.get_transaction_proof(version, ledger_version)?,
            self.get_transaction_info(version)?,
        ))
    }

    /// Get proof for transaction at `version` towards root of ledger at `ledger_version`.
    pub fn get_transaction_proof(
        &self,
        version: Version,
        ledger_version: Version,
    ) -> Result<TransactionAccumulatorProof> {
        Accumulator::get_proof(self, ledger_version + 1 /* num_leaves */, version)
            .map_err(Into::into)
    }

    /// Get proof for `num_txns` consecutive transactions starting from `start_version` towards
    /// root of ledger at `ledger_version`.
    pub fn get_transaction_range_proof(
        &self,
        start_version: Option<Version>,
        num_txns: u64,
        ledger_version: Version,
    ) -> Result<TransactionAccumulatorRangeProof> {
        Accumulator::get_range_proof(
            self,
            ledger_version + 1, /* num_leaves */
            start_version,
            num_txns,
        )
        .map_err(Into::into)
    }

    /// Gets proof that shows the ledger at `ledger_version` is consistent with the ledger at
    /// `client_known_version`.
    pub fn get_consistency_proof(
        &self,
        client_known_version: Option<Version>,
        ledger_version: Version,
    ) -> Result<AccumulatorConsistencyProof> {
        let client_known_num_leaves = client_known_version
            .map(|v| v.saturating_add(1))
            .unwrap_or(0);
        let ledger_num_leaves = ledger_version.saturating_add(1);
        Accumulator::get_consistency_proof(self, ledger_num_leaves, client_known_num_leaves)
            .map_err(Into::into)
    }

    /// Write `txn_infos` to `batch`. Assigned `first_version` to the version number of the
    /// first transaction, and so on.
    pub fn put_transaction_infos(
        &self,
        first_version: u64,
        txn_infos: &[TransactionInfo],
        // TODO(grao): Consider remove this function and migrate all callers to use the two functions
        // below.
        transaction_info_batch: &SchemaBatch,
        transaction_accumulator_batch: &SchemaBatch,
    ) -> Result<HashValue> {
        // write txn_info
        (first_version..first_version + txn_infos.len() as u64)
            .zip_eq(txn_infos.iter())
            .try_for_each(|(version, txn_info)| {
                transaction_info_batch.put::<TransactionInfoSchema>(&version, txn_info)
            })?;

        // write hash of txn_info into the accumulator
        let txn_hashes: Vec<HashValue> = txn_infos.iter().map(TransactionInfo::hash).collect();
        let (root_hash, writes) = Accumulator::append(
            self,
            first_version, /* num_existing_leaves */
            &txn_hashes,
        )?;
        writes.iter().try_for_each(|(pos, hash)| {
            transaction_accumulator_batch.put::<TransactionAccumulatorSchema>(pos, hash)
        })?;
        Ok(root_hash)
    }

    pub fn put_transaction_accumulator(
        &self,
        first_version: Version,
        txns_to_commit: &[impl Borrow<TransactionToCommit>],
        transaction_accumulator_batch: &SchemaBatch,
    ) -> Result<HashValue> {
        let txn_hashes: Vec<_> = txns_to_commit
            .iter()
            .map(|t| t.borrow().transaction_info().hash())
            .collect();

        let (root_hash, writes) = Accumulator::append(
            self,
            first_version, /* num_existing_leaves */
            &txn_hashes,
        )?;
        writes.iter().try_for_each(|(pos, hash)| {
            transaction_accumulator_batch.put::<TransactionAccumulatorSchema>(pos, hash)
        })?;

        Ok(root_hash)
    }

    pub fn put_transaction_info(
        &self,
        version: Version,
        transaction_info: &TransactionInfo,
        transaction_info_batch: &SchemaBatch,
    ) -> Result<()> {
        transaction_info_batch.put::<TransactionInfoSchema>(&version, transaction_info)
    }

    pub fn get_root_hash(&self, version: Version) -> Result<HashValue> {
        Accumulator::get_root_hash(self, version + 1).map_err(Into::into)
    }
}

pub(crate) type Accumulator = MerkleAccumulator<LedgerStore, TransactionAccumulatorHasher>;

impl HashReader for LedgerStore {
    fn get(&self, position: Position) -> Result<HashValue, anyhow::Error> {
        self.ledger_db
            .transaction_accumulator_db()
            .get::<TransactionAccumulatorSchema>(&position)?
            .ok_or_else(|| anyhow!("{} does not exist.", position))
    }
}

#[cfg(test)]
mod transaction_info_test;

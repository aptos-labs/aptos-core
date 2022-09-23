// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! This file defines ledger store APIs that are related to the main ledger accumulator, from the
//! root(LedgerInfo) to leaf(TransactionInfo).

use crate::utils::iterators::{EpochEndingLedgerInfoIter, ExpectContinuousVersions};
use crate::{
    errors::AptosDbError,
    schema::{
        epoch_by_version::EpochByVersionSchema, ledger_info::LedgerInfoSchema,
        transaction_accumulator::TransactionAccumulatorSchema,
        transaction_info::TransactionInfoSchema,
    },
};
use accumulator::{HashReader, MerkleAccumulator};
use anyhow::{ensure, format_err, Result};
use aptos_crypto::{
    hash::{CryptoHash, TransactionAccumulatorHasher},
    HashValue,
};
use aptos_types::{
    epoch_state::EpochState,
    ledger_info::LedgerInfoWithSignatures,
    proof::{
        definition::LeafCount, position::Position, AccumulatorConsistencyProof,
        TransactionAccumulatorProof, TransactionAccumulatorRangeProof, TransactionInfoWithProof,
    },
    transaction::{TransactionInfo, Version},
};
use arc_swap::ArcSwap;
use itertools::Itertools;
use schemadb::{ReadOptions, SchemaBatch, DB};
use std::{ops::Deref, sync::Arc};

#[derive(Debug)]
pub struct LedgerStore {
    db: Arc<DB>,

    /// We almost always need the latest ledger info and signatures to serve read requests, so we
    /// cache it in memory in order to avoid reading DB and deserializing the object frequently. It
    /// should be updated every time new ledger info and signatures are persisted.
    latest_ledger_info: ArcSwap<Option<LedgerInfoWithSignatures>>,
}

impl LedgerStore {
    pub fn new(db: Arc<DB>) -> Self {
        // Upon restart, read the latest ledger info and signatures and cache them in memory.
        let ledger_info = {
            let mut iter = db
                .iter::<LedgerInfoSchema>(ReadOptions::default())
                .expect("Constructing iterator should work.");
            iter.seek_to_last();
            iter.next()
                .transpose()
                .expect("Reading latest ledger info from DB should work.")
                .map(|kv| kv.1)
        };

        Self {
            db,
            latest_ledger_info: ArcSwap::from(Arc::new(ledger_info)),
        }
    }

    pub fn get_epoch(&self, version: Version) -> Result<u64> {
        let mut iter = self
            .db
            .iter::<EpochByVersionSchema>(ReadOptions::default())?;
        // Search for the end of the previous epoch.
        iter.seek_for_prev(&version)?;
        let (epoch_end_version, epoch) = match iter.next().transpose()? {
            Some(x) => x,
            None => {
                // There should be a genesis LedgerInfo at version 0 (genesis only consists of one
                // transaction), so this normally doesn't happen. However this part of
                // implementation doesn't need to rely on this assumption.
                return Ok(0);
            }
        };
        ensure!(
            epoch_end_version <= version,
            "DB corruption: looking for epoch for version {}, got epoch {} ends at version {}",
            version,
            epoch,
            epoch_end_version
        );
        // If the obtained epoch ended before the given version, return epoch+1, otherwise
        // the given version is exactly the last version of the found epoch.
        Ok(if epoch_end_version < version {
            epoch + 1
        } else {
            epoch
        })
    }

    /// Gets ledger info at specified version and ensures it's an epoch ending.
    pub fn get_epoch_ending_ledger_info(
        &self,
        version: Version,
    ) -> Result<LedgerInfoWithSignatures> {
        let epoch = self.get_epoch(version)?;
        let li = self
            .db
            .get::<LedgerInfoSchema>(&epoch)?
            .ok_or_else(|| AptosDbError::NotFound(format!("LedgerInfo for epoch {}.", epoch)))?;
        ensure!(
            li.ledger_info().version() == version,
            "Epoch {} didn't end at version {}",
            epoch,
            version,
        );
        li.ledger_info()
            .next_epoch_state()
            .ok_or_else(|| format_err!("Not an epoch change at version {}", version))?;

        Ok(li)
    }

    pub fn get_latest_ledger_info_option(&self) -> Option<LedgerInfoWithSignatures> {
        let ledger_info_ptr = self.latest_ledger_info.load();
        let ledger_info: &Option<_> = ledger_info_ptr.deref();
        ledger_info.clone()
    }

    pub fn get_latest_ledger_info(&self) -> Result<LedgerInfoWithSignatures> {
        self.get_latest_ledger_info_option()
            .ok_or_else(|| AptosDbError::NotFound(String::from("Genesis LedgerInfo")).into())
    }

    pub fn set_latest_ledger_info(&self, ledger_info_with_sigs: LedgerInfoWithSignatures) {
        self.latest_ledger_info
            .store(Arc::new(Some(ledger_info_with_sigs)));
    }

    pub fn get_latest_ledger_info_in_epoch(&self, epoch: u64) -> Result<LedgerInfoWithSignatures> {
        self.db.get::<LedgerInfoSchema>(&epoch)?.ok_or_else(|| {
            AptosDbError::NotFound(format!("Last LedgerInfo of epoch {}", epoch)).into()
        })
    }

    pub fn get_epoch_state(&self, epoch: u64) -> Result<EpochState> {
        ensure!(epoch > 0, "EpochState only queryable for epoch >= 1.",);

        let ledger_info_with_sigs =
            self.db
                .get::<LedgerInfoSchema>(&(epoch - 1))?
                .ok_or_else(|| {
                    AptosDbError::NotFound(format!("Last LedgerInfo of epoch {}", epoch - 1))
                })?;
        let latest_epoch_state = ledger_info_with_sigs
            .ledger_info()
            .next_epoch_state()
            .ok_or_else(|| format_err!("Last LedgerInfo in epoch must carry next_epoch_state."))?;

        Ok(latest_epoch_state.clone())
    }

    pub fn get_frozen_subtree_hashes(&self, num_transactions: LeafCount) -> Result<Vec<HashValue>> {
        Accumulator::get_frozen_subtree_hashes(self, num_transactions)
    }

    /// Get transaction info given `version`
    pub fn get_transaction_info(&self, version: Version) -> Result<TransactionInfo> {
        self.db
            .get::<TransactionInfoSchema>(&version)?
            .ok_or_else(|| format_err!("No TransactionInfo at version {}", version))
    }

    pub fn get_latest_transaction_info_option(&self) -> Result<Option<(Version, TransactionInfo)>> {
        let mut iter = self
            .db
            .iter::<TransactionInfoSchema>(ReadOptions::default())?;
        iter.seek_to_last();
        iter.next().transpose()
    }

    /// Get latest transaction info together with its version. Note that during node syncing, this
    /// version can be greater than what's in the latest LedgerInfo.
    pub fn get_latest_transaction_info(&self) -> Result<(Version, TransactionInfo)> {
        self.get_latest_transaction_info_option()?
            .ok_or_else(|| AptosDbError::NotFound(String::from("Genesis TransactionInfo.")).into())
    }

    /// Gets an iterator that yields `num_transaction_infos` transaction infos starting from
    /// `start_version`.
    pub(crate) fn get_transaction_info_iter(
        &self,
        start_version: Version,
        num_transaction_infos: usize,
    ) -> Result<impl Iterator<Item = Result<TransactionInfo>> + '_> {
        let mut iter = self
            .db
            .iter::<TransactionInfoSchema>(ReadOptions::default())?;
        iter.seek(&start_version)?;
        iter.expect_continuous_versions(start_version, num_transaction_infos)
    }

    /// Gets an iterator that yields epoch ending ledger infos, starting
    /// from `start_epoch`, and ends at the one before `end_epoch`
    pub fn get_epoch_ending_ledger_info_iter(
        &self,
        start_epoch: u64,
        end_epoch: u64,
    ) -> Result<EpochEndingLedgerInfoIter> {
        let mut iter = self.db.iter::<LedgerInfoSchema>(ReadOptions::default())?;
        iter.seek(&start_epoch)?;
        Ok(EpochEndingLedgerInfoIter::new(iter, start_epoch, end_epoch))
    }

    pub fn ensure_epoch_ending(&self, version: Version) -> Result<()> {
        self.db
            .get::<EpochByVersionSchema>(&version)?
            .ok_or_else(|| format_err!("Version {} is not epoch ending.", version))?;
        Ok(())
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
    }

    /// Write `txn_infos` to `batch`. Assigned `first_version` to the version number of the
    /// first transaction, and so on.
    pub fn put_transaction_infos(
        &self,
        first_version: u64,
        txn_infos: &[TransactionInfo],
        batch: &mut SchemaBatch,
    ) -> Result<HashValue> {
        // write txn_info
        (first_version..first_version + txn_infos.len() as u64)
            .zip_eq(txn_infos.iter())
            .try_for_each(|(version, txn_info)| {
                batch.put::<TransactionInfoSchema>(&version, txn_info)
            })?;

        // write hash of txn_info into the accumulator
        let txn_hashes: Vec<HashValue> = txn_infos.iter().map(TransactionInfo::hash).collect();
        let (root_hash, writes) = Accumulator::append(
            self,
            first_version, /* num_existing_leaves */
            &txn_hashes,
        )?;
        writes
            .iter()
            .try_for_each(|(pos, hash)| batch.put::<TransactionAccumulatorSchema>(pos, hash))?;
        Ok(root_hash)
    }

    /// Write `ledger_info_with_sigs` to `batch`.
    pub fn put_ledger_info(
        &self,
        ledger_info_with_sigs: &LedgerInfoWithSignatures,
        batch: &mut SchemaBatch,
    ) -> Result<()> {
        let ledger_info = ledger_info_with_sigs.ledger_info();

        if ledger_info.ends_epoch() {
            // This is the last version of the current epoch, update the epoch by version index.
            batch.put::<EpochByVersionSchema>(&ledger_info.version(), &ledger_info.epoch())?;
        }
        batch.put::<LedgerInfoSchema>(&ledger_info.epoch(), ledger_info_with_sigs)
    }

    pub fn get_root_hash(&self, version: Version) -> Result<HashValue> {
        Accumulator::get_root_hash(self, version + 1)
    }
}

pub(crate) type Accumulator = MerkleAccumulator<LedgerStore, TransactionAccumulatorHasher>;

impl HashReader for LedgerStore {
    fn get(&self, position: Position) -> Result<HashValue> {
        self.db
            .get::<TransactionAccumulatorSchema>(&position)?
            .ok_or_else(|| format_err!("{} does not exist.", position))
    }
}

#[cfg(test)]
mod ledger_info_test;
#[cfg(test)]
pub(crate) mod ledger_info_test_utils;
#[cfg(test)]
mod transaction_info_test;

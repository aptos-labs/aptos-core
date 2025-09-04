// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    ledger_db::LedgerDb,
    metrics::{
        BACKUP_EPOCH_ENDING_EPOCH, BACKUP_STATE_SNAPSHOT_LEAF_IDX, BACKUP_STATE_SNAPSHOT_VERSION,
        BACKUP_TXN_VERSION,
    },
    state_store::StateStore,
};
use aptos_crypto::hash::HashValue;
use aptos_storage_interface::{AptosDbError, Result, db_ensure as ensure};
use aptos_types::{
    contract_event::ContractEvent,
    ledger_info::LedgerInfoWithSignatures,
    proof::{SparseMerkleRangeProof, TransactionAccumulatorRangeProof, TransactionInfoWithProof},
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::{PersistedAuxiliaryInfo, Transaction, TransactionInfo, Version},
    write_set::WriteSet,
};
use serde::{Deserialize, Serialize};
use std::{fmt, sync::Arc};

/// `BackupHandler` provides functionalities for AptosDB data backup.
#[derive(Clone)]
pub struct BackupHandler {
    state_store: Arc<StateStore>,
    ledger_db: Arc<LedgerDb>,
}

impl BackupHandler {
    pub(crate) fn new(state_store: Arc<StateStore>, ledger_db: Arc<LedgerDb>) -> Self {
        Self {
            state_store,
            ledger_db,
        }
    }

    /// Gets an iterator that yields a range of transactions.
    pub fn get_transaction_iter(
        &self,
        start_version: Version,
        num_transactions: usize,
    ) -> Result<
        impl Iterator<
            Item = Result<(
                Transaction,
                PersistedAuxiliaryInfo,
                TransactionInfo,
                Vec<ContractEvent>,
                WriteSet,
            )>,
        > + '_,
    > {
        let txn_iter = self
            .ledger_db
            .transaction_db()
            .get_transaction_iter(start_version, num_transactions)?;
        let mut txn_info_iter = self
            .ledger_db
            .transaction_info_db()
            .get_transaction_info_iter(start_version, num_transactions)?;
        let mut event_vec_iter = self
            .ledger_db
            .event_db()
            .get_events_by_version_iter(start_version, num_transactions)?;
        let mut write_set_iter = self
            .ledger_db
            .write_set_db()
            .get_write_set_iter(start_version, num_transactions)?;
        let mut persisted_aux_info_iter = self
            .ledger_db
            .persisted_auxiliary_info_db()
            .get_persisted_auxiliary_info_iter(start_version, num_transactions)?;

        let zipped = txn_iter.enumerate().map(move |(idx, txn_res)| {
            let version = start_version + idx as u64; // overflow is impossible since it's check upon txn_iter construction.

            let txn = txn_res?;
            let txn_info = txn_info_iter.next().ok_or_else(|| {
                AptosDbError::NotFound(format!(
                    "TransactionInfo not found when Transaction exists, version {}",
                    version
                ))
            })??;
            let event_vec = event_vec_iter.next().ok_or_else(|| {
                AptosDbError::NotFound(format!(
                    "Events not found when Transaction exists., version {}",
                    version
                ))
            })??;
            let write_set = write_set_iter.next().ok_or_else(|| {
                AptosDbError::NotFound(format!(
                    "WriteSet not found when Transaction exists, version {}",
                    version
                ))
            })??;
            let persisted_aux_info = persisted_aux_info_iter.next().ok_or_else(|| {
                AptosDbError::NotFound(format!(
                    "PersistedAuxiliaryInfo not found when Transaction exists, version {}",
                    version
                ))
            })??;
            BACKUP_TXN_VERSION.set(version as i64);
            Ok((txn, persisted_aux_info, txn_info, event_vec, write_set))
        });
        Ok(zipped)
    }

    /// Gets the proof for a transaction chunk.
    /// N.B. the `LedgerInfo` returned will always be in the same epoch of the `last_version`.
    pub fn get_transaction_range_proof(
        &self,
        first_version: Version,
        last_version: Version,
    ) -> Result<(TransactionAccumulatorRangeProof, LedgerInfoWithSignatures)> {
        ensure!(
            last_version >= first_version,
            "Bad transaction range: [{}, {}]",
            first_version,
            last_version
        );
        let num_transactions = last_version - first_version + 1;
        let ledger_metadata_db = self.ledger_db.metadata_db();
        let epoch = ledger_metadata_db.get_epoch(last_version)?;
        let ledger_info = ledger_metadata_db.get_latest_ledger_info_in_epoch(epoch)?;
        let accumulator_proof = self
            .ledger_db
            .transaction_accumulator_db()
            .get_transaction_range_proof(
                Some(first_version),
                num_transactions,
                ledger_info.ledger_info().version(),
            )?;
        Ok((accumulator_proof, ledger_info))
    }

    /// Gets the number of items in a state snapshot.
    pub fn get_state_item_count(&self, version: Version) -> Result<usize> {
        self.state_store.get_value_count(version)
    }

    /// Iterate through items in a state snapshot
    pub fn get_state_item_iter(
        &self,
        version: Version,
        start_idx: usize,
        limit: usize,
    ) -> Result<impl Iterator<Item = Result<(StateKey, StateValue)>> + Send> {
        let iterator = self
            .state_store
            .get_state_key_and_value_iter(version, start_idx)?
            .take(limit)
            .enumerate()
            .map(move |(idx, res)| {
                BACKUP_STATE_SNAPSHOT_VERSION.set(version as i64);
                BACKUP_STATE_SNAPSHOT_LEAF_IDX.set((start_idx + idx) as i64);
                res
            });
        Ok(Box::new(iterator))
    }

    /// Gets the proof that proves a range of accounts.
    pub fn get_account_state_range_proof(
        &self,
        rightmost_key: HashValue,
        version: Version,
    ) -> Result<SparseMerkleRangeProof> {
        self.state_store
            .get_value_range_proof(rightmost_key, version)
    }

    /// Gets the epoch, committed version, and synced version of the DB.
    pub fn get_db_state(&self) -> Result<Option<DbState>> {
        Ok(self
            .ledger_db
            .metadata_db()
            .get_latest_ledger_info_option()
            .map(|li| DbState {
                epoch: li.ledger_info().epoch(),
                committed_version: li.ledger_info().version(),
            }))
    }

    /// Gets the proof of the state root at specified version.
    /// N.B. the `LedgerInfo` returned will always be in the same epoch of the version.
    pub fn get_state_root_proof(
        &self,
        version: Version,
    ) -> Result<(TransactionInfoWithProof, LedgerInfoWithSignatures)> {
        let ledger_metadata_db = self.ledger_db.metadata_db();
        let epoch = ledger_metadata_db.get_epoch(version)?;
        let ledger_info = ledger_metadata_db.get_latest_ledger_info_in_epoch(epoch)?;
        let txn_info = self
            .ledger_db
            .transaction_info_db()
            .get_transaction_info_with_proof(
                version,
                ledger_info.ledger_info().version(),
                self.ledger_db.transaction_accumulator_db(),
            )?;

        Ok((txn_info, ledger_info))
    }

    pub fn get_epoch_ending_ledger_info_iter(
        &self,
        start_epoch: u64,
        end_epoch: u64,
    ) -> Result<impl Iterator<Item = Result<LedgerInfoWithSignatures>> + '_> {
        Ok(self
            .ledger_db
            .metadata_db()
            .get_epoch_ending_ledger_info_iter(start_epoch, end_epoch)?
            .enumerate()
            .map(move |(idx, li)| {
                BACKUP_EPOCH_ENDING_EPOCH.set((start_epoch + idx as u64) as i64);
                li
            }))
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct DbState {
    pub epoch: u64,
    pub committed_version: Version,
}

impl fmt::Display for DbState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "epoch: {}, committed_version: {}",
            self.epoch, self.committed_version,
        )
    }
}

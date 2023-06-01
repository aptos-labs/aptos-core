// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    backup::restore_utils,
    db_metadata::{DbMetadataKey, DbMetadataSchema},
    event_store::EventStore,
    ledger_store::LedgerStore,
    state_restore::{StateSnapshotRestore, StateSnapshotRestoreMode},
    state_store::StateStore,
    transaction_store::TransactionStore,
    AptosDB,
};
use anyhow::Result;
use aptos_crypto::HashValue;
use aptos_storage_interface::DbReader;
use aptos_types::{
    contract_event::ContractEvent,
    ledger_info::LedgerInfoWithSignatures,
    proof::definition::LeafCount,
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::{Transaction, TransactionInfo, Version},
    write_set::WriteSet,
};
use std::sync::Arc;

/// Provides functionalities for AptosDB data restore.
#[derive(Clone)]
pub struct RestoreHandler {
    pub aptosdb: Arc<AptosDB>,
    ledger_store: Arc<LedgerStore>,
    transaction_store: Arc<TransactionStore>,
    state_store: Arc<StateStore>,
    event_store: Arc<EventStore>,
}

impl RestoreHandler {
    pub(crate) fn new(
        aptosdb: Arc<AptosDB>,
        ledger_store: Arc<LedgerStore>,
        transaction_store: Arc<TransactionStore>,
        state_store: Arc<StateStore>,
        event_store: Arc<EventStore>,
    ) -> Self {
        Self {
            aptosdb,
            ledger_store,
            transaction_store,
            state_store,
            event_store,
        }
    }

    pub fn get_state_restore_receiver(
        &self,
        version: Version,
        expected_root_hash: HashValue,
        restore_mode: StateSnapshotRestoreMode,
    ) -> Result<StateSnapshotRestore<StateKey, StateValue>> {
        StateSnapshotRestore::new(
            &self.state_store.state_merkle_db,
            &self.state_store,
            version,
            expected_root_hash,
            true, /* async_commit */
            restore_mode,
        )
    }

    pub fn reset_state_store(&self) {
        self.state_store.reset();
    }

    pub fn save_ledger_infos(&self, ledger_infos: &[LedgerInfoWithSignatures]) -> Result<()> {
        restore_utils::save_ledger_infos(
            self.aptosdb.ledger_db.metadata_db(),
            self.ledger_store.clone(),
            ledger_infos,
            None,
        )
    }

    pub fn confirm_or_save_frozen_subtrees(
        &self,
        num_leaves: LeafCount,
        frozen_subtrees: &[HashValue],
    ) -> Result<()> {
        restore_utils::confirm_or_save_frozen_subtrees(
            self.aptosdb.ledger_db.transaction_accumulator_db(),
            num_leaves,
            frozen_subtrees,
            None,
        )
    }

    pub fn save_transactions(
        &self,
        first_version: Version,
        txns: &[Transaction],
        txn_infos: &[TransactionInfo],
        events: &[Vec<ContractEvent>],
        write_sets: Vec<WriteSet>,
    ) -> Result<()> {
        restore_utils::save_transactions(
            self.ledger_store.clone(),
            self.transaction_store.clone(),
            self.event_store.clone(),
            self.state_store.clone(),
            first_version,
            txns,
            txn_infos,
            events,
            write_sets,
            None,
            false,
        )
    }

    pub fn save_transactions_and_replay_kv(
        &self,
        first_version: Version,
        txns: &[Transaction],
        txn_infos: &[TransactionInfo],
        events: &[Vec<ContractEvent>],
        write_sets: Vec<WriteSet>,
    ) -> Result<()> {
        restore_utils::save_transactions(
            self.ledger_store.clone(),
            self.transaction_store.clone(),
            self.event_store.clone(),
            self.state_store.clone(),
            first_version,
            txns,
            txn_infos,
            events,
            write_sets,
            None,
            true,
        )
    }

    pub fn get_next_expected_transaction_version(&self) -> Result<Version> {
        Ok(self
            .aptosdb
            .get_latest_transaction_info_option()?
            .map_or(0, |(ver, _txn_info)| ver + 1))
    }

    pub fn get_state_snapshot_before(
        &self,
        version: Version,
    ) -> Result<Option<(Version, HashValue)>> {
        self.aptosdb.get_state_snapshot_before(version)
    }

    pub fn get_in_progress_state_kv_snapshot_version(&self) -> Result<Option<Version>> {
        let mut iter = self
            .aptosdb
            .ledger_db
            .metadata_db()
            .iter::<DbMetadataSchema>(Default::default())?;
        iter.seek_to_first();
        while let Some((k, _v)) = iter.next().transpose()? {
            if let DbMetadataKey::StateSnapshotRestoreProgress(version) = k {
                return Ok(Some(version));
            }
        }
        Ok(None)
    }
}

// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    backup::restore_utils,
    ledger_db::LedgerDb,
    schema::db_metadata::{DbMetadataKey, DbMetadataSchema},
    state_restore::{StateSnapshotRestore, StateSnapshotRestoreMode},
    state_store::StateStore,
    AptosDB,
};
use aptos_crypto::HashValue;
use aptos_storage_interface::{DbReader, Result};
use aptos_types::{
    contract_event::ContractEvent,
    ledger_info::LedgerInfoWithSignatures,
    proof::definition::LeafCount,
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::{PersistedAuxiliaryInfo, Transaction, TransactionInfo, Version},
    write_set::WriteSet,
};
use std::sync::Arc;

/// Provides functionalities for AptosDB data restore.
#[derive(Clone)]
pub struct RestoreHandler {
    pub aptosdb: Arc<AptosDB>,
    state_store: Arc<StateStore>,
    ledger_db: Arc<LedgerDb>,
}

impl RestoreHandler {
    pub(crate) fn new(aptosdb: Arc<AptosDB>, state_store: Arc<StateStore>) -> Self {
        Self {
            ledger_db: Arc::clone(&aptosdb.ledger_db),
            aptosdb,
            state_store,
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
        restore_utils::save_ledger_infos(self.aptosdb.ledger_db.metadata_db(), ledger_infos, None)
    }

    pub fn confirm_or_save_frozen_subtrees(
        &self,
        num_leaves: LeafCount,
        frozen_subtrees: &[HashValue],
    ) -> Result<()> {
        restore_utils::confirm_or_save_frozen_subtrees(
            self.aptosdb.ledger_db.transaction_accumulator_db_raw(),
            num_leaves,
            frozen_subtrees,
            None,
        )
    }

    /// Transaction-backup restore replays only main state; it doesn't
    /// repopulate the native-position DBs, so a node restored with the feature
    /// enabled would carry an empty/stale position base and diverge once
    /// COMPUTE_TRADING_NATIVE_STATE_ROOTS is on. Refuse until native-position
    /// restore exists. No-op while ENABLE_TRADING_NATIVE is off.
    fn ensure_native_position_disabled(&self) -> Result<()> {
        if self.aptosdb.position().is_some() {
            return Err(aptos_storage_interface::AptosDbError::Other(
                "native-position transaction-backup restore is not yet supported; \
                 cannot restore with native position enabled"
                    .to_string(),
            ));
        }
        Ok(())
    }

    pub fn save_transactions(
        &self,
        first_version: Version,
        txns: &[Transaction],
        persisted_aux_info: &[PersistedAuxiliaryInfo],
        txn_infos: &[TransactionInfo],
        events: &[Vec<ContractEvent>],
        write_sets: Vec<WriteSet>,
    ) -> Result<()> {
        self.ensure_native_position_disabled()?;
        restore_utils::save_transactions(
            self.state_store.clone(),
            self.ledger_db.clone(),
            first_version,
            txns,
            persisted_aux_info,
            txn_infos,
            events,
            write_sets,
            None,
            false,
        )
    }

    pub fn force_state_version_for_kv_restore(&self, version: Option<Version>) -> Result<()> {
        self.state_store.init_state_ignoring_summary(version)
    }

    pub fn save_transactions_and_replay_kv(
        &self,
        first_version: Version,
        txns: &[Transaction],
        persisted_aux_info: &[PersistedAuxiliaryInfo],
        txn_infos: &[TransactionInfo],
        events: &[Vec<ContractEvent>],
        write_sets: Vec<WriteSet>,
    ) -> Result<()> {
        self.ensure_native_position_disabled()?;
        restore_utils::save_transactions(
            self.state_store.clone(),
            self.ledger_db.clone(),
            first_version,
            txns,
            persisted_aux_info,
            txn_infos,
            events,
            write_sets,
            None,
            true,
        )
    }

    pub fn get_next_expected_transaction_version(&self) -> Result<Version> {
        Ok(self.aptosdb.get_synced_version()?.map_or(0, |ver| ver + 1))
    }

    pub fn get_state_snapshot_before(
        &self,
        version: Version,
    ) -> Result<Option<(Version, HashValue)>> {
        self.aptosdb.get_state_snapshot_before(version)
    }

    pub fn get_in_progress_state_kv_snapshot_version(&self) -> Result<Option<Version>> {
        let db = self.aptosdb.state_kv_db.metadata_db_arc();
        let mut iter = db.iter::<DbMetadataSchema>()?;
        iter.seek_to_first();
        while let Some((k, _v)) = iter.next().transpose()? {
            if let DbMetadataKey::StateSnapshotKvRestoreProgress(version) = k {
                return Ok(Some(version));
            }
        }
        Ok(None)
    }
}

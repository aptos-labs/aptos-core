// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    backup::restore_utils,
    ledger_db::LedgerDb,
    native_store::state_sync::new_native_snapshot_restore,
    schema::db_metadata::{DbMetadataKey, DbMetadataSchema},
    state_restore::{StateSnapshotRestore, StateSnapshotRestoreMode},
    state_store::StateStore,
    AptosDB,
};
use aptos_crypto::HashValue;
use aptos_storage_interface::{AptosDbError, DbReader, Result};
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
            Arc::clone(&self.state_store.state_merkle_db),
            Arc::clone(&self.state_store),
            version,
            expected_root_hash,
            true, /* async_commit */
            restore_mode,
        )
    }

    pub fn reset_state_store(&self) -> Result<()> {
        self.state_store.reload()?;
        self.aptosdb.reset_position_store_to_ledger()
    }

    /// Whether this DB keeps native-position state, i.e. whether a restore
    /// must bring positions along with the state snapshot.
    pub fn has_position_store(&self) -> bool {
        self.aptosdb.position.is_some()
    }

    pub fn get_position_restore_receiver(
        &self,
        version: Version,
        expected_root_hash: HashValue,
    ) -> Result<StateSnapshotRestore<StateKey, StateValue>> {
        let position = self.aptosdb.position.as_ref().ok_or_else(|| {
            AptosDbError::Other("native-position storage is not enabled on this DB".into())
        })?;
        new_native_snapshot_restore(
            &position.kv_db,
            &position.merkle_db,
            version,
            expected_root_hash,
        )
    }

    /// Re-seat the position store on the snapshot just restored at `version`.
    pub fn reset_position_store(
        &self,
        version: Version,
        expected_root_hash: HashValue,
    ) -> Result<()> {
        self.aptosdb
            .reset_position_after_fast_sync(version, expected_root_hash)
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

    pub fn save_transactions(
        &self,
        first_version: Version,
        txns: &[Transaction],
        persisted_aux_info: &[PersistedAuxiliaryInfo],
        txn_infos: &[TransactionInfo],
        events: &[Vec<ContractEvent>],
        write_sets: Vec<WriteSet>,
    ) -> Result<()> {
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

    pub fn save_transactions_and_replay_kv(
        &self,
        first_version: Version,
        txns: &[Transaction],
        persisted_aux_info: &[PersistedAuxiliaryInfo],
        txn_infos: &[TransactionInfo],
        events: &[Vec<ContractEvent>],
        write_sets: Vec<WriteSet>,
    ) -> Result<()> {
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

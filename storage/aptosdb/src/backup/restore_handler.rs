// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    backup::restore_utils, event_store::EventStore, ledger_store::LedgerStore,
    state_store::StateStore, transaction_store::TransactionStore, AptosDB,
};
use anyhow::Result;
use aptos_crypto::{hash::SPARSE_MERKLE_PLACEHOLDER_HASH, HashValue};
use aptos_jellyfish_merkle::restore::StateSnapshotRestore;
use aptos_types::{
    contract_event::ContractEvent,
    ledger_info::LedgerInfoWithSignatures,
    proof::{accumulator::InMemoryAccumulator, definition::LeafCount},
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::{Transaction, TransactionInfo, Version},
};
use schemadb::DB;
use std::sync::Arc;
use storage_interface::{in_memory_state::InMemoryState, DbReader, ExecutedTrees};

/// Provides functionalities for AptosDB data restore.
#[derive(Clone)]
pub struct RestoreHandler {
    ledger_db: Arc<DB>,
    pub aptosdb: Arc<AptosDB>,
    ledger_store: Arc<LedgerStore>,
    transaction_store: Arc<TransactionStore>,
    state_store: Arc<StateStore>,
    event_store: Arc<EventStore>,
}

impl RestoreHandler {
    pub(crate) fn new(
        ledger_db: Arc<DB>,
        aptosdb: Arc<AptosDB>,
        ledger_store: Arc<LedgerStore>,
        transaction_store: Arc<TransactionStore>,
        state_store: Arc<StateStore>,
        event_store: Arc<EventStore>,
    ) -> Self {
        Self {
            ledger_db,
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
    ) -> Result<StateSnapshotRestore<StateKey, StateValue>> {
        StateSnapshotRestore::new_overwrite(
            &self.state_store.state_merkle_db,
            &self.state_store,
            version,
            expected_root_hash,
        )
    }

    pub fn save_ledger_infos(&self, ledger_infos: &[LedgerInfoWithSignatures]) -> Result<()> {
        restore_utils::save_ledger_infos(
            self.ledger_db.clone(),
            self.ledger_store.clone(),
            ledger_infos,
        )
    }

    pub fn confirm_or_save_frozen_subtrees(
        &self,
        num_leaves: LeafCount,
        frozen_subtrees: &[HashValue],
    ) -> Result<()> {
        restore_utils::confirm_or_save_frozen_subtrees(
            self.ledger_db.clone(),
            num_leaves,
            frozen_subtrees,
        )
    }

    pub fn save_transactions(
        &self,
        first_version: Version,
        txns: &[Transaction],
        txn_infos: &[TransactionInfo],
        events: &[Vec<ContractEvent>],
    ) -> Result<()> {
        restore_utils::save_transactions(
            self.ledger_db.clone(),
            self.ledger_store.clone(),
            self.transaction_store.clone(),
            self.event_store.clone(),
            first_version,
            txns,
            txn_infos,
            events,
        )
    }

    pub fn get_executed_trees(&self, version: Option<Version>) -> Result<ExecutedTrees> {
        let num_transactions: LeafCount = version.map_or(0, |v| v + 1);
        let frozen_subtrees = self
            .ledger_store
            .get_frozen_subtree_hashes(num_transactions)?;
        // For now, we know there must be a state snapshot at `version`. We need to recover checkpoint after make commit async.
        let (committed_version, committed_root_hash) = if let Some((version, hash)) = self
            .state_store
            .get_state_snapshot_before(num_transactions)?
        {
            (Some(version), hash)
        } else {
            (None, *SPARSE_MERKLE_PLACEHOLDER_HASH)
        };

        let transaction_accumulator =
            Arc::new(InMemoryAccumulator::new(frozen_subtrees, num_transactions)?);
        Ok(ExecutedTrees::new(
            InMemoryState::new_at_checkpoint(committed_root_hash, committed_version),
            transaction_accumulator,
        ))
    }

    pub fn get_next_expected_transaction_version(&self) -> Result<Version> {
        Ok(self
            .aptosdb
            .get_latest_transaction_info_option()?
            .map_or(0, |(ver, _txn_info)| ver + 1))
    }
}

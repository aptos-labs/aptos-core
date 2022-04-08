// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    backup::restore_utils, event_store::EventStore, ledger_store::LedgerStore,
    state_store::StateStore, transaction_store::TransactionStore, AptosDB,
};
use anyhow::Result;
use aptos_crypto::{hash::SPARSE_MERKLE_PLACEHOLDER_HASH, HashValue};
use aptos_jellyfish_merkle::restore::JellyfishMerkleRestore;
use aptos_types::{
    contract_event::ContractEvent,
    ledger_info::LedgerInfoWithSignatures,
    proof::definition::LeafCount,
    state_store::state_value::StateKeyAndValue,
    transaction::{Transaction, TransactionInfo, Version, PRE_GENESIS_VERSION},
};
use schemadb::DB;
use std::sync::Arc;
use storage_interface::{DbReader, TreeState};

/// Provides functionalities for AptosDB data restore.
#[derive(Clone)]
pub struct RestoreHandler {
    db: Arc<DB>,
    pub aptosdb: Arc<AptosDB>,
    ledger_store: Arc<LedgerStore>,
    transaction_store: Arc<TransactionStore>,
    state_store: Arc<StateStore>,
    event_store: Arc<EventStore>,
}

impl RestoreHandler {
    pub(crate) fn new(
        db: Arc<DB>,
        aptosdb: Arc<AptosDB>,
        ledger_store: Arc<LedgerStore>,
        transaction_store: Arc<TransactionStore>,
        state_store: Arc<StateStore>,
        event_store: Arc<EventStore>,
    ) -> Self {
        Self {
            db,
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
    ) -> Result<JellyfishMerkleRestore<StateKeyAndValue>> {
        JellyfishMerkleRestore::new_overwrite(
            Arc::clone(&self.state_store),
            version,
            expected_root_hash,
        )
    }

    pub fn save_ledger_infos(&self, ledger_infos: &[LedgerInfoWithSignatures]) -> Result<()> {
        restore_utils::save_ledger_infos(self.db.clone(), self.ledger_store.clone(), ledger_infos)
    }

    pub fn confirm_or_save_frozen_subtrees(
        &self,
        num_leaves: LeafCount,
        frozen_subtrees: &[HashValue],
    ) -> Result<()> {
        restore_utils::confirm_or_save_frozen_subtrees(self.db.clone(), num_leaves, frozen_subtrees)
    }

    pub fn save_transactions(
        &self,
        first_version: Version,
        txns: &[Transaction],
        txn_infos: &[TransactionInfo],
        events: &[Vec<ContractEvent>],
    ) -> Result<()> {
        restore_utils::save_transactions(
            self.db.clone(),
            self.ledger_store.clone(),
            self.transaction_store.clone(),
            self.event_store.clone(),
            first_version,
            txns,
            txn_infos,
            events,
        )
    }

    pub fn get_tree_state(&self, num_transactions: LeafCount) -> Result<TreeState> {
        let frozen_subtrees = self
            .ledger_store
            .get_frozen_subtree_hashes(num_transactions)?;
        let state_root_hash = if num_transactions == 0 {
            self.state_store
                .get_root_hash_option(PRE_GENESIS_VERSION)?
                .unwrap_or(*SPARSE_MERKLE_PLACEHOLDER_HASH)
        } else {
            self.state_store.get_root_hash(num_transactions - 1)?
        };

        Ok(TreeState::new(
            num_transactions,
            frozen_subtrees,
            state_root_hash,
        ))
    }

    pub fn get_next_expected_transaction_version(&self) -> Result<Version> {
        Ok(self
            .aptosdb
            .get_latest_transaction_info_option()?
            .map_or(0, |(ver, _txn_info)| ver + 1))
    }
}

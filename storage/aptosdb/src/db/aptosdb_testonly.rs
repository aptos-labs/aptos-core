// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::db::AptosDB;
#[cfg(test)]
use crate::state_merkle_db::StateMerkleDb;
use aptos_config::config::{
    RocksdbConfigs, StorageDirPaths, BUFFERED_STATE_TARGET_ITEMS_FOR_TEST,
    DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD, NO_OP_STORAGE_PRUNER_CONFIG,
};
use aptos_executor_types::transactions_with_output::TransactionsToKeep;
use aptos_storage_interface::{
    chunk_to_commit::ChunkToCommit, state_store::state_summary::ProvableStateSummary, DbReader,
    DbWriter, Result,
};
use aptos_types::{
    contract_event::ContractEvent,
    ledger_info::LedgerInfoWithSignatures,
    transaction::{
        PersistedAuxiliaryInfo, Transaction, TransactionInfo, TransactionOutput, TransactionStatus,
        TransactionToCommit, Version,
    },
};
use itertools::Itertools;
use std::path::Path;
#[cfg(test)]
use std::sync::Arc;

impl AptosDB {
    /// This opens db in non-readonly mode, without the pruner.
    pub fn new_for_test<P: AsRef<Path> + Clone>(db_root_path: P) -> Self {
        Self::new_without_pruner(
            db_root_path,
            false,
            BUFFERED_STATE_TARGET_ITEMS_FOR_TEST,
            DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
            false, /* indexer */
            false,
        )
    }

    /// This opens db with sharding enabled.
    pub fn new_for_test_with_sharding<P: AsRef<Path> + Clone>(
        db_root_path: P,
        max_node_cache: usize,
    ) -> Self {
        let db_config = RocksdbConfigs {
            enable_storage_sharding: true,
            ..Default::default()
        };
        Self::open(
            StorageDirPaths::from_path(db_root_path),
            false,
            NO_OP_STORAGE_PRUNER_CONFIG, /* pruner */
            db_config,
            false, /* indexer */
            BUFFERED_STATE_TARGET_ITEMS_FOR_TEST,
            max_node_cache,
            None,
        )
        .expect("Unable to open AptosDB")
    }

    /// This opens db in non-readonly mode, without the pruner and cache.
    pub fn new_for_test_no_cache<P: AsRef<Path> + Clone>(db_root_path: P) -> Self {
        Self::new_without_pruner(
            db_root_path,
            false,
            BUFFERED_STATE_TARGET_ITEMS_FOR_TEST,
            0,
            false,
            false,
        )
    }

    /// This opens db in non-readonly mode, without the pruner, and with the indexer
    pub fn new_for_test_with_indexer<P: AsRef<Path> + Clone>(
        db_root_path: P,
        enable_sharding: bool,
    ) -> Self {
        Self::new_without_pruner(
            db_root_path,
            false,
            BUFFERED_STATE_TARGET_ITEMS_FOR_TEST,
            DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
            true, /* indexer */
            enable_sharding,
        )
    }

    /// This opens db in non-readonly mode, without the pruner.
    pub fn new_for_test_with_buffered_state_target_items<P: AsRef<Path> + Clone>(
        db_root_path: P,
        buffered_state_target_items: usize,
    ) -> Self {
        Self::new_without_pruner(
            db_root_path,
            false,
            buffered_state_target_items,
            DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
            false, /* indexer */
            false,
        )
    }

    /// This opens db in non-readonly mode, without the pruner.
    pub fn new_readonly_for_test<P: AsRef<Path> + Clone>(db_root_path: P) -> Self {
        Self::new_without_pruner(
            db_root_path,
            true,
            BUFFERED_STATE_TARGET_ITEMS_FOR_TEST,
            DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
            false, /* indexer */
            false,
        )
    }

    #[cfg(test)]
    pub(crate) fn state_merkle_db(&self) -> Arc<StateMerkleDb> {
        self.state_store.state_db.state_merkle_db.clone()
    }
}

/// Test only methods for the DB
impl AptosDB {
    pub fn save_transactions_for_test(
        &self,
        txns_to_commit: &[TransactionToCommit],
        first_version: Version,
        ledger_info_with_sigs: Option<&LedgerInfoWithSignatures>,
        sync_commit: bool,
    ) -> Result<()> {
        let (transactions, transaction_outputs, transaction_infos) =
            Self::disassemble_txns_to_commit(txns_to_commit);
        let is_reconfig = transaction_outputs
            .iter()
            .rev()
            .flat_map(TransactionOutput::events)
            .any(ContractEvent::is_new_epoch_event);
        let auxiliary_info = transactions
            .iter()
            .map(|_| PersistedAuxiliaryInfo::None)
            .collect();
        let transactions_to_keep = TransactionsToKeep::make(
            first_version,
            transactions,
            transaction_outputs,
            auxiliary_info,
            is_reconfig,
        );

        let current = self.state_store.current_state_locked().clone();
        let (hot_state, persisted_state) = self.state_store.get_persisted_state()?;
        let (new_state, reads) = current.ledger_state().update_with_db_reader(
            &persisted_state,
            hot_state,
            transactions_to_keep.state_update_refs(),
            self.state_store.clone(),
        )?;
        let persisted_summary = self.state_store.get_persisted_state_summary()?;
        let new_state_summary = current.ledger_state_summary().update(
            &ProvableStateSummary::new(persisted_summary, self),
            transactions_to_keep.state_update_refs(),
        )?;

        let chunk = ChunkToCommit {
            first_version,
            transactions: &transactions_to_keep.transactions,
            persisted_info: &transactions_to_keep.persisted_info,
            transaction_outputs: &transactions_to_keep.transaction_outputs,
            transaction_infos: &transaction_infos,
            state: &new_state,
            state_summary: &new_state_summary,
            state_update_refs: transactions_to_keep.state_update_refs(),
            state_reads: &reads,
            is_reconfig,
        };

        self.save_transactions(chunk, ledger_info_with_sigs, sync_commit)
    }

    fn disassemble_txns_to_commit(
        txns_to_commit: &[TransactionToCommit],
    ) -> (
        Vec<Transaction>,
        Vec<TransactionOutput>,
        Vec<TransactionInfo>,
    ) {
        txns_to_commit
            .iter()
            .map(|txn_to_commit| {
                let TransactionToCommit {
                    transaction,
                    transaction_info,
                    write_set,
                    events,
                    is_reconfig: _,
                    transaction_auxiliary_data,
                } = txn_to_commit;

                let transaction_output = TransactionOutput::new(
                    write_set.clone(),
                    events.clone(),
                    transaction_info.gas_used(),
                    TransactionStatus::Keep(transaction_info.status().clone()),
                    transaction_auxiliary_data.clone(),
                );

                (
                    transaction.clone(),
                    transaction_output,
                    transaction_info.clone(),
                )
            })
            .multiunzip()
    }
}

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::buffered_state::BufferedState;
use aptos_config::config::{ BUFFERED_STATE_TARGET_ITEMS_FOR_TEST, DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD};
use aptos_infallible::Mutex;
use aptos_types::state_store::{create_empty_sharded_state_updates, ShardedStateUpdates};
use std::default::Default;
use aptos_storage_interface::cached_state_view::ShardedStateCache;
use aptos_storage_interface::state_delta::StateDelta;
use aptos_types::transaction::{TransactionStatus, TransactionToCommit};

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
        Self::new_without_pruner(db_root_path, false,
                                 BUFFERED_STATE_TARGET_ITEMS_FOR_TEST,
                                 0, false, false)
    }

    /// This opens db in non-readonly mode, without the pruner, and with the indexer
    pub fn new_for_test_with_indexer<P: AsRef<Path> + Clone>(db_root_path: P, enable_sharding: bool) -> Self {
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

    /// This gets the current buffered_state in StateStore.
    pub fn buffered_state(&self) -> &Mutex<BufferedState> {
        self.state_store.buffered_state()
    }

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
        base_state_version: Option<Version>,
        ledger_info_with_sigs: Option<&LedgerInfoWithSignatures>,
        sync_commit: bool,
        latest_in_memory_state: &StateDelta,
    ) -> Result<()> {
        let chunk = ChunkToCommitOwned::from_test_txns_to_commit(
            txns_to_commit,
            first_version,
            base_state_version,
            latest_in_memory_state,
        );
        self.save_transactions(
            chunk.as_ref(),
            ledger_info_with_sigs,
            sync_commit,
        )
    }
}

pub struct ChunkToCommitOwned {
    first_version: Version,
    transactions: Vec<Transaction>,
    transaction_outputs: Vec<TransactionOutput>,
    transaction_infos: Vec<TransactionInfo>,
    base_state_version: Option<Version>,
    latest_in_memory_state: Arc<StateDelta>,
    per_version_state_updates: Vec<ShardedStateUpdates>,
    state_updates_until_last_checkpoint: Option<ShardedStateUpdates>,
    sharded_state_cache: Option<ShardedStateCache>,
    is_reconfig: bool,
}

impl ChunkToCommitOwned {
    pub fn from_test_txns_to_commit(
        txns_to_commit: &[TransactionToCommit],
        first_version: Version,
        base_state_version: Option<Version>,
        latest_in_memory_state: &StateDelta,
    ) -> Self {
        let (transactions, transaction_outputs, transaction_infos, per_version_state_updates) = Self::disassemble_txns_to_commit(txns_to_commit);

        let state_updates_until_last_checkpoint = Self::gather_state_updates_until_last_checkpoint(
            first_version,
            latest_in_memory_state,
            &per_version_state_updates,
            &transaction_infos,
        );

        let is_reconfig = transaction_outputs
            .iter()
            .rev()
            .flat_map(TransactionOutput::events)
            .any(ContractEvent::is_new_epoch_event);

        Self {
            first_version,
            transactions,
            transaction_outputs,
            transaction_infos,
            base_state_version,
            latest_in_memory_state: Arc::new(latest_in_memory_state.clone()),
            per_version_state_updates,
            state_updates_until_last_checkpoint,
            sharded_state_cache: None,
            is_reconfig,
        }
    }

    pub fn as_ref(&self) -> ChunkToCommit {
        ChunkToCommit {
            first_version: self.first_version,
            transactions: &self.transactions,
            transaction_outputs: &self.transaction_outputs,
            transaction_infos: &self.transaction_infos,
            base_state_version: self.base_state_version,
            latest_in_memory_state: &self.latest_in_memory_state,
            per_version_state_updates: &self.per_version_state_updates,
            state_updates_until_last_checkpoint: self.state_updates_until_last_checkpoint.as_ref(),
            sharded_state_cache: self.sharded_state_cache.as_ref(),
            is_reconfig: self.is_reconfig,
        }
    }

    fn disassemble_txns_to_commit(txns_to_commit: &[TransactionToCommit]) -> (
        Vec<Transaction>, Vec<TransactionOutput>, Vec<TransactionInfo>, Vec<ShardedStateUpdates>,
    ) {
        txns_to_commit.iter().map(|txn_to_commit| {
            let TransactionToCommit {
                transaction, transaction_info, state_updates, write_set, events, is_reconfig: _, transaction_auxiliary_data
            } = txn_to_commit;

            let transaction_output = TransactionOutput::new(
                write_set.clone(),
                events.clone(),
                transaction_info.gas_used(),
                TransactionStatus::Keep(transaction_info.status().clone()),
                transaction_auxiliary_data.clone(),
            );

            (transaction.clone(), transaction_output, transaction_info.clone(), state_updates.clone())
        }).multiunzip()
    }

    pub fn gather_state_updates_until_last_checkpoint(
        first_version: Version,
        latest_in_memory_state: &StateDelta,
        per_version_state_updates: &[ShardedStateUpdates],
        transaction_infos: &[TransactionInfo],
    ) -> Option<ShardedStateUpdates> {
        if let Some(latest_checkpoint_version) = latest_in_memory_state.base_version {
            if latest_checkpoint_version >= first_version {
                let idx = (latest_checkpoint_version - first_version) as usize;
                assert!(
                    transaction_infos[idx].state_checkpoint_hash().is_some(),
                    "The new latest snapshot version passed in {:?} does not match with the last checkpoint version in txns_to_commit {:?}",
                    latest_checkpoint_version,
                    first_version + idx as u64
                );
                let mut sharded_state_updates = create_empty_sharded_state_updates();
                sharded_state_updates.par_iter_mut().enumerate().for_each(
                    |(shard_id, state_updates_shard)| {
                        per_version_state_updates[..=idx].iter().for_each(|updates| {
                            state_updates_shard.extend(updates[shard_id].clone());
                        })
                    },
                );
                return Some(sharded_state_updates);
            }
        }

        None
    }

}

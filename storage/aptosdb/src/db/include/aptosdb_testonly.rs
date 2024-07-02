// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::buffered_state::BufferedState;
use aptos_config::config::{
    BUFFERED_STATE_TARGET_ITEMS, DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
};
use aptos_infallible::Mutex;
use aptos_types::state_store::create_empty_sharded_state_updates;
use std::default::Default;

impl AptosDB {
    /// This opens db in non-readonly mode, without the pruner.
    pub fn new_for_test<P: AsRef<Path> + Clone>(db_root_path: P) -> Self {
        Self::new_without_pruner(
            db_root_path,
            false,
            BUFFERED_STATE_TARGET_ITEMS,
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
            BUFFERED_STATE_TARGET_ITEMS,
            max_node_cache,
        )
        .expect("Unable to open AptosDB")
    }

    /// This opens db in non-readonly mode, without the pruner and cache.
    pub fn new_for_test_no_cache<P: AsRef<Path> + Clone>(db_root_path: P) -> Self {
        Self::new_without_pruner(db_root_path, false, BUFFERED_STATE_TARGET_ITEMS, 0, false, false)
    }

    /// This opens db in non-readonly mode, without the pruner, and with the indexer
    pub fn new_for_test_with_indexer<P: AsRef<Path> + Clone>(db_root_path: P, enable_sharding: bool) -> Self {
        Self::new_without_pruner(
            db_root_path,
            false,
            BUFFERED_STATE_TARGET_ITEMS,
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
            BUFFERED_STATE_TARGET_ITEMS,
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

pub fn gather_state_updates_until_last_checkpoint(
    first_version: Version,
    latest_in_memory_state: &StateDelta,
    txns_to_commit: &[TransactionToCommit],
) -> Option<ShardedStateUpdates> {
    if let Some(latest_checkpoint_version) = latest_in_memory_state.base_version {
        if latest_checkpoint_version >= first_version {
            let idx = (latest_checkpoint_version - first_version) as usize;
            assert!(
                    txns_to_commit[idx].has_state_checkpoint_hash(),
                    "The new latest snapshot version passed in {:?} does not match with the last checkpoint version in txns_to_commit {:?}",
                    latest_checkpoint_version,
                    first_version + idx as u64
                );
            let mut sharded_state_updates = create_empty_sharded_state_updates();
            sharded_state_updates.par_iter_mut().enumerate().for_each(
                |(shard_id, state_updates_shard)| {
                    txns_to_commit[..=idx].iter().for_each(|txn_to_commit| {
                        state_updates_shard.extend(txn_to_commit.state_updates()[shard_id].clone());
                    })
                },
            );
            return Some(sharded_state_updates);
        }
    }

    None
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
        latest_in_memory_state: StateDelta,
    ) -> Result<()> {
        let state_updates_until_last_checkpoint = gather_state_updates_until_last_checkpoint(
            first_version,
            &latest_in_memory_state,
            txns_to_commit,
        );
        self.save_transactions(
            txns_to_commit,
            first_version,
            base_state_version,
            ledger_info_with_sigs,
            sync_commit,
            latest_in_memory_state,
            state_updates_until_last_checkpoint,
            None,
        )
    }
}

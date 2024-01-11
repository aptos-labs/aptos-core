// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::buffered_state::BufferedState;
use aptos_config::config::{
    BUFFERED_STATE_TARGET_ITEMS, DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
};
use aptos_infallible::Mutex;
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
            false, /* indexer async v2 */
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
            false, /* indexer async v2 */
        )
        .expect("Unable to open AptosDB")
    }

    /// This opens db in non-readonly mode, without the pruner and cache.
    pub fn new_for_test_no_cache<P: AsRef<Path> + Clone>(db_root_path: P) -> Self {
        Self::new_without_pruner(db_root_path, false, BUFFERED_STATE_TARGET_ITEMS, 0, false, false)
    }

    /// This opens db in non-readonly mode, without the pruner, and with the indexer
    pub fn new_for_test_with_indexer<P: AsRef<Path> + Clone>(db_root_path: P) -> Self {
        Self::new_without_pruner(
            db_root_path,
            false,
            BUFFERED_STATE_TARGET_ITEMS,
            DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
            true, /* indexer */
            true, /* indexer async v2 */
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
            false, /* indexer async v2 */
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
            false, /* indexer async v2 */
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

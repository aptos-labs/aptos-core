// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::lru_node_cache::LruNodeCache;
use crate::versioned_node_cache::VersionedNodeCache;
use schemadb::DB;
use std::sync::Arc;

#[derive(Debug)]
pub struct StateXerkleDb {
    pub(crate) db: Arc<DB>,
    enable_cache: bool,
    version_cache: VersionedNodeCache,
    lru_cache: LruNodeCache,
}

impl StateXerkleDb {
    pub fn new(state_xerkle_rocksdb: Arc<DB>, max_nodes_per_lru_cache_shard: usize) -> Self {
        Self {
            db: state_xerkle_rocksdb,
            enable_cache: max_nodes_per_lru_cache_shard > 0,
            version_cache: VersionedNodeCache::new(),
            lru_cache: LruNodeCache::new(max_nodes_per_lru_cache_shard),
        }
    }
}

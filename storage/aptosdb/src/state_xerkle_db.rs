// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::lru_node_cache::LruNodeCache;
use crate::versioned_node_cache::VersionedNodeCache;
use anyhow::Result;
use aptos_crypto::HashValue;
use aptos_types::state_store::state_key::StateKey;
use aptos_types::transaction::Version;
use aptos_types::xibble::XibblePath;
use schemadb::{SchemaBatch, DB};
use std::collections::HashMap;
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

    pub fn merklize_value_set(
        &self,
        value_set: Vec<(HashValue, Option<&(HashValue, StateKey)>)>,
        node_hashes: Option<&HashMap<XibblePath, HashValue>>,
        version: Version,
        base_version: Option<Version>,
        previous_epoch_ending_version: Option<Version>,
    ) -> Result<(SchemaBatch, HashValue)> {
        todo!()
        // let (new_root_hash, tree_update_batch) = {
        //     let _timer = OTHER_TIMERS_SECONDS
        //         .with_label_values(&["jmt_update"])
        //         .start_timer();
        //
        //     self.batch_put_value_set(value_set, node_hashes, base_version, version)
        // }?;
        //
        // if self.cache_enabled() {
        //     self.version_cache.add_version(
        //         version,
        //         tree_update_batch
        //             .node_batch
        //             .iter()
        //             .flatten()
        //             .cloned()
        //             .collect(),
        //     );
        // }
        //
        // let batch = SchemaBatch::new();
        // {
        //     let _timer = OTHER_TIMERS_SECONDS
        //         .with_label_values(&["serialize_jmt_commit"])
        //         .start_timer();
        //
        //     tree_update_batch
        //         .node_batch
        //         .iter()
        //         .flatten()
        //         .collect::<Vec<_>>()
        //         .par_iter()
        //         .with_min_len(128)
        //         .map(|(node_key, node)| batch.put::<JellyfishMerkleNodeSchema>(node_key, node))
        //         .collect::<Result<Vec<_>>>()?;
        //
        //     tree_update_batch
        //         .stale_node_index_batch
        //         .iter()
        //         .flatten()
        //         .collect::<Vec<_>>()
        //         .par_iter()
        //         .with_min_len(128)
        //         .map(|row| {
        //             if previous_epoch_ending_version.is_some()
        //                 && row.node_key.version() <= previous_epoch_ending_version.unwrap()
        //             {
        //                 // These are processed by the epoch snapshot pruner.
        //                 batch.put::<StaleNodeIndexCrossEpochSchema>(row, &())
        //             } else {
        //                 // These are processed by the state merkle pruner.
        //                 batch.put::<StaleNodeIndexSchema>(row, &())
        //             }
        //         })
        //         .collect::<Result<Vec<()>>>()?;
        // }
        //
        // Ok((batch, new_root_hash))
    }
}

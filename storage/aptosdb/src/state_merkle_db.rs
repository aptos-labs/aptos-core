// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    db_options::{gen_state_merkle_cfds, state_merkle_db_column_families},
    lru_node_cache::LruNodeCache,
    metrics::NODE_CACHE_SECONDS,
    schema::jellyfish_merkle_node::JellyfishMerkleNodeSchema,
    stale_node_index::StaleNodeIndexSchema,
    stale_node_index_cross_epoch::StaleNodeIndexCrossEpochSchema,
    versioned_node_cache::VersionedNodeCache,
    NUM_STATE_SHARDS, OTHER_TIMERS_SECONDS,
};
use anyhow::Result;
use aptos_config::config::{RocksdbConfig, RocksdbConfigs};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_jellyfish_merkle::{
    node_type::{NodeKey, NodeType},
    JellyfishMerkleTree, TreeReader, TreeUpdateBatch, TreeWriter,
};
use aptos_logger::prelude::*;
use aptos_rocksdb_options::gen_rocksdb_options;
use aptos_schemadb::{SchemaBatch, DB};
use aptos_types::{
    nibble::{nibble_path::NibblePath, ROOT_NIBBLE_HEIGHT},
    proof::{SparseMerkleProofExt, SparseMerkleRangeProof},
    state_store::state_key::StateKey,
    transaction::Version,
};
use arr_macro::arr;
use rayon::prelude::*;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};

pub const STATE_MERKLE_DB_FOLDER_NAME: &str = "state_merkle_db";
pub const STATE_MERKLE_DB_NAME: &str = "state_merkle_db";
pub const STATE_MERKLE_METADATA_DB_NAME: &str = "state_merkle_metadata_db";

pub(crate) type LeafNode = aptos_jellyfish_merkle::node_type::LeafNode<StateKey>;
pub(crate) type Node = aptos_jellyfish_merkle::node_type::Node<StateKey>;
type NodeBatch = aptos_jellyfish_merkle::NodeBatch<StateKey>;

#[derive(Debug)]
pub struct StateMerkleDb {
    // Stores metadata and top levels (non-sharded part) of tree nodes.
    state_merkle_metadata_db: Arc<DB>,
    // Stores sharded part of tree nodes.
    _state_merkle_db_shards: [Arc<DB>; NUM_STATE_SHARDS],
    enable_cache: bool,
    version_cache: VersionedNodeCache,
    lru_cache: LruNodeCache,
}

impl StateMerkleDb {
    pub(crate) fn new<P: AsRef<Path>>(
        db_root_path: P,
        rocksdb_configs: RocksdbConfigs,
        readonly: bool,
        max_nodes_per_lru_cache_shard: usize,
    ) -> Result<Self> {
        let state_merkle_db_config = rocksdb_configs.state_merkle_db_config;
        // TODO(grao): Currently when this value is set to 0 we disable both caches. This is
        // hacky, need to revisit.
        //
        // TODO(grao): Support cache sharding.
        let enable_cache = max_nodes_per_lru_cache_shard > 0;
        let version_cache = VersionedNodeCache::new();
        let lru_cache = LruNodeCache::new(max_nodes_per_lru_cache_shard);
        if !rocksdb_configs.use_sharded_state_merkle_db {
            info!("Sharded state merkle DB is not enabled!");
            let state_merkle_db_path = db_root_path.as_ref().join(STATE_MERKLE_DB_NAME);
            let db = Arc::new(Self::open_db(
                state_merkle_db_path,
                STATE_MERKLE_DB_NAME,
                &state_merkle_db_config,
                readonly,
            )?);
            return Ok(Self {
                state_merkle_metadata_db: Arc::clone(&db),
                _state_merkle_db_shards: arr![Arc::clone(&db); 16],
                enable_cache,
                version_cache,
                lru_cache,
            });
        }

        let state_merkle_metadata_db_path = db_root_path
            .as_ref()
            .join(STATE_MERKLE_DB_FOLDER_NAME)
            .join("metadata");

        let state_merkle_metadata_db = Arc::new(Self::open_db(
            state_merkle_metadata_db_path.clone(),
            STATE_MERKLE_METADATA_DB_NAME,
            &state_merkle_db_config,
            readonly,
        )?);

        info!(
            state_merkle_metadata_db_path = state_merkle_metadata_db_path,
            "Opened state merkle metadata db!"
        );

        let mut shard_id: usize = 0;
        let state_merkle_db_shards = arr![{
            let db = Self::open_shard(db_root_path.as_ref(), shard_id as u8, &state_merkle_db_config, readonly)?;
            shard_id += 1;
            Arc::new(db)
        }; 16];

        let state_merkle_db = Self {
            state_merkle_metadata_db,
            _state_merkle_db_shards: state_merkle_db_shards,
            enable_cache,
            version_cache,
            lru_cache,
        };

        // TODO(grao): Truncate shards to make them consistent.
        Ok(state_merkle_db)
    }

    pub(crate) fn metadata_db(&self) -> &DB {
        &self.state_merkle_metadata_db
    }

    pub fn get_with_proof_ext(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> Result<(
        Option<(HashValue, (StateKey, Version))>,
        SparseMerkleProofExt,
    )> {
        JellyfishMerkleTree::new(self).get_with_proof_ext(state_key.hash(), version)
    }

    pub fn get_range_proof(
        &self,
        rightmost_key: HashValue,
        version: Version,
    ) -> Result<SparseMerkleRangeProof> {
        JellyfishMerkleTree::new(self).get_range_proof(rightmost_key, version)
    }

    pub fn get_root_hash(&self, version: Version) -> Result<HashValue> {
        JellyfishMerkleTree::new(self).get_root_hash(version)
    }

    pub fn get_leaf_count(&self, version: Version) -> Result<usize> {
        JellyfishMerkleTree::new(self).get_leaf_count(version)
    }

    pub fn batch_put_value_set(
        &self,
        value_set: Vec<(HashValue, Option<&(HashValue, StateKey)>)>,
        node_hashes: Option<&HashMap<NibblePath, HashValue>>,
        persisted_version: Option<Version>,
        version: Version,
    ) -> Result<(HashValue, TreeUpdateBatch<StateKey>)> {
        JellyfishMerkleTree::new(self).batch_put_value_set(
            value_set,
            node_hashes,
            persisted_version,
            version,
        )
    }

    pub fn get_state_snapshot_version_before(
        &self,
        next_version: Version,
    ) -> Result<Option<Version>> {
        if next_version > 0 {
            let max_possible_version = next_version - 1;
            let mut iter = self
                .metadata_db()
                .rev_iter::<JellyfishMerkleNodeSchema>(Default::default())?;
            iter.seek_for_prev(&NodeKey::new_empty_path(max_possible_version))?;
            if let Some((key, _node)) = iter.next().transpose()? {
                // TODO: If we break up a single update batch to multiple commits, we would need to
                // deal with a partial version, which hasn't got the root committed.
                return Ok(Some(key.version()));
            }
        }
        // No version before genesis.
        Ok(None)
    }

    /// Merklize the results generated by `value_state_sets` to `batch` and return the result root
    /// hashes for each write set.
    pub fn merklize_value_set(
        &self,
        value_set: Vec<(HashValue, Option<&(HashValue, StateKey)>)>,
        node_hashes: Option<&HashMap<NibblePath, HashValue>>,
        version: Version,
        base_version: Option<Version>,
        previous_epoch_ending_version: Option<Version>,
    ) -> Result<(SchemaBatch, HashValue)> {
        let (new_root_hash, tree_update_batch) = {
            let _timer = OTHER_TIMERS_SECONDS
                .with_label_values(&["jmt_update"])
                .start_timer();

            self.batch_put_value_set(value_set, node_hashes, base_version, version)
        }?;

        if self.cache_enabled() {
            self.version_cache.add_version(
                version,
                tree_update_batch
                    .node_batch
                    .iter()
                    .flatten()
                    .cloned()
                    .collect(),
            );
        }

        let batch = SchemaBatch::new();
        {
            let _timer = OTHER_TIMERS_SECONDS
                .with_label_values(&["serialize_jmt_commit"])
                .start_timer();

            tree_update_batch
                .node_batch
                .iter()
                .flatten()
                .collect::<Vec<_>>()
                .par_iter()
                .with_min_len(128)
                .map(|(node_key, node)| batch.put::<JellyfishMerkleNodeSchema>(node_key, node))
                .collect::<Result<Vec<_>>>()?;

            tree_update_batch
                .stale_node_index_batch
                .iter()
                .flatten()
                .collect::<Vec<_>>()
                .par_iter()
                .with_min_len(128)
                .map(|row| {
                    if previous_epoch_ending_version.is_some()
                        && row.node_key.version() <= previous_epoch_ending_version.unwrap()
                    {
                        // These are processed by the epoch snapshot pruner.
                        batch.put::<StaleNodeIndexCrossEpochSchema>(row, &())
                    } else {
                        // These are processed by the state merkle pruner.
                        batch.put::<StaleNodeIndexSchema>(row, &())
                    }
                })
                .collect::<Result<Vec<()>>>()?;
        }

        Ok((batch, new_root_hash))
    }

    pub(crate) fn cache_enabled(&self) -> bool {
        self.enable_cache
    }

    pub(crate) fn version_cache(&self) -> &VersionedNodeCache {
        &self.version_cache
    }

    pub(crate) fn lru_cache(&self) -> &LruNodeCache {
        &self.lru_cache
    }

    fn open_shard<P: AsRef<Path>>(
        db_root_path: P,
        shard_id: u8,
        state_merkle_db_config: &RocksdbConfig,
        readonly: bool,
    ) -> Result<DB> {
        let shard_name = format!("shard_{}", shard_id);
        let db_name = format!("state_merkle_db_shard_{}", shard_id);
        let path = db_root_path
            .as_ref()
            .join(STATE_MERKLE_DB_FOLDER_NAME)
            .join(Path::new(&shard_name));
        Self::open_db(path, &db_name, state_merkle_db_config, readonly)
    }

    fn open_db(
        path: PathBuf,
        name: &str,
        state_merkle_db_config: &RocksdbConfig,
        readonly: bool,
    ) -> Result<DB> {
        Ok(if readonly {
            DB::open_cf_readonly(
                &gen_rocksdb_options(state_merkle_db_config, true),
                path,
                name,
                state_merkle_db_column_families(),
            )?
        } else {
            DB::open_cf(
                &gen_rocksdb_options(state_merkle_db_config, false),
                path,
                name,
                gen_state_merkle_cfds(state_merkle_db_config),
            )?
        })
    }

    /// Finds the rightmost leaf by scanning the entire DB.
    #[cfg(test)]
    pub fn get_rightmost_leaf_naive(
        &self,
        version: Version,
    ) -> Result<Option<(NodeKey, LeafNode)>> {
        let mut ret = None;

        // TODO(grao): Support sharding here.
        let mut iter = self
            .metadata_db()
            .iter::<JellyfishMerkleNodeSchema>(Default::default())?;
        iter.seek(&(version, 0)).unwrap();

        while let Some((node_key, node)) = iter.next().transpose()? {
            if let Node::Leaf(leaf_node) = node {
                if node_key.version() != version {
                    break;
                }
                match ret {
                    None => ret = Some((node_key, leaf_node)),
                    Some(ref other) => {
                        if leaf_node.account_key() > other.1.account_key() {
                            ret = Some((node_key, leaf_node));
                        }
                    },
                }
            }
        }

        Ok(ret)
    }
}

impl TreeReader<StateKey> for StateMerkleDb {
    fn get_node_option(&self, node_key: &NodeKey, tag: &str) -> Result<Option<Node>> {
        // TODO(grao): Support sharding here.
        let start_time = Instant::now();
        if !self.cache_enabled() {
            let node_opt = self
                .metadata_db()
                .get::<JellyfishMerkleNodeSchema>(node_key)?;
            NODE_CACHE_SECONDS
                .with_label_values(&[tag, "cache_disabled"])
                .observe(start_time.elapsed().as_secs_f64());
            return Ok(node_opt);
        }
        let node_opt = if let Some(node_cache) = self.version_cache.get_version(node_key.version())
        {
            let node = node_cache.get(node_key).cloned();
            NODE_CACHE_SECONDS
                .with_label_values(&[tag, "versioned_cache_hit"])
                .observe(start_time.elapsed().as_secs_f64());
            node
        } else if let Some(node) = self.lru_cache.get(node_key) {
            NODE_CACHE_SECONDS
                .with_label_values(&[tag, "lru_cache_hit"])
                .observe(start_time.elapsed().as_secs_f64());
            Some(node)
        } else {
            let node_opt = self
                .metadata_db()
                .get::<JellyfishMerkleNodeSchema>(node_key)?;
            if let Some(node) = &node_opt {
                self.lru_cache.put(node_key.clone(), node.clone());
            }
            NODE_CACHE_SECONDS
                .with_label_values(&[tag, "cache_miss"])
                .observe(start_time.elapsed().as_secs_f64());
            node_opt
        };
        Ok(node_opt)
    }

    fn get_rightmost_leaf(&self, version: Version) -> Result<Option<(NodeKey, LeafNode)>> {
        // Since everything has the same version during restore, we seek to the first node and get
        // its version.
        //
        // TODO(grao): Support sharding here.
        let mut iter = self
            .metadata_db()
            .iter::<JellyfishMerkleNodeSchema>(Default::default())?;
        iter.seek(&(version, 0))?;
        match iter.next().transpose()? {
            Some((node_key, node)) => {
                if node.node_type() == NodeType::Null || node_key.version() != version {
                    return Ok(None);
                }
            },
            None => return Ok(None),
        };

        // The encoding of key and value in DB looks like:
        //
        // | <-------------- key --------------> | <- value -> |
        // | version | num_nibbles | nibble_path |    node     |
        //
        // Here version is fixed. For each num_nibbles, there could be a range of nibble paths
        // of the same length. If one of them is the rightmost leaf R, it must be at the end of this
        // range. Otherwise let's assume the R is in the middle of the range, so we
        // call the node at the end of this range X:
        //   1. If X is leaf, then X.account_key() > R.account_key(), because the nibble path is a
        //      prefix of the account key. So R is not the rightmost leaf.
        //   2. If X is internal node, then X must be on the right side of R, so all its children's
        //      account keys are larger than R.account_key(). So R is not the rightmost leaf.
        //
        // Given that num_nibbles ranges from 0 to ROOT_NIBBLE_HEIGHT, there are only
        // ROOT_NIBBLE_HEIGHT+1 ranges, so we can just find the node at the end of each range and
        // then pick the one with the largest account key.
        let mut ret = None;

        for num_nibbles in 1..=ROOT_NIBBLE_HEIGHT + 1 {
            // TODO(grao): Support sharding here.
            let mut iter = self
                .metadata_db()
                .iter::<JellyfishMerkleNodeSchema>(Default::default())?;
            // nibble_path is always non-empty except for the root, so if we use an empty nibble
            // path as the seek key, the iterator will end up pointing to the end of the previous
            // range.
            let seek_key = (version, num_nibbles as u8);
            iter.seek_for_prev(&seek_key)?;

            if let Some((node_key, node)) = iter.next().transpose()? {
                if node_key.version() != version {
                    continue;
                }
                if let Node::Leaf(leaf_node) = node {
                    match ret {
                        None => ret = Some((node_key, leaf_node)),
                        Some(ref other) => {
                            if leaf_node.account_key() > other.1.account_key() {
                                ret = Some((node_key, leaf_node));
                            }
                        },
                    }
                }
            }
        }

        Ok(ret)
    }
}

impl TreeWriter<StateKey> for StateMerkleDb {
    fn write_node_batch(&self, node_batch: &NodeBatch) -> Result<()> {
        // TODO(grao): Support sharding here.
        let _timer = OTHER_TIMERS_SECONDS
            .with_label_values(&["tree_writer_write_batch"])
            .start_timer();
        let batch = SchemaBatch::new();
        node_batch.iter().try_for_each(|(node_key, node)| {
            batch.put::<JellyfishMerkleNodeSchema>(node_key, node)
        })?;
        self.metadata_db().write_schemas(batch)
    }
}

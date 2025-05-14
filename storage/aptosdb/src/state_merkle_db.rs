// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    db_options::gen_state_merkle_cfds,
    lru_node_cache::LruNodeCache,
    metrics::{NODE_CACHE_SECONDS, OTHER_TIMERS_SECONDS},
    schema::{
        db_metadata::{DbMetadataKey, DbMetadataSchema, DbMetadataValue},
        jellyfish_merkle_node::JellyfishMerkleNodeSchema,
        stale_node_index::StaleNodeIndexSchema,
        stale_node_index_cross_epoch::StaleNodeIndexCrossEpochSchema,
    },
    utils::truncation_helper::{get_state_merkle_commit_progress, truncate_state_merkle_db_shards},
    versioned_node_cache::VersionedNodeCache,
};
use aptos_config::config::{RocksdbConfig, RocksdbConfigs, StorageDirPaths};
use aptos_crypto::HashValue;
use aptos_experimental_runtimes::thread_manager::THREAD_MANAGER;
use aptos_jellyfish_merkle::{
    node_type::NodeKey, JellyfishMerkleTree, TreeReader, TreeUpdateBatch, TreeWriter,
};
use aptos_logger::prelude::*;
use aptos_metrics_core::TimerHelper;
use aptos_rocksdb_options::gen_rocksdb_options;
use aptos_schemadb::{
    batch::{IntoRawBatch, RawBatch, SchemaBatch, WriteBatch},
    DB,
};
#[cfg(test)]
use aptos_scratchpad::get_state_shard_id;
use aptos_storage_interface::{
    db_ensure as ensure, state_store::NUM_STATE_SHARDS, AptosDbError, Result,
};
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
    state_merkle_db_shards: [Arc<DB>; NUM_STATE_SHARDS],
    enable_sharding: bool,
    enable_cache: bool,
    // shard_id -> cache.
    version_caches: HashMap<Option<u8>, VersionedNodeCache>,
    lru_cache: LruNodeCache,
}

impl StateMerkleDb {
    pub(crate) fn new(
        db_paths: &StorageDirPaths,
        rocksdb_configs: RocksdbConfigs,
        readonly: bool,
        max_nodes_per_lru_cache_shard: usize,
    ) -> Result<Self> {
        let sharding = rocksdb_configs.enable_storage_sharding;
        let state_merkle_db_config = rocksdb_configs.state_merkle_db_config;
        // TODO(grao): Currently when this value is set to 0 we disable both caches. This is
        // hacky, need to revisit.
        let enable_cache = max_nodes_per_lru_cache_shard > 0;
        let mut version_caches = HashMap::with_capacity(NUM_STATE_SHARDS + 1);
        version_caches.insert(None, VersionedNodeCache::new());
        for i in 0..NUM_STATE_SHARDS {
            version_caches.insert(Some(i as u8), VersionedNodeCache::new());
        }
        let lru_cache = LruNodeCache::new(max_nodes_per_lru_cache_shard);
        if !sharding {
            info!("Sharded state merkle DB is not enabled!");
            let state_merkle_db_path = db_paths.default_root_path().join(STATE_MERKLE_DB_NAME);
            let db = Arc::new(Self::open_db(
                state_merkle_db_path,
                STATE_MERKLE_DB_NAME,
                &state_merkle_db_config,
                readonly,
            )?);
            return Ok(Self {
                state_merkle_metadata_db: Arc::clone(&db),
                state_merkle_db_shards: arr![Arc::clone(&db); 16],
                enable_sharding: false,
                enable_cache,
                version_caches,
                lru_cache,
            });
        }

        Self::open(
            db_paths,
            state_merkle_db_config,
            readonly,
            enable_cache,
            version_caches,
            lru_cache,
        )
    }

    pub(crate) fn commit(
        &self,
        version: Version,
        top_levels_batch: impl IntoRawBatch,
        batches_for_shards: Vec<impl IntoRawBatch + Send>,
    ) -> Result<()> {
        ensure!(
            batches_for_shards.len() == NUM_STATE_SHARDS,
            "Shard count mismatch."
        );
        THREAD_MANAGER.get_io_pool().install(|| {
            batches_for_shards
                .into_par_iter()
                .enumerate()
                .for_each(|(shard_id, batch)| {
                    self.db_shard(shard_id as u8)
                        .write_schemas(batch)
                        .unwrap_or_else(|err| {
                            panic!("Failed to commit state merkle shard {shard_id}: {err}")
                        });
                })
        });

        self.commit_top_levels(version, top_levels_batch)
    }

    /// Only used by fast sync / restore.
    pub(crate) fn commit_no_progress(
        &self,
        top_level_batch: SchemaBatch,
        batches_for_shards: Vec<SchemaBatch>,
    ) -> Result<()> {
        ensure!(
            batches_for_shards.len() == NUM_STATE_SHARDS,
            "Shard count mismatch."
        );
        let mut batches = batches_for_shards.into_iter();
        for shard_id in 0..NUM_STATE_SHARDS {
            let state_merkle_batch = batches.next().unwrap();
            self.state_merkle_db_shards[shard_id].write_schemas(state_merkle_batch)?;
        }

        self.state_merkle_metadata_db.write_schemas(top_level_batch)
    }

    pub(crate) fn create_checkpoint(
        db_root_path: impl AsRef<Path>,
        cp_root_path: impl AsRef<Path>,
        sharding: bool,
    ) -> Result<()> {
        let rocksdb_configs = RocksdbConfigs {
            enable_storage_sharding: sharding,
            ..Default::default()
        };
        // TODO(grao): Support path override here.
        let state_merkle_db = Self::new(
            &StorageDirPaths::from_path(db_root_path),
            rocksdb_configs,
            /*readonly=*/ false,
            /*max_nodes_per_lru_cache_shard=*/ 0,
        )?;
        let cp_state_merkle_db_path = cp_root_path.as_ref().join(STATE_MERKLE_DB_FOLDER_NAME);

        info!("Creating state_merkle_db checkpoint at: {cp_state_merkle_db_path:?}");

        std::fs::remove_dir_all(&cp_state_merkle_db_path).unwrap_or(());
        if sharding {
            std::fs::create_dir_all(&cp_state_merkle_db_path).unwrap_or(());
        }

        state_merkle_db
            .metadata_db()
            .create_checkpoint(Self::metadata_db_path(cp_root_path.as_ref(), sharding))?;

        if sharding {
            for shard_id in 0..NUM_STATE_SHARDS {
                state_merkle_db
                    .db_shard(shard_id as u8)
                    .create_checkpoint(Self::db_shard_path(
                        cp_root_path.as_ref(),
                        shard_id as u8,
                    ))?;
            }
        }

        Ok(())
    }

    pub(crate) fn metadata_db(&self) -> &DB {
        &self.state_merkle_metadata_db
    }

    pub(crate) fn metadata_db_arc(&self) -> Arc<DB> {
        Arc::clone(&self.state_merkle_metadata_db)
    }

    pub(crate) fn db_shard(&self, shard_id: u8) -> &DB {
        &self.state_merkle_db_shards[shard_id as usize]
    }

    pub(crate) fn db_shard_arc(&self, shard_id: u8) -> Arc<DB> {
        Arc::clone(&self.state_merkle_db_shards[shard_id as usize])
    }

    pub(crate) fn db(&self, shard_id: Option<u8>) -> &DB {
        if let Some(shard_id) = shard_id {
            self.db_shard(shard_id)
        } else {
            self.metadata_db()
        }
    }

    pub(crate) fn commit_top_levels(
        &self,
        version: Version,
        batch: impl IntoRawBatch,
    ) -> Result<()> {
        info!(version = version, "Committing StateMerkleDb.");
        self.state_merkle_metadata_db.write_schemas(batch)
    }

    pub fn get_with_proof_ext(
        &self,
        key: &HashValue,
        version: Version,
        root_depth: usize,
    ) -> Result<(
        Option<(HashValue, (StateKey, Version))>,
        SparseMerkleProofExt,
    )> {
        JellyfishMerkleTree::new(self).get_with_proof_ext(key, version, root_depth)
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

    pub fn batch_put_value_set_for_shard(
        &self,
        shard_id: u8,
        value_set: Vec<(HashValue, Option<&(HashValue, StateKey)>)>,
        node_hashes: Option<&HashMap<NibblePath, HashValue>>,
        persisted_version: Option<Version>,
        version: Version,
    ) -> Result<(Node, TreeUpdateBatch<StateKey>)> {
        JellyfishMerkleTree::new(self).batch_put_value_set_for_shard(
            shard_id,
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
            let mut iter = self.metadata_db().rev_iter::<JellyfishMerkleNodeSchema>()?;
            iter.seek_for_prev(&NodeKey::new_empty_path(max_possible_version))?;
            if let Some((key, _node)) = iter.next().transpose()? {
                let version = key.version();
                if self
                    .metadata_db()
                    .get::<JellyfishMerkleNodeSchema>(&NodeKey::new_empty_path(version))?
                    .is_some()
                {
                    return Ok(Some(version));
                }
                // Since we split state merkle commit into multiple batches, it's possible that
                // the root is not committed yet. In this case we need to look at the previous
                // root.
                return self.get_state_snapshot_version_before(version);
            }
        }
        // No version before genesis.
        Ok(None)
    }

    fn create_jmt_commit_batch_for_shard(
        &self,
        version: Version,
        shard_id: Option<u8>,
        tree_update_batch: &TreeUpdateBatch<StateKey>,
        previous_epoch_ending_version: Option<Version>,
    ) -> Result<RawBatch> {
        let _timer = OTHER_TIMERS_SECONDS.timer_with(&["create_jmt_commit_batch_for_shard"]);

        let mut batch = self.db(shard_id).new_native_batch();

        let node_batch = tree_update_batch
            .node_batch
            .iter()
            .flatten()
            .collect::<Vec<_>>();
        node_batch.iter().try_for_each(|(node_key, node)| {
            ensure!(node_key.get_shard_id() == shard_id, "shard_id mismatch");
            batch.put::<JellyfishMerkleNodeSchema>(node_key, node)
        })?;

        let stale_node_index_batch = tree_update_batch
            .stale_node_index_batch
            .iter()
            .flatten()
            .collect::<Vec<_>>();
        stale_node_index_batch.iter().try_for_each(|row| {
            ensure!(row.node_key.get_shard_id() == shard_id, "shard_id mismatch");
            if previous_epoch_ending_version.is_some()
                && row.node_key.version() <= previous_epoch_ending_version.unwrap()
            {
                batch.put::<StaleNodeIndexCrossEpochSchema>(row, &())
            } else {
                // These are processed by the state merkle pruner.
                batch.put::<StaleNodeIndexSchema>(row, &())
            }
        })?;

        Self::put_progress(Some(version), shard_id, &mut batch)?;

        batch.into_raw_batch(self.db(shard_id))
    }

    pub(crate) fn put_progress(
        version: Option<Version>,
        shard_id: Option<u8>,
        batch: &mut impl WriteBatch,
    ) -> Result<()> {
        let key = if let Some(shard_id) = shard_id {
            DbMetadataKey::StateMerkleShardCommitProgress(shard_id as usize)
        } else {
            DbMetadataKey::StateMerkleCommitProgress
        };

        if let Some(version) = version {
            batch.put::<DbMetadataSchema>(&key, &DbMetadataValue::Version(version))
        } else {
            batch.delete::<DbMetadataSchema>(&key)
        }
    }

    // A non-sharded helper function accepting KV updates from all shards.
    #[cfg(test)]
    pub fn merklize_value_set(
        &self,
        value_set: Vec<(HashValue, Option<&(HashValue, StateKey)>)>,
        version: Version,
        base_version: Option<Version>,
        previous_epoch_ending_version: Option<Version>,
    ) -> Result<(RawBatch, Vec<RawBatch>, HashValue)> {
        let mut sharded_value_set: Vec<Vec<(HashValue, Option<&(HashValue, StateKey)>)>> =
            Vec::new();
        sharded_value_set.resize(NUM_STATE_SHARDS, Default::default());
        value_set.into_iter().for_each(|(k, v)| {
            sharded_value_set[get_state_shard_id(&k) as usize].push((k, v));
        });

        let (shard_root_nodes, sharded_batches) = (0..16)
            .map(|shard_id| {
                self.merklize_value_set_for_shard(
                    shard_id,
                    sharded_value_set[shard_id as usize].clone(),
                    /*node_hashes=*/ None,
                    version,
                    base_version,
                    base_version,
                    previous_epoch_ending_version,
                )
                .unwrap()
            })
            .collect::<Vec<_>>()
            .into_iter()
            .unzip();

        let (root_hash, _leaf_count, top_levels_batch) = self.calculate_top_levels(
            shard_root_nodes,
            version,
            base_version,
            previous_epoch_ending_version,
        )?;

        Ok((top_levels_batch, sharded_batches, root_hash))
    }

    /// Calculates db updates for nodes in shard `shard_id`.
    ///
    /// Assumes 16 shards in total for now.
    pub fn merklize_value_set_for_shard(
        &self,
        shard_id: u8,
        value_set: Vec<(HashValue, Option<&(HashValue, StateKey)>)>,
        node_hashes: Option<&HashMap<NibblePath, HashValue>>,
        version: Version,
        base_version: Option<Version>,
        shard_persisted_version: Option<Version>,
        previous_epoch_ending_version: Option<Version>,
    ) -> Result<(Node, RawBatch)> {
        if let Some(shard_persisted_version) = shard_persisted_version {
            assert!(shard_persisted_version <= base_version.expect("Must have base version."));
        }

        let (shard_root_node, tree_update_batch) = {
            let _timer = OTHER_TIMERS_SECONDS
                .with_label_values(&["jmt_update"])
                .start_timer();

            self.batch_put_value_set_for_shard(
                shard_id,
                value_set,
                node_hashes,
                shard_persisted_version,
                version,
            )
        }?;

        if self.cache_enabled() {
            self.version_caches
                .get(&Some(shard_id))
                .unwrap()
                .add_version(
                    version,
                    tree_update_batch
                        .node_batch
                        .iter()
                        .flatten()
                        .cloned()
                        .collect(),
                );
        }

        let batch = self.create_jmt_commit_batch_for_shard(
            version,
            Some(shard_id),
            &tree_update_batch,
            previous_epoch_ending_version,
        )?;

        Ok((shard_root_node, batch))
    }

    /// Calculates db updates for non-sharded nodes at top levels.
    ///
    /// Assumes 16 shards in total for now.
    pub fn calculate_top_levels(
        &self,
        shard_root_nodes: Vec<Node>,
        version: Version,
        base_version: Option<Version>,
        previous_epoch_ending_version: Option<Version>,
    ) -> Result<(HashValue, usize, RawBatch)> {
        assert!(shard_root_nodes.len() == 16);

        let (root_hash, leaf_count, tree_update_batch) = JellyfishMerkleTree::new(self)
            .put_top_levels_nodes(shard_root_nodes, base_version, version)?;

        if self.cache_enabled() {
            self.version_caches.get(&None).unwrap().add_version(
                version,
                tree_update_batch
                    .node_batch
                    .iter()
                    .flatten()
                    .cloned()
                    .collect(),
            );
        }

        let batch = self.create_jmt_commit_batch_for_shard(
            version,
            None,
            &tree_update_batch,
            previous_epoch_ending_version,
        )?;

        Ok((root_hash, leaf_count, batch.into_raw_batch(self.db(None))?))
    }

    pub(crate) fn get_shard_persisted_versions(
        &self,
        root_persisted_version: Option<Version>,
    ) -> Result<[Option<Version>; NUM_STATE_SHARDS]> {
        JellyfishMerkleTree::new(self).get_shard_persisted_versions(root_persisted_version)
    }

    pub(crate) fn sharding_enabled(&self) -> bool {
        self.enable_sharding
    }

    pub(crate) fn cache_enabled(&self) -> bool {
        self.enable_cache
    }

    pub(crate) fn version_caches(&self) -> &HashMap<Option<u8>, VersionedNodeCache> {
        &self.version_caches
    }

    pub(crate) fn lru_cache(&self) -> &LruNodeCache {
        &self.lru_cache
    }

    pub(crate) fn write_pruner_progress(&self, version: Version) -> Result<()> {
        self.state_merkle_metadata_db.put::<DbMetadataSchema>(
            &DbMetadataKey::StateMerklePrunerProgress,
            &DbMetadataValue::Version(version),
        )
    }

    pub(crate) fn num_shards(&self) -> u8 {
        NUM_STATE_SHARDS as u8
    }

    pub(crate) fn hack_num_real_shards(&self) -> usize {
        if self.enable_sharding {
            NUM_STATE_SHARDS
        } else {
            1
        }
    }

    fn db_by_key(&self, node_key: &NodeKey) -> &DB {
        if let Some(shard_id) = node_key.get_shard_id() {
            self.db_shard(shard_id)
        } else {
            self.metadata_db()
        }
    }

    fn open(
        db_paths: &StorageDirPaths,
        state_merkle_db_config: RocksdbConfig,
        readonly: bool,
        enable_cache: bool,
        version_caches: HashMap<Option<u8>, VersionedNodeCache>,
        lru_cache: LruNodeCache,
    ) -> Result<Self> {
        let state_merkle_metadata_db_path = Self::metadata_db_path(
            db_paths.state_merkle_db_metadata_root_path(),
            /*sharding=*/ true,
        );

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

        let state_merkle_db_shards = (0..NUM_STATE_SHARDS)
            .into_par_iter()
            .map(|shard_id| {
                let shard_root_path = db_paths.state_merkle_db_shard_root_path(shard_id as u8);
                let db = Self::open_shard(
                    shard_root_path,
                    shard_id as u8,
                    &state_merkle_db_config,
                    readonly,
                )
                .unwrap_or_else(|e| {
                    panic!("Failed to open state merkle db shard {shard_id}: {e:?}.")
                });
                Arc::new(db)
            })
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let state_merkle_db = Self {
            state_merkle_metadata_db,
            state_merkle_db_shards,
            enable_sharding: true,
            enable_cache,
            version_caches,
            lru_cache,
        };

        if !readonly {
            if let Some(overall_state_merkle_commit_progress) =
                get_state_merkle_commit_progress(&state_merkle_db)?
            {
                truncate_state_merkle_db_shards(
                    &state_merkle_db,
                    overall_state_merkle_commit_progress,
                )?;
            }
        }

        Ok(state_merkle_db)
    }

    fn open_shard<P: AsRef<Path>>(
        db_root_path: P,
        shard_id: u8,
        state_merkle_db_config: &RocksdbConfig,
        readonly: bool,
    ) -> Result<DB> {
        let db_name = format!("state_merkle_db_shard_{}", shard_id);
        Self::open_db(
            Self::db_shard_path(db_root_path, shard_id),
            &db_name,
            state_merkle_db_config,
            readonly,
        )
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
                gen_state_merkle_cfds(state_merkle_db_config),
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

    fn db_shard_path<P: AsRef<Path>>(db_root_path: P, shard_id: u8) -> PathBuf {
        let shard_sub_path = format!("shard_{}", shard_id);
        db_root_path
            .as_ref()
            .join(STATE_MERKLE_DB_FOLDER_NAME)
            .join(Path::new(&shard_sub_path))
    }

    fn metadata_db_path<P: AsRef<Path>>(db_root_path: P, sharding: bool) -> PathBuf {
        if sharding {
            db_root_path
                .as_ref()
                .join(STATE_MERKLE_DB_FOLDER_NAME)
                .join("metadata")
        } else {
            db_root_path.as_ref().join(STATE_MERKLE_DB_NAME)
        }
    }

    /// Finds the rightmost leaf by scanning the entire DB.
    #[cfg(test)]
    pub fn get_rightmost_leaf_naive(
        &self,
        version: Version,
    ) -> Result<Option<(NodeKey, LeafNode)>> {
        let mut ret = None;

        // traverse all shards in a naive way
        let shards = 0..self.hack_num_real_shards();
        let start_num_of_nibbles = if self.enable_sharding { 1 } else { 0 };
        for shard_id in shards.rev() {
            let shard_db = self.state_merkle_db_shards[shard_id].clone();
            let mut shard_iter = shard_db.iter::<JellyfishMerkleNodeSchema>()?;
            // DB sharded only contain nodes with num_of_nibbles >= 1
            shard_iter.seek(&(version, start_num_of_nibbles)).unwrap();

            while let Some((node_key, node)) = shard_iter.next().transpose()? {
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
        }

        Ok(ret)
    }

    fn get_rightmost_leaf_in_single_shard(
        &self,
        version: Version,
        shard_id: u8,
    ) -> Result<Option<(NodeKey, LeafNode)>> {
        assert!(
            shard_id < NUM_STATE_SHARDS as u8,
            "Invalid shard_id: {}",
            shard_id
        );
        let shard_db = self.state_merkle_db_shards[shard_id as usize].clone();
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

        for num_nibbles in 0..=ROOT_NIBBLE_HEIGHT {
            let mut iter = shard_db.iter::<JellyfishMerkleNodeSchema>()?;
            // nibble_path is always non-empty except for the root, so if we use an empty nibble
            // path as the seek key, the iterator will end up pointing to the end of the previous
            // range.
            let seek_key = (version, (num_nibbles + 1) as u8);
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

impl TreeReader<StateKey> for StateMerkleDb {
    fn get_node_option(&self, node_key: &NodeKey, tag: &str) -> Result<Option<Node>> {
        let start_time = Instant::now();
        if !self.cache_enabled() {
            let node_opt = self
                .db_by_key(node_key)
                .get::<JellyfishMerkleNodeSchema>(node_key)?;
            NODE_CACHE_SECONDS
                .with_label_values(&[tag, "cache_disabled"])
                .observe(start_time.elapsed().as_secs_f64());
            return Ok(node_opt);
        }
        let node_opt = if let Some(node_cache) = self
            .version_caches
            .get(&node_key.get_shard_id())
            .unwrap()
            .get_version(node_key.version())
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
                .db_by_key(node_key)
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
        let ret = None;
        let shards = 0..self.hack_num_real_shards();

        // Search from right to left to find the first leaf node.
        for shard_id in shards.rev() {
            if let Some((node_key, leaf_node)) =
                self.get_rightmost_leaf_in_single_shard(version, shard_id as u8)?
            {
                return Ok(Some((node_key, leaf_node)));
            }
        }

        Ok(ret)
    }
}

impl TreeWriter<StateKey> for StateMerkleDb {
    fn write_node_batch(&self, node_batch: &NodeBatch) -> Result<()> {
        let _timer = OTHER_TIMERS_SECONDS
            .with_label_values(&["tree_writer_write_batch"])
            .start_timer();
        // Get the top level batch and sharded batch from raw NodeBatch
        let mut top_level_batch = SchemaBatch::new();
        let mut jmt_shard_batches: Vec<SchemaBatch> = Vec::with_capacity(NUM_STATE_SHARDS);
        jmt_shard_batches.resize_with(NUM_STATE_SHARDS, SchemaBatch::new);
        node_batch.iter().try_for_each(|(node_key, node)| {
            if let Some(shard_id) = node_key.get_shard_id() {
                jmt_shard_batches[shard_id as usize]
                    .put::<JellyfishMerkleNodeSchema>(node_key, node)
            } else {
                top_level_batch.put::<JellyfishMerkleNodeSchema>(node_key, node)
            }
        })?;
        self.commit_no_progress(top_level_batch, jmt_shard_batches)
    }
}

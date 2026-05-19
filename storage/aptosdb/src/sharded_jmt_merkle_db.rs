// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Shared sharded JMT merkle DB substrate.
//!
//! Holds the 16-shard RocksDB layout + a separate metadata DB for the
//! top-level (non-sharded) JMT nodes, plus the two cache layers
//! (`VersionedNodeCache` per shard + per-top-level, optional `LruNodeCache`).
//!
//! All JMT-side operations live here:
//! `batch_put_value_set_for_shard`, `merklize_value_set_for_shard`,
//! `calculate_top_levels`, `commit`, `create_jmt_commit_batch_for_shard`,
//! plus `TreeReader<StateKey>` / `TreeWriter<StateKey>` impls.
//!
//! Domain-specific concerns — how the shard/metadata RocksDB
//! instances are opened (paths, CFDs, hot/cold split, truncation-on-startup),
//! checkpoint creation, and the metrics tag used for per-tree timer
//! labels — are left to outer wrappers that compose this substrate.

use crate::{
    common::populate_jmt_writes,
    lru_node_cache::LruNodeCache,
    metrics::{NODE_CACHE_SECONDS, OTHER_TIMERS_SECONDS},
    schema::{
        db_metadata::{DbMetadataKey, DbMetadataSchema, DbMetadataValue},
        jellyfish_merkle_node::JellyfishMerkleNodeSchema,
    },
    versioned_node_cache::VersionedNodeCache,
};
use aptos_crypto::HashValue;
use aptos_experimental_runtimes::thread_manager::THREAD_MANAGER;
use aptos_jellyfish_merkle::{
    node_type::NodeKey, JellyfishMerkleTree, TreeReader, TreeUpdateBatch, TreeWriter,
};
use aptos_logger::info;
use aptos_metrics_core::TimerHelper;
use aptos_schemadb::{
    batch::{IntoRawBatch, RawBatch, SchemaBatch, WriteBatch},
    DB,
};
#[cfg(test)]
use aptos_scratchpad::get_state_shard_id;
use aptos_storage_interface::{db_ensure as ensure, AptosDbError, Result};
use aptos_types::{
    nibble::{nibble_path::NibblePath, ROOT_NIBBLE_HEIGHT},
    proof::{SparseMerkleProofExt, SparseMerkleRangeProof},
    state_store::{state_key::StateKey, NUM_STATE_SHARDS},
    transaction::Version,
};
use rayon::prelude::*;
use std::{collections::HashMap, sync::Arc, time::Instant};

pub(crate) type LeafNode = aptos_jellyfish_merkle::node_type::LeafNode<StateKey>;
pub(crate) type Node = aptos_jellyfish_merkle::node_type::Node<StateKey>;
type NodeBatch = aptos_jellyfish_merkle::NodeBatch<StateKey>;

/// Sharded JMT merkle DB substrate. See module docs.
#[derive(Debug)]
pub struct ShardedJmtMerkleDb {
    /// Stores metadata and top levels (non-sharded part) of tree nodes.
    metadata_db: Arc<DB>,
    /// Stores sharded part of tree nodes.
    shards: [Arc<DB>; NUM_STATE_SHARDS],
    /// shard_id -> cache. `None` key is the top-levels cache.
    version_caches: HashMap<Option<usize>, VersionedNodeCache>,
    /// `None` means the LRU cache is disabled.
    lru_cache: Option<LruNodeCache>,
    /// Metrics tag for per-tree timer labels (e.g. `"hot"`, `"cold"`, `"position"`).
    db_tag: &'static str,
}

impl ShardedJmtMerkleDb {
    pub(crate) fn new(
        metadata_db: Arc<DB>,
        shards: [Arc<DB>; NUM_STATE_SHARDS],
        max_nodes_per_lru_cache_shard: usize,
        db_tag: &'static str,
    ) -> Self {
        let mut version_caches = HashMap::with_capacity(NUM_STATE_SHARDS + 1);
        version_caches.insert(None, VersionedNodeCache::new());
        for i in 0..NUM_STATE_SHARDS {
            version_caches.insert(Some(i), VersionedNodeCache::new());
        }
        let lru_cache =
            std::num::NonZeroUsize::new(max_nodes_per_lru_cache_shard).map(LruNodeCache::new);
        Self {
            metadata_db,
            shards,
            version_caches,
            lru_cache,
            db_tag,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn db_tag(&self) -> &'static str {
        self.db_tag
    }

    pub(crate) fn metadata_db(&self) -> &DB {
        &self.metadata_db
    }

    pub(crate) fn metadata_db_arc(&self) -> Arc<DB> {
        Arc::clone(&self.metadata_db)
    }

    pub(crate) fn db_shard(&self, shard_id: usize) -> &DB {
        &self.shards[shard_id]
    }

    pub(crate) fn db_shard_arc(&self, shard_id: usize) -> Arc<DB> {
        Arc::clone(&self.shards[shard_id])
    }

    pub(crate) fn db(&self, shard_id: Option<usize>) -> &DB {
        if let Some(shard_id) = shard_id {
            self.db_shard(shard_id)
        } else {
            self.metadata_db()
        }
    }

    pub(crate) fn num_shards(&self) -> usize {
        NUM_STATE_SHARDS
    }

    pub(crate) fn cache_enabled(&self) -> bool {
        self.lru_cache.is_some()
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
                    self.db_shard(shard_id)
                        .write_schemas(batch)
                        .unwrap_or_else(|err| {
                            panic!("Failed to commit state merkle shard {shard_id}: {err}")
                        });
                })
        });

        self.commit_top_levels(version, top_levels_batch)?;

        // Evict obsolete cached versions if caching is enabled. Same
        // logic that used to live in `StateMerkleBatchCommitter`; now
        // every consumer of `ShardedJmtMerkleDb::commit` automatically
        // gets it (position once caching is wired on the position
        // wrapper).
        if let Some(lru_cache) = self.lru_cache.as_ref() {
            self.version_caches
                .iter()
                .for_each(|(_, cache)| cache.maybe_evict_version(lru_cache));
        }

        Ok(())
    }

    /// Commits JMT node data without writing any commit progress metadata.
    /// Used by `TreeWriter::write_node_batch` during fast-sync / state
    /// snapshot restore. Sequential write so that on crash all
    /// fully-committed shards form a prefix.
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
            let batch = batches.next().unwrap();
            self.shards[shard_id].write_schemas(batch)?;
        }
        self.metadata_db.write_schemas(top_level_batch)
    }

    pub(crate) fn commit_top_levels(
        &self,
        version: Version,
        batch: impl IntoRawBatch,
    ) -> Result<()> {
        info!(
            version = version,
            db_tag = self.db_tag,
            "Committing merkle metadata DB."
        );
        self.metadata_db.write_schemas(batch)
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
        shard_id: usize,
        value_set: Vec<(HashValue, Option<&(HashValue, StateKey)>)>,
        node_hashes: Option<&HashMap<NibblePath, HashValue>>,
        persisted_version: Option<Version>,
        version: Version,
    ) -> Result<(Node, TreeUpdateBatch<StateKey>)> {
        JellyfishMerkleTree::new(self).batch_put_value_set_for_shard(
            shard_id as u8,
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

    pub(crate) fn create_jmt_commit_batch_for_shard(
        &self,
        version: Version,
        shard_id: Option<usize>,
        tree_update_batch: &TreeUpdateBatch<StateKey>,
        previous_epoch_ending_version: Option<Version>,
    ) -> Result<RawBatch> {
        let _timer = OTHER_TIMERS_SECONDS.timer_with(&[&format!(
            "{}__create_jmt_commit_batch_for_shard",
            self.db_tag
        )]);

        // Paranoia: every node/stale-index this shard's caller produced
        // must already be tagged for this shard. Cheap pre-pass that
        // would catch a JMT-side bug before we wrote into the wrong DB.
        for (node_key, _) in tree_update_batch.node_batch.iter().flatten() {
            ensure!(node_key.get_shard_id() == shard_id, "shard_id mismatch");
        }
        for row in tree_update_batch.stale_node_index_batch.iter().flatten() {
            ensure!(row.node_key.get_shard_id() == shard_id, "shard_id mismatch");
        }

        let mut batch = self.db(shard_id).new_native_batch();
        populate_jmt_writes(&mut batch, tree_update_batch, previous_epoch_ending_version)?;
        Self::put_progress(Some(version), shard_id, &mut batch)?;

        batch.into_raw_batch(self.db(shard_id))
    }

    pub(crate) fn put_progress(
        version: Option<Version>,
        shard_id: Option<usize>,
        batch: &mut impl WriteBatch,
    ) -> Result<()> {
        let key = if let Some(shard_id) = shard_id {
            DbMetadataKey::StateMerkleShardCommitProgress(shard_id)
        } else {
            DbMetadataKey::StateMerkleCommitProgress
        };

        if let Some(version) = version {
            batch.put::<DbMetadataSchema>(&key, &DbMetadataValue::Version(version))
        } else {
            batch.delete::<DbMetadataSchema>(&key)
        }
    }

    /// Test-only helper accepting KV updates from all shards.
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
                    sharded_value_set[shard_id].clone(),
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
    pub fn merklize_value_set_for_shard(
        &self,
        shard_id: usize,
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
            let _timer =
                OTHER_TIMERS_SECONDS.timer_with(&[&format!("{}__jmt_update", self.db_tag)]);

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

    /// Per-snapshot JMT merklize pass shared by both main-state and
    /// position pipelines. Runs `merklize_value_set_for_shard × 16` in
    /// parallel (on the non-exec CPU pool), feeding each shard the
    /// pre-computed `new_node_hashes_since` from the scratchpad SMT,
    /// then aggregates via `calculate_top_levels`.
    ///
    /// Caller pre-shards `all_updates` by `state_key_hash.nibble(0)` —
    /// state's `make_delta` returns the pre-sharded form directly;
    /// position's caller fans out a flat `Vec` before calling.
    ///
    /// Asserts that the resulting JMT root hash matches the in-memory
    /// SMT root, catching scratchpad/JMT drift early.
    ///
    /// Returns `(root_hash, leaf_count, top_levels_batch, batches_for_shards)`.
    pub fn merklize_pass(
        &self,
        base_version: Option<Version>,
        version: Version,
        last_smt: &aptos_scratchpad::SparseMerkleTree,
        smt: &aptos_scratchpad::SparseMerkleTree,
        all_updates: [Vec<(HashValue, Option<(HashValue, StateKey)>)>; NUM_STATE_SHARDS],
        previous_epoch_ending_version: Option<Version>,
    ) -> Result<(HashValue, usize, RawBatch, Vec<RawBatch>)> {
        let shard_persisted_versions = self.get_shard_persisted_versions(base_version)?;

        let (shard_root_nodes, batches_for_shards): (Vec<_>, Vec<_>) =
            THREAD_MANAGER.get_non_exe_cpu_pool().install(|| {
                let _timer = OTHER_TIMERS_SECONDS.timer_with(&["calculate_batches_for_shards"]);
                all_updates
                    .into_par_iter()
                    .enumerate()
                    .map(|(shard_id, updates)| {
                        let node_hashes = smt.new_node_hashes_since(last_smt, shard_id as u8);
                        let updates_refs: Vec<_> =
                            updates.iter().map(|(h, v)| (*h, v.as_ref())).collect();
                        self.merklize_value_set_for_shard(
                            shard_id,
                            updates_refs,
                            Some(&node_hashes),
                            version,
                            base_version,
                            shard_persisted_versions[shard_id],
                            previous_epoch_ending_version,
                        )
                    })
                    .collect::<Result<Vec<_>>>()
                    .expect("Error calculating shard JMT batches.")
                    .into_iter()
                    .unzip()
            });

        let _timer = OTHER_TIMERS_SECONDS.timer_with(&["calculate_top_levels_batch"]);
        let (root_hash, leaf_count, top_levels_batch) = self.calculate_top_levels(
            shard_root_nodes,
            version,
            base_version,
            previous_epoch_ending_version,
        )?;
        assert_eq!(
            root_hash,
            smt.root_hash(),
            "JMT vs SMT root hash mismatch — scratchpad/JMT drift detected: jmt={}, smt={}",
            root_hash,
            smt.root_hash()
        );

        Ok((root_hash, leaf_count, top_levels_batch, batches_for_shards))
    }

    pub(crate) fn write_pruner_progress(
        &self,
        progress_key: &DbMetadataKey,
        version: Version,
    ) -> Result<()> {
        self.metadata_db
            .put::<DbMetadataSchema>(progress_key, &DbMetadataValue::Version(version))
    }

    fn db_by_key(&self, node_key: &NodeKey) -> &DB {
        if let Some(shard_id) = node_key.get_shard_id() {
            self.db_shard(shard_id)
        } else {
            self.metadata_db()
        }
    }

    /// Finds the rightmost leaf by scanning the entire DB. Test-only.
    #[cfg(test)]
    pub fn get_rightmost_leaf_naive(
        &self,
        version: Version,
    ) -> Result<Option<(NodeKey, LeafNode)>> {
        let mut ret = None;
        let shards = 0..self.num_shards();
        let start_num_of_nibbles = 1;
        for shard_id in shards.rev() {
            let shard_db = self.shards[shard_id].clone();
            let mut shard_iter = shard_db.iter::<JellyfishMerkleNodeSchema>()?;
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
        shard_id: usize,
    ) -> Result<Option<(NodeKey, LeafNode)>> {
        assert!(
            shard_id < NUM_STATE_SHARDS,
            "Invalid shard_id: {}",
            shard_id
        );
        let shard_db = self.shards[shard_id].clone();
        let mut ret = None;

        for num_nibbles in 0..=ROOT_NIBBLE_HEIGHT {
            let mut iter = shard_db.iter::<JellyfishMerkleNodeSchema>()?;
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

impl TreeReader<StateKey> for ShardedJmtMerkleDb {
    fn get_node_option(&self, node_key: &NodeKey, tag: &str) -> Result<Option<Node>> {
        let start_time = Instant::now();
        if !self.cache_enabled() {
            let node_opt = self
                .db_by_key(node_key)
                .get::<JellyfishMerkleNodeSchema>(node_key)?;
            NODE_CACHE_SECONDS.observe_with(
                &[tag, "cache_disabled", self.db_tag],
                start_time.elapsed().as_secs_f64(),
            );
            return Ok(node_opt);
        }
        if let Some(node_cache) = self
            .version_caches
            .get(&node_key.get_shard_id())
            .unwrap()
            .get_version(node_key.version())
        {
            let node = node_cache.get(node_key).cloned();
            NODE_CACHE_SECONDS.observe_with(
                &[tag, "versioned_cache_hit", self.db_tag],
                start_time.elapsed().as_secs_f64(),
            );
            return Ok(node);
        }

        if let Some(lru_cache) = &self.lru_cache {
            if let Some(node) = lru_cache.get(node_key) {
                NODE_CACHE_SECONDS.observe_with(
                    &[tag, "lru_cache_hit", self.db_tag],
                    start_time.elapsed().as_secs_f64(),
                );
                return Ok(Some(node));
            }
        }

        let node_opt = self
            .db_by_key(node_key)
            .get::<JellyfishMerkleNodeSchema>(node_key)?;
        if let Some(lru_cache) = &self.lru_cache {
            if let Some(node) = &node_opt {
                lru_cache.put(node_key.clone(), node.clone());
            }
        }
        NODE_CACHE_SECONDS.observe_with(
            &[tag, "cache_miss", self.db_tag],
            start_time.elapsed().as_secs_f64(),
        );
        Ok(node_opt)
    }

    fn get_rightmost_leaf(&self, version: Version) -> Result<Option<(NodeKey, LeafNode)>> {
        let ret = None;
        let shards = 0..NUM_STATE_SHARDS;

        // Search from right to left to find the first leaf node.
        for shard_id in shards.rev() {
            if let Some((node_key, leaf_node)) =
                self.get_rightmost_leaf_in_single_shard(version, shard_id)?
            {
                return Ok(Some((node_key, leaf_node)));
            }
        }

        Ok(ret)
    }
}

impl TreeWriter<StateKey> for ShardedJmtMerkleDb {
    fn write_node_batch(&self, node_batch: &NodeBatch) -> Result<()> {
        let _timer = OTHER_TIMERS_SECONDS
            .timer_with(&[&format!("{}__tree_writer_write_batch", self.db_tag)]);
        // Get the top level batch and sharded batch from raw NodeBatch
        let mut top_level_batch = SchemaBatch::new();
        let mut jmt_shard_batches: Vec<SchemaBatch> = Vec::with_capacity(NUM_STATE_SHARDS);
        jmt_shard_batches.resize_with(NUM_STATE_SHARDS, SchemaBatch::new);
        node_batch.iter().try_for_each(|(node_key, node)| {
            if let Some(shard_id) = node_key.get_shard_id() {
                jmt_shard_batches[shard_id].put::<JellyfishMerkleNodeSchema>(node_key, node)
            } else {
                top_level_batch.put::<JellyfishMerkleNodeSchema>(node_key, node)
            }
        })?;
        self.commit_no_progress(top_level_batch, jmt_shard_batches)
    }
}

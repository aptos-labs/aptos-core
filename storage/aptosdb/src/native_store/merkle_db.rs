// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![forbid(unsafe_code)]

use crate::{
    db_options::gen_native_merkle_cfds,
    native_store::{kv_db::NativeKvDb, NativeStoreSpec, PositionStore},
    sharded_db_layout::{ShardedDbNames, ShardedDbOpenOptions},
    sharded_jmt_merkle_db::{LeafNode, Node as ShardedNode, ShardedJmtMerkleDb},
    state_value_chunk::jmt_leaves_with_values,
    utils::truncation_helper::find_closest_node_version_at_or_before,
};
use aptos_config::config::{RocksdbConfig, StorageDirPaths};
use aptos_crypto::HashValue;
use aptos_jellyfish_merkle::{node_type::NodeKey, JellyfishMerkleTree, TreeReader, TreeWriter};
use aptos_logger::info;
use aptos_schemadb::{Cache, Env};
use aptos_storage_interface::{AptosDbError, Result};
use aptos_types::{
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::Version,
};
use std::{marker::PhantomData, ops::Deref, path::Path, sync::Arc};

pub(crate) type PositionMerkleDb = NativeMerkleDb<PositionStore>;

/// A native store's JMT: one metadata DB for the top levels plus
/// `NUM_NATIVE_MERKLE_SHARDS` shards.
#[derive(Debug)]
pub(crate) struct NativeMerkleDb<T> {
    inner: ShardedJmtMerkleDb,
    _phantom: PhantomData<T>,
}

impl<T> Deref for NativeMerkleDb<T> {
    type Target = ShardedJmtMerkleDb;

    fn deref(&self) -> &ShardedJmtMerkleDb {
        &self.inner
    }
}

impl<T: NativeStoreSpec> NativeMerkleDb<T> {
    fn names() -> ShardedDbNames {
        ShardedDbNames {
            folder: T::MERKLE_DB_FOLDER_NAME,
            metadata_db: T::MERKLE_METADATA_DB_NAME,
            shard_db_prefix: T::MERKLE_SHARD_DB_NAME_PREFIX,
        }
    }

    pub fn new(
        db_paths: &StorageDirPaths,
        rocksdb_config: RocksdbConfig,
        env: Option<&Env>,
        block_cache: Option<&Cache>,
        readonly: bool,
        max_nodes_per_lru_cache_shard: usize,
    ) -> Result<Self> {
        let (metadata_db, shards) = Self::names().open(
            T::merkle_db_metadata_root_path(db_paths),
            |shard_id| T::merkle_db_shard_root_path(db_paths, shard_id).as_path(),
            &ShardedDbOpenOptions {
                rocksdb_config: &rocksdb_config,
                env,
                block_cache,
                readonly,
                delete_on_restart: false,
            },
            gen_native_merkle_cfds,
        )?;
        info!(store = T::NAME, "Opened {}.", T::MERKLE_DB_FOLDER_NAME);
        let inner =
            ShardedJmtMerkleDb::new(metadata_db, shards, max_nodes_per_lru_cache_shard, T::NAME);
        Ok(Self {
            inner,
            _phantom: PhantomData,
        })
    }

    pub(crate) fn create_checkpoint(
        db_root_path: impl AsRef<Path>,
        cp_root_path: impl AsRef<Path>,
    ) -> Result<()> {
        let merkle_db = Self::new(
            &StorageDirPaths::from_path(db_root_path),
            RocksdbConfig::default(),
            None,
            None,
            false,
            0,
        )?;
        Self::names().create_checkpoint(
            merkle_db.metadata_db(),
            merkle_db.shards(),
            cp_root_path.as_ref(),
        )
    }

    /// Returns the JMT root at exactly `version`.
    ///
    /// **Precondition:** `version` must be a snapshot version (one at
    /// which the store's commit actually wrote nodes). Native-store
    /// snapshots are sparse — many ledger versions have no JMT
    /// nodes — so callers holding an arbitrary ledger version must first
    /// resolve it via [`Self::latest_snapshot_version_at_or_before`].
    /// Returns an error if no snapshot exists at `version`.
    pub fn get_root_hash(&self, version: Version) -> Result<HashValue> {
        let tree = JellyfishMerkleTree::<_, StateKey>::new(&self.inner);
        tree.get_root_hash_option(version)
            .map_err(|e| {
                AptosDbError::Other(format!("{} get_root_hash: {e}", T::MERKLE_DB_FOLDER_NAME))
            })?
            .ok_or_else(|| {
                AptosDbError::Other(format!(
                    "{}: no JMT snapshot at version {version}; \
                     callers must pass a snapshot version (see \
                     latest_snapshot_version_at_or_before)",
                    T::MERKLE_DB_FOLDER_NAME,
                ))
            })
    }

    /// Resolves an arbitrary ledger `version` to the latest JMT snapshot
    /// version at or before it. Returns `None` if no snapshot exists at
    /// or before `version` (e.g. the store is fresh
    /// and `version` precedes its first write).
    pub fn latest_snapshot_version_at_or_before(
        &self,
        version: Version,
    ) -> Result<Option<Version>> {
        find_closest_node_version_at_or_before(self.metadata_db(), version)
    }

    /// JMT-walk + KV-CF value join: yields `(StateKey, StateValue)` for
    /// every live leaf at `version`, starting at JMT index `start_idx`.
    /// Thin wrapper around [`jmt_leaves_with_values`] with the tree's
    /// value lookup baked in.
    ///
    /// **Precondition:** same as [`Self::get_root_hash`] — `version`
    /// must be a snapshot version.
    pub fn iter_active_leaves_with_values(
        self: &Arc<Self>,
        kv_db: Arc<NativeKvDb<T>>,
        version: Version,
        start_idx: usize,
    ) -> Result<impl Iterator<Item = Result<(StateKey, StateValue)>> + Send + Sync + use<T>> {
        jmt_leaves_with_values(
            Arc::clone(self),
            version,
            start_idx,
            move |key, leaf_version| kv_db.expect_value_by_version(key, leaf_version),
        )
    }

    /// Bounded variant of [`Self::iter_active_leaves_with_values`] —
    /// takes at most `chunk_size` leaves starting at `first_index`.
    ///
    /// **Precondition:** same as [`Self::get_root_hash`] — `version`
    /// must be a snapshot version.
    pub fn iter_active_leaves_chunk(
        self: &Arc<Self>,
        kv_db: Arc<NativeKvDb<T>>,
        version: Version,
        first_index: usize,
        chunk_size: usize,
    ) -> Result<impl Iterator<Item = Result<(StateKey, StateValue)>> + Send + Sync + use<T>> {
        Ok(self
            .iter_active_leaves_with_values(kv_db, version, first_index)?
            .take(chunk_size))
    }
}

impl<T: NativeStoreSpec> TreeReader<StateKey> for NativeMerkleDb<T> {
    fn get_node_option(&self, node_key: &NodeKey, tag: &str) -> Result<Option<ShardedNode>> {
        self.inner.get_node_option(node_key, tag)
    }

    fn get_rightmost_leaf(&self, version: Version) -> Result<Option<(NodeKey, LeafNode)>> {
        self.inner.get_rightmost_leaf(version)
    }
}

impl<T: NativeStoreSpec> TreeWriter<StateKey> for NativeMerkleDb<T> {
    fn write_node_batch(
        &self,
        node_batch: &aptos_jellyfish_merkle::NodeBatch<StateKey>,
    ) -> Result<()> {
        self.inner.write_node_batch(node_batch)
    }
}

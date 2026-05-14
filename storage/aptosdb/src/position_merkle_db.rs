// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Dedicated JMT for native-position keys.

#![forbid(unsafe_code)]

use crate::{
    db_options::gen_position_merkle_cfds,
    sharded_jmt_merkle_db::{LeafNode, Node as ShardedNode, ShardedJmtMerkleDb},
};
use aptos_config::config::RocksdbConfig;
use aptos_crypto::{hash::SPARSE_MERKLE_PLACEHOLDER_HASH, HashValue};
use aptos_jellyfish_merkle::{
    iterator::JellyfishMerkleIterator, node_type::NodeKey, TreeReader, TreeWriter,
};
use aptos_logger::info;
use aptos_rocksdb_options::gen_rocksdb_options;
use aptos_schemadb::{Cache, Env, DB};
use aptos_storage_interface::{AptosDbError, Result};
use aptos_types::{
    state_store::{state_key::StateKey, NUM_STATE_SHARDS},
    transaction::Version,
};
use rayon::prelude::*;
use std::{
    ops::Deref,
    path::{Path, PathBuf},
    sync::Arc,
};

pub const NUM_POSITION_MERKLE_SHARDS: usize = NUM_STATE_SHARDS;

#[derive(Debug)]
pub struct PositionMerkleDb {
    inner: ShardedJmtMerkleDb,
}

impl Deref for PositionMerkleDb {
    type Target = ShardedJmtMerkleDb;

    fn deref(&self) -> &ShardedJmtMerkleDb {
        &self.inner
    }
}

impl PositionMerkleDb {
    pub fn new(
        path: &Path,
        rocksdb_config: RocksdbConfig,
        env: Option<&Env>,
        block_cache: Option<&Cache>,
        readonly: bool,
        max_nodes_per_lru_cache_shard: usize,
    ) -> Result<Self> {
        let metadata_db_path = path.join("metadata");
        let metadata_db = Arc::new(Self::open_db(
            metadata_db_path.clone(),
            "position_merkle_db_metadata",
            &rocksdb_config,
            env,
            block_cache,
            readonly,
        )?);
        info!(
            metadata_db_path = %metadata_db_path.display(),
            "Opened position_merkle_db metadata db."
        );

        let shards: [Arc<DB>; NUM_POSITION_MERKLE_SHARDS] = (0..NUM_POSITION_MERKLE_SHARDS)
            .into_par_iter()
            .map(|shard_id| {
                let db =
                    Self::open_shard(path, shard_id, &rocksdb_config, env, block_cache, readonly)
                        .unwrap_or_else(|e| {
                            panic!("Failed to open position_merkle_db shard {shard_id}: {e:?}.")
                        });
                Arc::new(db)
            })
            .collect::<Vec<_>>()
            .try_into()
            .expect("Collected exactly NUM_POSITION_MERKLE_SHARDS shards");

        let inner = ShardedJmtMerkleDb::new(
            metadata_db,
            shards,
            max_nodes_per_lru_cache_shard,
            "position",
        );
        Ok(Self { inner })
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn new_uniform_for_test(db: Arc<DB>) -> Self {
        let shards: [Arc<DB>; NUM_POSITION_MERKLE_SHARDS] =
            std::array::from_fn(|_| Arc::clone(&db));
        let inner = ShardedJmtMerkleDb::new(db, shards, 0, "position");
        Self { inner }
    }

    fn open_shard<P: AsRef<Path>>(
        db_root_path: P,
        shard_id: usize,
        rocksdb_config: &RocksdbConfig,
        env: Option<&Env>,
        block_cache: Option<&Cache>,
        readonly: bool,
    ) -> Result<DB> {
        let shard_path = db_root_path.as_ref().join(format!("shard_{shard_id}"));
        let name = format!("position_merkle_db_shard_{shard_id}");
        Self::open_db(
            shard_path,
            &name,
            rocksdb_config,
            env,
            block_cache,
            readonly,
        )
    }

    fn open_db(
        path: PathBuf,
        name: &str,
        rocksdb_config: &RocksdbConfig,
        env: Option<&Env>,
        block_cache: Option<&Cache>,
        readonly: bool,
    ) -> Result<DB> {
        let rocksdb_opts = gen_rocksdb_options(rocksdb_config, env, readonly);
        let cfds = gen_position_merkle_cfds(rocksdb_config, block_cache);
        let res = if readonly {
            DB::open_cf_readonly(rocksdb_opts, path.as_path(), name, cfds)
        } else {
            DB::open_cf(rocksdb_opts, path.as_path(), name, cfds)
        };
        res.map_err(|e| AptosDbError::Other(format!("failed to open {name}: {e}")))
    }

    pub fn create_checkpoint(&self, cp_root_path: &Path) -> Result<()> {
        let target = cp_root_path.join("position_merkle_db");
        std::fs::remove_dir_all(&target).unwrap_or(());
        std::fs::create_dir_all(&target)
            .map_err(|e| AptosDbError::Other(format!("create_checkpoint mkdir {target:?}: {e}")))?;
        self.metadata_db()
            .create_checkpoint(target.join("metadata"))?;
        for shard_id in 0..NUM_POSITION_MERKLE_SHARDS {
            self.db_shard(shard_id)
                .create_checkpoint(target.join(format!("shard_{shard_id}")))?;
        }
        Ok(())
    }

    pub fn get_root_hash(&self, version: Version) -> Result<HashValue> {
        let tree = aptos_jellyfish_merkle::JellyfishMerkleTree::new(&self.inner);
        match tree.get_root_hash_option(version) {
            Ok(Some(h)) => Ok(h),
            Ok(None) => Ok(*SPARSE_MERKLE_PLACEHOLDER_HASH),
            Err(e) => Err(AptosDbError::Other(format!(
                "position_merkle_db get_root_hash: {e}"
            ))),
        }
    }

    pub fn iter_active_leaves(
        self: &Arc<Self>,
        version: Version,
    ) -> Result<impl Iterator<Item = Result<(StateKey, HashValue)>>> {
        let iter = JellyfishMerkleIterator::<Self, StateKey>::new(
            Arc::clone(self),
            version,
            HashValue::zero(),
        )?;
        Ok(iter.map(|res| res.map(|(key_hash, (state_key, _leaf_version))| (state_key, key_hash))))
    }
}

impl TreeReader<StateKey> for PositionMerkleDb {
    fn get_node_option(&self, node_key: &NodeKey, tag: &str) -> Result<Option<ShardedNode>> {
        self.inner.get_node_option(node_key, tag)
    }

    fn get_rightmost_leaf(&self, version: Version) -> Result<Option<(NodeKey, LeafNode)>> {
        self.inner.get_rightmost_leaf(version)
    }
}

impl TreeWriter<StateKey> for PositionMerkleDb {
    fn write_node_batch(
        &self,
        node_batch: &aptos_jellyfish_merkle::NodeBatch<StateKey>,
    ) -> Result<()> {
        self.inner.write_node_batch(node_batch)
    }
}

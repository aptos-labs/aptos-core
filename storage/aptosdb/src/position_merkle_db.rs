// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![forbid(unsafe_code)]

use crate::{
    db_options::gen_position_merkle_cfds,
    sharded_jmt_merkle_db::{LeafNode, Node as ShardedNode, ShardedJmtMerkleDb},
};
use aptos_config::config::{RocksdbConfig, StorageDirPaths};
use aptos_jellyfish_merkle::{node_type::NodeKey, TreeReader, TreeWriter};
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

const POSITION_MERKLE_DB_FOLDER: &str = "position_merkle_db";

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
        db_paths: &StorageDirPaths,
        rocksdb_config: RocksdbConfig,
        env: Option<&Env>,
        block_cache: Option<&Cache>,
        readonly: bool,
        max_nodes_per_lru_cache_shard: usize,
    ) -> Result<Self> {
        let metadata_db_path =
            Self::metadata_db_path(db_paths.position_merkle_db_metadata_root_path());
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
                let shard_root = db_paths.position_merkle_db_shard_root_path(shard_id);
                let db = Self::open_shard(
                    shard_root,
                    shard_id,
                    &rocksdb_config,
                    env,
                    block_cache,
                    readonly,
                )
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

    #[allow(dead_code)]
    pub(crate) fn create_checkpoint(
        db_root_path: impl AsRef<Path>,
        cp_root_path: impl AsRef<Path>,
    ) -> Result<()> {
        let pm = Self::new(
            &StorageDirPaths::from_path(db_root_path),
            RocksdbConfig::default(),
            None,
            None,
            false,
            0,
        )?;
        let cp = cp_root_path.as_ref().join(POSITION_MERKLE_DB_FOLDER);
        info!(cp = %cp.display(), "Creating position_merkle_db checkpoint.");
        std::fs::remove_dir_all(&cp).unwrap_or(());
        std::fs::create_dir_all(&cp)
            .map_err(|e| AptosDbError::Other(format!("create_checkpoint mkdir {cp:?}: {e}")))?;
        pm.metadata_db()
            .create_checkpoint(Self::metadata_db_path(cp_root_path.as_ref()))?;
        for shard_id in 0..NUM_POSITION_MERKLE_SHARDS {
            pm.db_shard(shard_id)
                .create_checkpoint(Self::db_shard_path(cp_root_path.as_ref(), shard_id))?;
        }
        Ok(())
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
        let shard_path = Self::db_shard_path(db_root_path, shard_id);
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

    fn db_shard_path<P: AsRef<Path>>(db_root_path: P, shard_id: usize) -> PathBuf {
        db_root_path
            .as_ref()
            .join(POSITION_MERKLE_DB_FOLDER)
            .join(format!("shard_{shard_id}"))
    }

    fn metadata_db_path<P: AsRef<Path>>(db_root_path: P) -> PathBuf {
        db_root_path
            .as_ref()
            .join(POSITION_MERKLE_DB_FOLDER)
            .join("metadata")
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

// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Main-state merkle DB. Thin wrapper around the shared
//! [`crate::sharded_jmt_merkle_db::ShardedJmtMerkleDb`] substrate that
//! adds the state-specific bits: opening shards via
//! `StorageDirPaths::{state,hot_state}_merkle_db_*_root_path`,
//! hot-vs-cold folder naming, the truncation-on-startup integration,
//! and `create_checkpoint`.
//!
//! `Deref` to `ShardedJmtMerkleDb` so all the JMT/commit/read methods
//! (commit, merklize_value_set_for_shard, calculate_top_levels,
//! get_root_hash, etc.) keep working on `&StateMerkleDb` exactly as
//! before. `TreeReader<StateKey>` / `TreeWriter<StateKey>` are
//! delegated.

#![forbid(unsafe_code)]

pub(crate) use crate::sharded_jmt_merkle_db::{LeafNode, Node};
use crate::{
    db_options::gen_state_merkle_cfds,
    sharded_jmt_merkle_db::ShardedJmtMerkleDb,
    utils::truncation_helper::{get_state_merkle_commit_progress, truncate_state_merkle_db_shards},
};
use aptos_config::config::{RocksdbConfig, StorageDirPaths};
use aptos_jellyfish_merkle::{node_type::NodeKey, TreeReader, TreeWriter};
use aptos_logger::prelude::*;
use aptos_rocksdb_options::gen_rocksdb_options;
use aptos_schemadb::{Cache, Env, DB};
use aptos_storage_interface::{db_ensure as ensure, AptosDbError, Result};
use aptos_types::{
    state_store::{state_key::StateKey, NUM_STATE_SHARDS},
    transaction::Version,
};
use std::{
    ops::Deref,
    path::{Path, PathBuf},
    sync::Arc,
};

fn db_folder_name(is_hot: bool) -> &'static str {
    if is_hot {
        "hot_state_merkle_db"
    } else {
        "state_merkle_db"
    }
}

fn metadata_db_name(is_hot: bool) -> &'static str {
    if is_hot {
        "hot_state_merkle_metadata_db"
    } else {
        "state_merkle_metadata_db"
    }
}

#[derive(Debug)]
pub struct StateMerkleDb {
    inner: ShardedJmtMerkleDb,
    is_hot: bool,
}

impl Deref for StateMerkleDb {
    type Target = ShardedJmtMerkleDb;

    fn deref(&self) -> &ShardedJmtMerkleDb {
        &self.inner
    }
}

impl StateMerkleDb {
    pub(crate) fn is_hot(&self) -> bool {
        self.is_hot
    }

    pub(crate) fn new(
        db_paths: &StorageDirPaths,
        state_merkle_db_config: RocksdbConfig,
        env: Option<&Env>,
        block_cache: Option<&Cache>,
        readonly: bool,
        // TODO(grao): Currently when this value is set to 0 we disable both caches. This is
        // hacky, need to revisit.
        max_nodes_per_lru_cache_shard: usize,
        is_hot: bool,
        delete_on_restart: bool,
    ) -> Result<Self> {
        assert!(
            !delete_on_restart || is_hot,
            "Only hot state can be cleared on restart"
        );

        Self::open(
            db_paths,
            state_merkle_db_config,
            env,
            block_cache,
            readonly,
            max_nodes_per_lru_cache_shard,
            is_hot,
            delete_on_restart,
        )
    }

    pub(crate) fn create_checkpoint(
        db_root_path: impl AsRef<Path>,
        cp_root_path: impl AsRef<Path>,
        is_hot: bool,
    ) -> Result<()> {
        // TODO(grao): Support path override here.
        let state_merkle_db = Self::new(
            &StorageDirPaths::from_path(db_root_path),
            RocksdbConfig::default(),
            /*env=*/ None,
            /*block_cache=*/ None,
            /*readonly=*/ false,
            /*max_nodes_per_lru_cache_shard=*/ 0,
            is_hot,
            /* delete_on_restart = */ false,
        )?;
        let cp_state_merkle_db_path = cp_root_path.as_ref().join(db_folder_name(is_hot));

        info!("Creating state_merkle_db checkpoint at: {cp_state_merkle_db_path:?}");

        std::fs::remove_dir_all(&cp_state_merkle_db_path).unwrap_or(());
        std::fs::create_dir_all(&cp_state_merkle_db_path).unwrap_or(());

        state_merkle_db
            .metadata_db()
            .create_checkpoint(Self::metadata_db_path(cp_root_path.as_ref(), is_hot))?;

        for shard_id in 0..NUM_STATE_SHARDS {
            state_merkle_db
                .db_shard(shard_id)
                .create_checkpoint(Self::db_shard_path(cp_root_path.as_ref(), shard_id, is_hot))?;
        }

        Ok(())
    }

    fn open(
        db_paths: &StorageDirPaths,
        state_merkle_db_config: RocksdbConfig,
        env: Option<&Env>,
        block_cache: Option<&Cache>,
        readonly: bool,
        max_nodes_per_lru_cache_shard: usize,
        is_hot: bool,
        delete_on_restart: bool,
    ) -> Result<Self> {
        let state_merkle_metadata_db_path = Self::metadata_db_path(
            if is_hot {
                db_paths.hot_state_merkle_db_metadata_root_path()
            } else {
                db_paths.state_merkle_db_metadata_root_path()
            },
            is_hot,
        );

        let (metadata_db, shards) = std::thread::scope(|s| {
            let metadata_handle = s.spawn(|| {
                Self::open_db(
                    state_merkle_metadata_db_path.clone(),
                    metadata_db_name(is_hot),
                    &state_merkle_db_config,
                    env,
                    block_cache,
                    readonly,
                    delete_on_restart,
                )
            });

            let shard_handles: Vec<_> = (0..NUM_STATE_SHARDS)
                .map(|shard_id| {
                    s.spawn(move || {
                        let shard_root_path = if is_hot {
                            db_paths.hot_state_merkle_db_shard_root_path(shard_id)
                        } else {
                            db_paths.state_merkle_db_shard_root_path(shard_id)
                        };
                        let db = Self::open_shard(
                            shard_root_path,
                            shard_id,
                            &state_merkle_db_config,
                            env,
                            block_cache,
                            readonly,
                            is_hot,
                            delete_on_restart,
                        )
                        .unwrap_or_else(|e| {
                            panic!("Failed to open state merkle db shard {shard_id}: {e:?}.")
                        });
                        Arc::new(db)
                    })
                })
                .collect();

            // Joined in shard-id order so each array index matches its shard id.
            let shards = shard_handles
                .into_iter()
                .map(|handle| {
                    handle
                        .join()
                        .expect("State merkle shard open thread panicked")
                })
                .collect::<Vec<_>>();
            let metadata_db = metadata_handle
                .join()
                .expect("State merkle metadata open thread panicked");
            (metadata_db, shards)
        });

        let state_merkle_metadata_db = Arc::new(metadata_db?);

        info!(
            state_merkle_metadata_db_path = state_merkle_metadata_db_path,
            "Opened state merkle metadata db!"
        );

        let state_merkle_db_shards: [Arc<DB>; NUM_STATE_SHARDS] = shards
            .try_into()
            .expect("Collected exactly NUM_STATE_SHARDS shards");

        let db_tag: &'static str = if is_hot { "hot" } else { "cold" };
        let inner = ShardedJmtMerkleDb::new(
            state_merkle_metadata_db,
            state_merkle_db_shards,
            max_nodes_per_lru_cache_shard,
            db_tag,
        );

        let state_merkle_db = Self { inner, is_hot };

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
        shard_id: usize,
        state_merkle_db_config: &RocksdbConfig,
        env: Option<&Env>,
        block_cache: Option<&Cache>,
        readonly: bool,
        is_hot: bool,
        delete_on_restart: bool,
    ) -> Result<DB> {
        let db_name = if is_hot {
            format!("hot_state_merkle_db_shard_{}", shard_id)
        } else {
            format!("state_merkle_db_shard_{}", shard_id)
        };
        Self::open_db(
            Self::db_shard_path(db_root_path, shard_id, is_hot),
            &db_name,
            state_merkle_db_config,
            env,
            block_cache,
            readonly,
            delete_on_restart,
        )
    }

    fn open_db(
        path: PathBuf,
        name: &str,
        state_merkle_db_config: &RocksdbConfig,
        env: Option<&Env>,
        block_cache: Option<&Cache>,
        readonly: bool,
        delete_on_restart: bool,
    ) -> Result<DB> {
        if delete_on_restart {
            ensure!(!readonly, "Should not reset DB in read-only mode.");
            info!("delete_on_restart is true. Removing {path:?} entirely.");
            std::fs::remove_dir_all(&path).unwrap_or(());
        }

        Ok(if readonly {
            DB::open_cf_readonly(
                gen_rocksdb_options(state_merkle_db_config, env, true),
                path,
                name,
                gen_state_merkle_cfds(state_merkle_db_config, block_cache),
            )?
        } else {
            DB::open_cf(
                gen_rocksdb_options(state_merkle_db_config, env, false),
                path,
                name,
                gen_state_merkle_cfds(state_merkle_db_config, block_cache),
            )?
        })
    }

    fn db_shard_path<P: AsRef<Path>>(db_root_path: P, shard_id: usize, is_hot: bool) -> PathBuf {
        let shard_sub_path = format!("shard_{}", shard_id);
        db_root_path
            .as_ref()
            .join(db_folder_name(is_hot))
            .join(Path::new(&shard_sub_path))
    }

    fn metadata_db_path<P: AsRef<Path>>(db_root_path: P, is_hot: bool) -> PathBuf {
        db_root_path
            .as_ref()
            .join(db_folder_name(is_hot))
            .join("metadata")
    }
}

impl TreeReader<StateKey> for StateMerkleDb {
    fn get_node_option(&self, node_key: &NodeKey, tag: &str) -> Result<Option<Node>> {
        self.inner.get_node_option(node_key, tag)
    }

    fn get_rightmost_leaf(&self, version: Version) -> Result<Option<(NodeKey, LeafNode)>> {
        self.inner.get_rightmost_leaf(version)
    }
}

impl TreeWriter<StateKey> for StateMerkleDb {
    fn write_node_batch(
        &self,
        node_batch: &aptos_jellyfish_merkle::NodeBatch<StateKey>,
    ) -> Result<()> {
        self.inner.write_node_batch(node_batch)
    }
}

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
    common::{HOT_STATE_MERKLE_DB_FOLDER_NAME, STATE_MERKLE_DB_FOLDER_NAME},
    db_options::gen_state_merkle_cfds,
    sharded_db_layout::{ShardedDbNames, ShardedDbOpenOptions},
    sharded_jmt_merkle_db::ShardedJmtMerkleDb,
    utils::truncation_helper::{get_state_merkle_commit_progress, truncate_state_merkle_db_shards},
};
use aptos_config::config::{RocksdbConfig, StorageDirPaths};
use aptos_jellyfish_merkle::{node_type::NodeKey, TreeReader, TreeWriter};
use aptos_schemadb::{Cache, Env};
use aptos_storage_interface::Result;
use aptos_types::{state_store::state_key::StateKey, transaction::Version};
use std::{ops::Deref, path::Path};

fn names(is_hot: bool) -> ShardedDbNames {
    if is_hot {
        ShardedDbNames {
            folder: HOT_STATE_MERKLE_DB_FOLDER_NAME,
            metadata_db: "hot_state_merkle_metadata_db",
            shard_db_prefix: "hot_state_merkle_db_shard",
        }
    } else {
        ShardedDbNames {
            folder: STATE_MERKLE_DB_FOLDER_NAME,
            metadata_db: "state_merkle_metadata_db",
            shard_db_prefix: "state_merkle_db_shard",
        }
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
        names(is_hot).create_checkpoint(
            state_merkle_db.metadata_db(),
            state_merkle_db.shards(),
            cp_root_path.as_ref(),
        )
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
        let (metadata_db, state_merkle_db_shards) = names(is_hot)
            .open(
                if is_hot {
                    db_paths.hot_state_merkle_db_metadata_root_path()
                } else {
                    db_paths.state_merkle_db_metadata_root_path()
                },
                |shard_id| {
                    if is_hot {
                        db_paths.hot_state_merkle_db_shard_root_path(shard_id)
                    } else {
                        db_paths.state_merkle_db_shard_root_path(shard_id)
                    }
                    .as_path()
                },
                &ShardedDbOpenOptions {
                    rocksdb_config: &state_merkle_db_config,
                    env,
                    block_cache,
                    readonly,
                    delete_on_restart,
                },
                gen_state_merkle_cfds,
            )
            .unwrap_or_else(|e| panic!("Failed to open state merkle db: {e:?}."));

        let db_tag: &'static str = if is_hot { "hot" } else { "cold" };
        let inner = ShardedJmtMerkleDb::new(
            metadata_db,
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

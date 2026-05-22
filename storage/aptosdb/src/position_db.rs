// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Sharded RocksDB tier for native-position value storage.
//!
//! 16 shards keyed by `state_key.get_shard_id()` (the leading nibble
//! of the StateKey hash, matching `state_kv_db` and the JMT internal
//! shard convention), plus a separate per-DB metadata DB. The shard
//! DBs hold the per-key CFs (`position_value`,
//! `stale_position_value_index`); the metadata DB holds the
//! `db_metadata` CF (pruner-progress bookkeeping). Same layout main
//! state's `state_kv_db` uses (shards + metadata DB) — no metadata is
//! ever written to a shard DB.
//!
//! Lifecycle metadata (exchange-id allocations, deny-list) lives in
//! the `aptos_experimental::native_position::ExchangeRegistry` Move
//! resource at `@aptos_framework`, not here. There is no
//! `position_metadata` CF.
//!
//! See `PLAN_native_position.md` for design rationale.

#![forbid(unsafe_code)]

use crate::{
    db_options::{gen_position_cfds, gen_position_metadata_cfds},
    schema::{
        db_metadata::{DbMetadataKey, DbMetadataSchema, DbMetadataValue},
        position_value::PositionValueSchema,
    },
    sharded_kv_db::ShardedKvDb,
};
use aptos_config::config::RocksdbConfig;
use aptos_crypto::HashValue;
use aptos_experimental_runtimes::thread_manager::THREAD_MANAGER;
use aptos_logger::info;
use aptos_rocksdb_options::gen_rocksdb_options;
use aptos_schemadb::{batch::SchemaBatch, Cache, Env, DB};
use aptos_storage_interface::{AptosDbError, Result};
use aptos_types::{state_store::NUM_STATE_SHARDS, transaction::Version};
use rayon::prelude::*;
use std::{
    ops::Deref,
    path::{Path, PathBuf},
    sync::Arc,
};

pub const NUM_NATIVE_VALUE_SHARDS: usize = NUM_STATE_SHARDS;

#[derive(Debug)]
pub struct PositionDb {
    inner: ShardedKvDb,
}

impl Deref for PositionDb {
    type Target = ShardedKvDb;

    fn deref(&self) -> &ShardedKvDb {
        &self.inner
    }
}

impl PositionDb {
    pub fn new(
        path: &Path,
        rocksdb_config: RocksdbConfig,
        env: Option<&Env>,
        block_cache: Option<&Cache>,
        readonly: bool,
    ) -> Result<Self> {
        let metadata_db_path = path.join("metadata");
        let metadata_db = Arc::new(Self::open_db(
            metadata_db_path.clone(),
            "position_db_metadata",
            &rocksdb_config,
            env,
            block_cache,
            readonly,
            true,
        )?);
        info!(
            metadata_db_path = %metadata_db_path.display(),
            "Opened position_db metadata db."
        );

        let shards: [Arc<DB>; NUM_NATIVE_VALUE_SHARDS] = (0..NUM_NATIVE_VALUE_SHARDS)
            .into_par_iter()
            .map(|shard_id| {
                let db =
                    Self::open_shard(path, shard_id, &rocksdb_config, env, block_cache, readonly)
                        .unwrap_or_else(|e| {
                            panic!("Failed to open position_db shard {shard_id}: {e:?}.")
                        });
                Arc::new(db)
            })
            .collect::<Vec<_>>()
            .try_into()
            .expect("Collected exactly NUM_NATIVE_VALUE_SHARDS shards");

        Ok(Self {
            inner: ShardedKvDb::new(metadata_db, shards),
        })
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn new_uniform_for_test(db: Arc<DB>) -> Self {
        let shards: [Arc<DB>; NUM_NATIVE_VALUE_SHARDS] = std::array::from_fn(|_| Arc::clone(&db));
        Self {
            inner: ShardedKvDb::new(Arc::clone(&db), shards),
        }
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
        let name = format!("position_db_shard_{shard_id}");
        Self::open_db(
            shard_path,
            &name,
            rocksdb_config,
            env,
            block_cache,
            readonly,
            false,
        )
    }

    fn open_db(
        path: PathBuf,
        name: &str,
        rocksdb_config: &RocksdbConfig,
        env: Option<&Env>,
        block_cache: Option<&Cache>,
        readonly: bool,
        is_metadata: bool,
    ) -> Result<DB> {
        let rocksdb_opts = gen_rocksdb_options(rocksdb_config, env, readonly);
        let cfds = if is_metadata {
            gen_position_metadata_cfds(rocksdb_config, block_cache)
        } else {
            gen_position_cfds(rocksdb_config, block_cache)
        };
        let res = if readonly {
            DB::open_cf_readonly(rocksdb_opts, path.as_path(), name, cfds)
        } else {
            DB::open_cf(rocksdb_opts, path.as_path(), name, cfds)
        };
        res.map_err(|e| AptosDbError::Other(format!("failed to open {name}: {e}")))
    }

    /// Commit per-shard batches in parallel with progress stamps.
    /// Mirrors `StateKvDb::commit`.
    pub fn commit(
        &self,
        version: Version,
        metadata_batch: Option<SchemaBatch>,
        per_shard_batches: [Option<SchemaBatch>; NUM_NATIVE_VALUE_SHARDS],
    ) -> Result<()> {
        THREAD_MANAGER.get_io_pool().scope(|s| {
            for (shard_id, batch_opt) in per_shard_batches.into_iter().enumerate() {
                s.spawn(move |_| {
                    self.commit_single_shard(version, shard_id, batch_opt)
                        .unwrap_or_else(|err| {
                            panic!("Failed to commit position shard {shard_id}: {err}.")
                        });
                });
            }
        });
        if let Some(batch) = metadata_batch {
            self.metadata_db().write_schemas(batch)?;
        }
        self.write_progress(version)
    }

    pub fn commit_single_shard(
        &self,
        version: Version,
        shard_id: usize,
        batch_opt: Option<SchemaBatch>,
    ) -> Result<()> {
        let mut batch = batch_opt.unwrap_or_else(SchemaBatch::new);
        batch.put::<DbMetadataSchema>(
            &DbMetadataKey::PositionShardCommitProgress(shard_id),
            &DbMetadataValue::Version(version),
        )?;
        self.shard(shard_id).write_schemas(batch)
    }

    pub fn write_progress(&self, version: Version) -> Result<()> {
        self.metadata_db().put::<DbMetadataSchema>(
            &DbMetadataKey::PositionCommitProgress,
            &DbMetadataValue::Version(version),
        )
    }

    pub fn write_pruner_progress(&self, version: Version) -> Result<()> {
        self.metadata_db().put::<DbMetadataSchema>(
            &DbMetadataKey::PositionPrunerProgress,
            &DbMetadataValue::Version(version),
        )
    }

    /// Most recent prior version `< at_version` at which
    /// `state_key_hash` was written.
    pub fn find_prior_version(
        &self,
        state_key_hash: HashValue,
        at_version: Version,
    ) -> Result<Option<Version>> {
        if at_version == 0 {
            return Ok(None);
        }
        let shard = ShardedKvDb::shard_of_hash(state_key_hash);
        let mut iter = self.shard(shard).iter::<PositionValueSchema>()?;
        iter.seek(&(state_key_hash, at_version - 1))?;
        if let Some(Ok(((row_hash, row_version), _value))) = iter.next() {
            if row_hash == state_key_hash {
                return Ok(Some(row_version));
            }
        }
        Ok(None)
    }
}

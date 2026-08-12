// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![forbid(unsafe_code)]

use crate::{
    db_options::gen_position_cfds,
    schema::{
        db_metadata::{DbMetadataKey, DbMetadataSchema, DbMetadataValue},
        position_value::PositionValueSchema,
    },
    sharded_kv_db::ShardedKvDb,
};
use aptos_config::config::{RocksdbConfig, StorageDirPaths};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_experimental_runtimes::thread_manager::THREAD_MANAGER;
use aptos_logger::info;
use aptos_rocksdb_options::gen_rocksdb_options;
use aptos_schemadb::{batch::SchemaBatch, Cache, Env, ReadOptions, DB};
use aptos_storage_interface::{AptosDbError, Result};
use aptos_types::{
    state_store::{state_key::StateKey, state_value::StateValue, NUM_STATE_SHARDS},
    transaction::Version,
};
use rayon::prelude::*;
use std::{
    ops::Deref,
    path::{Path, PathBuf},
    sync::Arc,
};

const POSITION_DB_FOLDER: &str = "position_db";

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
        db_paths: &StorageDirPaths,
        rocksdb_config: RocksdbConfig,
        env: Option<&Env>,
        block_cache: Option<&Cache>,
        readonly: bool,
    ) -> Result<Self> {
        let metadata_db_path = Self::metadata_db_path(db_paths.position_db_metadata_root_path());
        let metadata_db = Arc::new(Self::open_db(
            metadata_db_path.clone(),
            "position_db_metadata",
            &rocksdb_config,
            env,
            block_cache,
            readonly,
        )?);
        info!(
            metadata_db_path = %metadata_db_path.display(),
            "Opened position_db metadata db."
        );

        let shards: [Arc<DB>; NUM_NATIVE_VALUE_SHARDS] = (0..NUM_NATIVE_VALUE_SHARDS)
            .into_par_iter()
            .map(|shard_id| {
                let shard_root = db_paths.position_db_shard_root_path(shard_id);
                let db = Self::open_shard(
                    shard_root,
                    shard_id,
                    &rocksdb_config,
                    env,
                    block_cache,
                    readonly,
                )
                .unwrap_or_else(|e| panic!("Failed to open position_db shard {shard_id}: {e:?}."));
                Arc::new(db)
            })
            .collect::<Vec<_>>()
            .try_into()
            .expect("Collected exactly NUM_NATIVE_VALUE_SHARDS shards");

        Ok(Self {
            inner: ShardedKvDb::new(metadata_db, shards),
        })
    }

    #[allow(dead_code)]
    pub(crate) fn create_checkpoint(
        db_root_path: impl AsRef<Path>,
        cp_root_path: impl AsRef<Path>,
    ) -> Result<()> {
        let position_db = Self::new(
            &StorageDirPaths::from_path(db_root_path),
            RocksdbConfig::default(),
            None,
            None,
            false,
        )?;
        let cp = cp_root_path.as_ref().join(POSITION_DB_FOLDER);
        info!(cp = %cp.display(), "Creating position_db checkpoint.");
        std::fs::remove_dir_all(&cp).unwrap_or(());
        std::fs::create_dir_all(&cp)
            .map_err(|e| AptosDbError::Other(format!("create_checkpoint mkdir {cp:?}: {e}")))?;
        position_db
            .metadata_db()
            .create_checkpoint(Self::metadata_db_path(cp_root_path.as_ref()))?;
        for shard_id in 0..NUM_NATIVE_VALUE_SHARDS {
            position_db
                .shard(shard_id)
                .create_checkpoint(Self::db_shard_path(cp_root_path.as_ref(), shard_id))?;
        }
        Ok(())
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
        let name = format!("position_db_shard_{shard_id}");
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
        let cfds = gen_position_cfds(rocksdb_config, block_cache);
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
            .join(POSITION_DB_FOLDER)
            .join(format!("shard_{shard_id}"))
    }

    fn metadata_db_path<P: AsRef<Path>>(db_root_path: P) -> PathBuf {
        db_root_path
            .as_ref()
            .join(POSITION_DB_FOLDER)
            .join("metadata")
    }

    pub fn num_shards(&self) -> usize {
        NUM_NATIVE_VALUE_SHARDS
    }

    pub fn db_shard(&self, shard_id: usize) -> &DB {
        self.shard(shard_id)
    }

    pub fn metadata_db_arc(&self) -> Arc<DB> {
        Arc::clone(self.metadata_db())
    }

    pub fn db_shard_arc(&self, shard_id: usize) -> Arc<DB> {
        Arc::clone(self.shard(shard_id))
    }

    #[allow(dead_code)]
    pub(crate) fn new_sharded_native_batches(
        &self,
    ) -> [aptos_schemadb::batch::NativeBatch<'_>; NUM_NATIVE_VALUE_SHARDS] {
        std::array::from_fn(|shard_id| self.shard(shard_id).new_native_batch())
    }

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
        let mut batch = batch_opt.unwrap_or_default();
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

    /// Point lookup for the position-value row referenced by a JMT
    /// leaf. Returns `Some((version, value))` for the latest version
    /// at or before `version` for `state_key_hash`, or `None` if no
    /// such row exists. `prefix_same_as_start` constrains the seek so
    /// a different `state_key_hash` doesn't bleed in.
    pub fn get_position_value(
        &self,
        state_key_hash: HashValue,
        version: Version,
    ) -> Result<Option<(Version, StateValue)>> {
        let mut read_opts = ReadOptions::default();
        read_opts.set_prefix_same_as_start(true);
        let shard = ShardedKvDb::shard_of_hash(state_key_hash);
        let mut iter = self
            .shard(shard)
            .iter_with_opts::<PositionValueSchema>(read_opts)?;
        iter.seek(&(state_key_hash, version))?;
        Ok(iter
            .next()
            .transpose()?
            .and_then(|((_, version), value_opt)| value_opt.map(|value| (version, value))))
    }

    /// Returns the value for `key` recorded at `version`, erroring if the row is
    /// missing. Mirrors `StateStore::expect_value_by_version` for the position CF.
    pub fn expect_value_by_version(&self, key: &StateKey, version: Version) -> Result<StateValue> {
        let key_hash = key.hash();
        let (_version, value) = self.get_position_value(key_hash, version)?.ok_or_else(|| {
            AptosDbError::Other(format!(
                "position_value row missing (state_key_hash={key_hash}, version={version})"
            ))
        })?;
        Ok(value)
    }

    /// Fans `(state_key_hash, version, value)` writes into per-shard batches.
    pub(crate) fn shard_position_value_writes(
        writes: impl IntoIterator<Item = (HashValue, Version, Option<StateValue>)>,
    ) -> Result<[Option<SchemaBatch>; NUM_NATIVE_VALUE_SHARDS]> {
        let mut per_shard: [Option<SchemaBatch>; NUM_NATIVE_VALUE_SHARDS] =
            std::array::from_fn(|_| None);
        for (state_key_hash, version, maybe_value) in writes {
            let shard = ShardedKvDb::shard_of_hash(state_key_hash);
            let batch = per_shard[shard].get_or_insert_with(SchemaBatch::new);
            batch.put::<PositionValueSchema>(&(state_key_hash, version), &maybe_value)?;
        }
        Ok(per_shard)
    }

    pub fn write_position_batch(
        &self,
        version: Version,
        writes: impl IntoIterator<Item = (HashValue, Option<StateValue>)>,
    ) -> Result<()> {
        let per_shard = Self::shard_position_value_writes(
            writes
                .into_iter()
                .map(|(hash, value)| (hash, version, value)),
        )?;
        for (shard, maybe_batch) in per_shard.into_iter().enumerate() {
            if let Some(batch) = maybe_batch {
                self.shard(shard).write_schemas(batch)?;
            }
        }
        Ok(())
    }

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
        // PositionValueSchema bit-inverts the version (newest-first); a
        // forward seek lands on the largest version <= at_version-1.
        iter.seek(&(state_key_hash, at_version - 1))?;
        if let Some(Ok(((row_hash, row_version), _value))) = iter.next()
            && row_hash == state_key_hash
        {
            return Ok(Some(row_version));
        }
        Ok(None)
    }
}

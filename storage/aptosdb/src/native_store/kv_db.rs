// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![forbid(unsafe_code)]

use crate::{
    db_options::gen_native_kv_cfds,
    native_store::{NativeStoreSpec, PositionStore},
    schema::{
        db_metadata::{DbMetadataSchema, DbMetadataValue},
        native_value::NativeValueSchema,
    },
    sharded_db_layout::{ShardedDbNames, ShardedDbOpenOptions},
    sharded_kv_db::ShardedKvDb,
};
use aptos_config::config::{RocksdbConfig, StorageDirPaths};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_experimental_runtimes::thread_manager::THREAD_MANAGER;
use aptos_logger::info;
use aptos_schemadb::{batch::SchemaBatch, Cache, Env, ReadOptions};
use aptos_storage_interface::{AptosDbError, Result};
use aptos_types::{
    state_store::{state_key::StateKey, state_value::StateValue, NUM_STATE_SHARDS},
    transaction::Version,
};
use std::{marker::PhantomData, ops::Deref, path::Path};

pub const NUM_NATIVE_VALUE_SHARDS: usize = NUM_STATE_SHARDS;

pub(crate) type PositionDb = NativeKvDb<PositionStore>;

/// A native store's value DB: one metadata DB plus `NUM_NATIVE_VALUE_SHARDS`
/// shards holding the value and stale-index CFs.
#[derive(Debug)]
pub(crate) struct NativeKvDb<T> {
    inner: ShardedKvDb,
    _phantom: PhantomData<T>,
}

impl<T> Deref for NativeKvDb<T> {
    type Target = ShardedKvDb;

    fn deref(&self) -> &ShardedKvDb {
        &self.inner
    }
}

impl<T: NativeStoreSpec> NativeKvDb<T> {
    fn names() -> ShardedDbNames {
        ShardedDbNames {
            folder: T::KV_DB_FOLDER_NAME,
            metadata_db: T::KV_METADATA_DB_NAME,
            shard_db_prefix: T::KV_SHARD_DB_NAME_PREFIX,
        }
    }

    pub fn new(
        db_paths: &StorageDirPaths,
        rocksdb_config: RocksdbConfig,
        env: Option<&Env>,
        block_cache: Option<&Cache>,
        readonly: bool,
    ) -> Result<Self> {
        let (metadata_db, shards) = Self::names().open(
            T::kv_db_metadata_root_path(db_paths),
            |shard_id| T::kv_db_shard_root_path(db_paths, shard_id).as_path(),
            &ShardedDbOpenOptions {
                rocksdb_config: &rocksdb_config,
                env,
                block_cache,
                readonly,
                delete_on_restart: false,
            },
            gen_native_kv_cfds,
        )?;
        info!(store = T::NAME, "Opened {}.", T::KV_DB_FOLDER_NAME);
        Ok(Self {
            inner: ShardedKvDb::new(metadata_db, shards),
            _phantom: PhantomData,
        })
    }

    pub(crate) fn create_checkpoint(
        db_root_path: impl AsRef<Path>,
        cp_root_path: impl AsRef<Path>,
    ) -> Result<()> {
        let kv_db = Self::new(
            &StorageDirPaths::from_path(db_root_path),
            RocksdbConfig::default(),
            None,
            None,
            false,
        )?;
        Self::names().create_checkpoint(kv_db.metadata_db(), kv_db.shards(), cp_root_path.as_ref())
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
                            panic!("Failed to commit {} shard {shard_id}: {err}.", T::NAME)
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
            &T::shard_commit_progress_key(shard_id),
            &DbMetadataValue::Version(version),
        )?;
        self.shard(shard_id).write_schemas(batch)
    }

    pub fn write_progress(&self, version: Version) -> Result<()> {
        self.metadata_db().put::<DbMetadataSchema>(
            &T::commit_progress_key(),
            &DbMetadataValue::Version(version),
        )
    }

    /// Point lookup for the value row referenced by a JMT leaf. Returns
    /// `Some((version, value))` for the latest version at or before
    /// `version` for `state_key_hash`, or `None` if no such row exists.
    /// `prefix_same_as_start` constrains the seek so a different
    /// `state_key_hash` doesn't bleed in.
    pub fn get_value(
        &self,
        state_key_hash: HashValue,
        version: Version,
    ) -> Result<Option<(Version, StateValue)>> {
        let mut read_opts = ReadOptions::default();
        read_opts.set_prefix_same_as_start(true);
        let shard = ShardedKvDb::shard_of_hash(state_key_hash);
        let mut iter = self
            .shard(shard)
            .iter_with_opts::<NativeValueSchema>(read_opts)?;
        iter.seek(&(state_key_hash, version))?;
        Ok(iter
            .next()
            .transpose()?
            .and_then(|((_, version), value_opt)| value_opt.map(|value| (version, value))))
    }

    /// Returns the value for `key` recorded at `version`, erroring if the row is
    /// missing. Mirrors `StateStore::expect_value_by_version` for the value CF.
    pub fn expect_value_by_version(&self, key: &StateKey, version: Version) -> Result<StateValue> {
        let key_hash = key.hash();
        let (_version, value) = self.get_value(key_hash, version)?.ok_or_else(|| {
            AptosDbError::Other(format!(
                "{} value row missing (state_key_hash={key_hash}, version={version})",
                T::NAME,
            ))
        })?;
        Ok(value)
    }

    /// Fans `(state_key_hash, version, value)` writes into per-shard batches.
    pub(crate) fn shard_value_writes(
        writes: impl IntoIterator<Item = (HashValue, Version, Option<StateValue>)>,
    ) -> Result<[Option<SchemaBatch>; NUM_NATIVE_VALUE_SHARDS]> {
        let mut per_shard: [Option<SchemaBatch>; NUM_NATIVE_VALUE_SHARDS] =
            std::array::from_fn(|_| None);
        for (state_key_hash, version, maybe_value) in writes {
            let shard = ShardedKvDb::shard_of_hash(state_key_hash);
            let batch = per_shard[shard].get_or_insert_with(SchemaBatch::new);
            batch.put::<NativeValueSchema>(&(state_key_hash, version), &maybe_value)?;
        }
        Ok(per_shard)
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
        let mut iter = self.shard(shard).iter::<NativeValueSchema>()?;
        // The value schema bit-inverts the version (newest-first); a
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

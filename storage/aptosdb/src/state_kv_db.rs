// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{
    common::NUM_STATE_SHARDS,
    db_options::gen_state_kv_cfds,
    metrics::OTHER_TIMERS_SECONDS,
    schema::{
        db_metadata::{DbMetadataKey, DbMetadataSchema, DbMetadataValue},
        state_value::StateValueSchema,
        state_value_by_key_hash::StateValueByKeyHashSchema,
    },
    utils::truncation_helper::{get_state_kv_commit_progress, truncate_state_kv_db_shards},
};
use aptos_config::config::{RocksdbConfig, RocksdbConfigs, StorageDirPaths};
use aptos_crypto::hash::CryptoHash;
use aptos_experimental_runtimes::thread_manager::THREAD_MANAGER;
use aptos_logger::prelude::info;
use aptos_rocksdb_options::gen_rocksdb_options;
use aptos_schemadb::{ReadOptions, SchemaBatch, DB};
use aptos_storage_interface::Result;
use aptos_types::{
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::Version,
};
use arr_macro::arr;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

pub const STATE_KV_DB_FOLDER_NAME: &str = "state_kv_db";
pub const STATE_KV_METADATA_DB_NAME: &str = "state_kv_metadata_db";

pub struct StateKvDb {
    state_kv_metadata_db: Arc<DB>,
    state_kv_db_shards: [Arc<DB>; NUM_STATE_SHARDS],
    enabled_sharding: bool,
}

impl StateKvDb {
    pub(crate) fn new(
        db_paths: &StorageDirPaths,
        rocksdb_configs: RocksdbConfigs,
        readonly: bool,
        ledger_db: Arc<DB>,
    ) -> Result<Self> {
        let sharding = rocksdb_configs.enable_storage_sharding;
        if !sharding {
            info!("State K/V DB is not enabled!");
            return Ok(Self {
                state_kv_metadata_db: Arc::clone(&ledger_db),
                state_kv_db_shards: arr![Arc::clone(&ledger_db); 16],
                enabled_sharding: false,
            });
        }

        Self::open(
            db_paths,
            rocksdb_configs.state_kv_db_config,
            readonly,
            sharding,
        )
    }

    pub(crate) fn open(
        db_paths: &StorageDirPaths,
        state_kv_db_config: RocksdbConfig,
        readonly: bool,
        enable_sharding: bool,
    ) -> Result<Self> {
        let state_kv_metadata_db_path =
            Self::metadata_db_path(db_paths.state_kv_db_metadata_root_path());

        let state_kv_metadata_db = Arc::new(Self::open_db(
            state_kv_metadata_db_path.clone(),
            STATE_KV_METADATA_DB_NAME,
            &state_kv_db_config,
            readonly,
            enable_sharding,
        )?);

        info!(
            state_kv_metadata_db_path = state_kv_metadata_db_path,
            "Opened state kv metadata db!"
        );

        let mut shard_id: usize = 0;
        let state_kv_db_shards = {
            arr![{
                let shard_root_path = db_paths.state_kv_db_shard_root_path(shard_id as u8);
                let db = Self::open_shard(shard_root_path, shard_id as u8, &state_kv_db_config, readonly, enable_sharding)?;
                shard_id += 1;
                Arc::new(db)
            }; 16]
        };

        let state_kv_db = Self {
            state_kv_metadata_db,
            state_kv_db_shards,
            enabled_sharding: true,
        };

        if let Some(overall_kv_commit_progress) = get_state_kv_commit_progress(&state_kv_db)? {
            truncate_state_kv_db_shards(&state_kv_db, overall_kv_commit_progress)?;
        }

        Ok(state_kv_db)
    }

    pub(crate) fn commit(
        &self,
        version: Version,
        state_kv_metadata_batch: SchemaBatch,
        sharded_state_kv_batches: [SchemaBatch; NUM_STATE_SHARDS],
    ) -> Result<()> {
        let _timer = OTHER_TIMERS_SECONDS
            .with_label_values(&["state_kv_db__commit"])
            .start_timer();
        {
            let _timer = OTHER_TIMERS_SECONDS
                .with_label_values(&["state_kv_db__commit_shards"])
                .start_timer();
            THREAD_MANAGER.get_io_pool().scope(|s| {
                let mut batches = sharded_state_kv_batches.into_iter();
                for shard_id in 0..NUM_STATE_SHARDS {
                    let state_kv_batch = batches
                        .next()
                        .expect("Not sufficient number of sharded state kv batches");
                    s.spawn(move |_| {
                        // TODO(grao): Consider propagating the error instead of panic, if necessary.
                        self.commit_single_shard(version, shard_id as u8, state_kv_batch)
                            .unwrap_or_else(|err| {
                                panic!("Failed to commit shard {shard_id}: {err}.")
                            });
                    });
                }
            });
        }

        {
            let _timer = OTHER_TIMERS_SECONDS
                .with_label_values(&["state_kv_db__commit_metadata"])
                .start_timer();
            self.state_kv_metadata_db
                .write_schemas(state_kv_metadata_batch)?;
        }

        self.write_progress(version)
    }

    pub(crate) fn write_progress(&self, version: Version) -> Result<()> {
        self.state_kv_metadata_db.put::<DbMetadataSchema>(
            &DbMetadataKey::StateKvCommitProgress,
            &DbMetadataValue::Version(version),
        )
    }

    pub(crate) fn write_pruner_progress(&self, version: Version) -> Result<()> {
        self.state_kv_metadata_db.put::<DbMetadataSchema>(
            &DbMetadataKey::StateKvPrunerProgress,
            &DbMetadataValue::Version(version),
        )
    }

    pub(crate) fn create_checkpoint(
        db_root_path: impl AsRef<Path>,
        cp_root_path: impl AsRef<Path>,
    ) -> Result<()> {
        // TODO(grao): Support path override here.
        let state_kv_db = Self::open(
            &StorageDirPaths::from_path(db_root_path),
            RocksdbConfig::default(),
            false,
            true,
        )?;
        let cp_state_kv_db_path = cp_root_path.as_ref().join(STATE_KV_DB_FOLDER_NAME);

        info!("Creating state_kv_db checkpoint at: {cp_state_kv_db_path:?}");

        std::fs::remove_dir_all(&cp_state_kv_db_path).unwrap_or(());
        std::fs::create_dir_all(&cp_state_kv_db_path).unwrap_or(());

        state_kv_db
            .metadata_db()
            .create_checkpoint(Self::metadata_db_path(cp_root_path.as_ref()))?;

        for shard_id in 0..NUM_STATE_SHARDS {
            state_kv_db
                .db_shard(shard_id as u8)
                .create_checkpoint(Self::db_shard_path(cp_root_path.as_ref(), shard_id as u8))?;
        }

        Ok(())
    }

    pub(crate) fn metadata_db(&self) -> &DB {
        &self.state_kv_metadata_db
    }

    pub(crate) fn db_shard(&self, shard_id: u8) -> &DB {
        &self.state_kv_db_shards[shard_id as usize]
    }

    pub(crate) fn db_shard_arc(&self, shard_id: u8) -> Arc<DB> {
        Arc::clone(&self.state_kv_db_shards[shard_id as usize])
    }

    pub(crate) fn enabled_sharding(&self) -> bool {
        self.enabled_sharding
    }

    pub(crate) fn num_shards(&self) -> u8 {
        NUM_STATE_SHARDS as u8
    }

    pub(crate) fn hack_num_real_shards(&self) -> usize {
        if self.enabled_sharding {
            NUM_STATE_SHARDS
        } else {
            1
        }
    }

    pub(crate) fn commit_single_shard(
        &self,
        version: Version,
        shard_id: u8,
        batch: SchemaBatch,
    ) -> Result<()> {
        batch.put::<DbMetadataSchema>(
            &DbMetadataKey::StateKvShardCommitProgress(shard_id as usize),
            &DbMetadataValue::Version(version),
        )?;
        self.state_kv_db_shards[shard_id as usize].write_schemas(batch)
    }

    fn open_shard<P: AsRef<Path>>(
        db_root_path: P,
        shard_id: u8,
        state_kv_db_config: &RocksdbConfig,
        readonly: bool,
        enable_sharding: bool,
    ) -> Result<DB> {
        let db_name = format!("state_kv_db_shard_{}", shard_id);
        Self::open_db(
            Self::db_shard_path(db_root_path, shard_id),
            &db_name,
            state_kv_db_config,
            readonly,
            enable_sharding,
        )
    }

    fn open_db(
        path: PathBuf,
        name: &str,
        state_kv_db_config: &RocksdbConfig,
        readonly: bool,
        enable_sharding: bool,
    ) -> Result<DB> {
        Ok(if readonly {
            DB::open_cf_readonly(
                &gen_rocksdb_options(state_kv_db_config, true),
                path,
                name,
                gen_state_kv_cfds(state_kv_db_config, enable_sharding),
            )?
        } else {
            DB::open_cf(
                &gen_rocksdb_options(state_kv_db_config, false),
                path,
                name,
                gen_state_kv_cfds(state_kv_db_config, enable_sharding),
            )?
        })
    }

    fn db_shard_path<P: AsRef<Path>>(db_root_path: P, shard_id: u8) -> PathBuf {
        let shard_sub_path = format!("shard_{}", shard_id);
        db_root_path
            .as_ref()
            .join(STATE_KV_DB_FOLDER_NAME)
            .join(Path::new(&shard_sub_path))
    }

    fn metadata_db_path<P: AsRef<Path>>(db_root_path: P) -> PathBuf {
        db_root_path
            .as_ref()
            .join(STATE_KV_DB_FOLDER_NAME)
            .join("metadata")
    }

    pub(crate) fn get_state_value_with_version_by_version(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> Result<Option<(Version, StateValue)>> {
        let mut read_opts = ReadOptions::default();

        // We want `None` if the state_key changes in iteration.
        read_opts.set_prefix_same_as_start(true);
        if !self.enabled_sharding() {
            let mut iter = self
                .db_shard(state_key.get_shard_id())
                .iter_with_opts::<StateValueSchema>(read_opts)?;
            iter.seek(&(state_key.clone(), version))?;
            Ok(iter
                .next()
                .transpose()?
                .and_then(|((_, version), value_opt)| value_opt.map(|value| (version, value))))
        } else {
            let mut iter = self
                .db_shard(state_key.get_shard_id())
                .iter_with_opts::<StateValueByKeyHashSchema>(read_opts)?;
            iter.seek(&(state_key.hash(), version))?;
            Ok(iter
                .next()
                .transpose()?
                .and_then(|((_, version), value_opt)| value_opt.map(|value| (version, value))))
        }
    }
}

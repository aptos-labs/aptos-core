// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{
    db_options::{gen_hot_state_kv_shard_cfds, gen_state_kv_shard_cfds},
    metrics::OTHER_TIMERS_SECONDS,
    schema::{
        db_metadata::{DbMetadataKey, DbMetadataSchema, DbMetadataValue},
        state_value::StateValueSchema,
        state_value_by_key_hash::StateValueByKeyHashSchema,
    },
    utils::{
        truncation_helper::{get_state_kv_commit_progress, truncate_state_kv_db_shards},
        ShardedStateKvSchemaBatch,
    },
};
use aptos_config::config::{RocksdbConfig, RocksdbConfigs, StorageDirPaths};
use aptos_crypto::hash::CryptoHash;
use aptos_experimental_runtimes::thread_manager::THREAD_MANAGER;
use aptos_logger::prelude::info;
use aptos_metrics_core::TimerHelper;
use aptos_rocksdb_options::gen_rocksdb_options;
use aptos_schemadb::{
    batch::{SchemaBatch, WriteBatch},
    ReadOptions, DB,
};
use aptos_storage_interface::{state_store::NUM_STATE_SHARDS, Result};
use aptos_types::{
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::Version,
};
use arr_macro::arr;
use itertools::Itertools;
use rayon::prelude::*;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

pub const STATE_KV_DB_FOLDER_NAME: &str = "state_kv_db";
pub const STATE_KV_METADATA_DB_NAME: &str = "state_kv_metadata_db";

pub struct StateKvDb {
    state_kv_metadata_db: Arc<DB>,
    state_kv_db_shards: [Arc<DB>; NUM_STATE_SHARDS],
    // TODO(HotState): no separate metadata db for hot state for now.
    #[allow(dead_code)] // TODO(HotState): can remove later.
    hot_state_kv_db_shards: Option<[Arc<DB>; NUM_STATE_SHARDS]>,
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
                hot_state_kv_db_shards: None,
                enabled_sharding: false,
            });
        }

        Self::open_sharded(db_paths, rocksdb_configs.state_kv_db_config, readonly)
    }

    pub(crate) fn open_sharded(
        db_paths: &StorageDirPaths,
        state_kv_db_config: RocksdbConfig,
        readonly: bool,
    ) -> Result<Self> {
        let state_kv_metadata_db_path =
            Self::metadata_db_path(db_paths.state_kv_db_metadata_root_path());

        let state_kv_metadata_db = Arc::new(Self::open_db(
            state_kv_metadata_db_path.clone(),
            STATE_KV_METADATA_DB_NAME,
            &state_kv_db_config,
            readonly,
            /* is_hot = */ false,
        )?);

        info!(
            state_kv_metadata_db_path = state_kv_metadata_db_path,
            "Opened state kv metadata db!"
        );

        let state_kv_db_shards = (0..NUM_STATE_SHARDS)
            .into_par_iter()
            .map(|shard_id| {
                let shard_root_path = db_paths.state_kv_db_shard_root_path(shard_id);
                let db = Self::open_shard(
                    shard_root_path,
                    shard_id,
                    &state_kv_db_config,
                    readonly,
                    /* is_hot = */ false,
                )
                .unwrap_or_else(|e| panic!("Failed to open state kv db shard {shard_id}: {e:?}."));
                Arc::new(db)
            })
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let hot_state_kv_db_shards = if readonly {
            // TODO(HotState): do not open it in readonly mode yet, until we have this DB
            // everywhere.
            None
        } else {
            Some(
                (0..NUM_STATE_SHARDS)
                    .into_par_iter()
                    .map(|shard_id| {
                        let shard_root_path = db_paths.hot_state_kv_db_shard_root_path(shard_id);
                        let db = Self::open_shard(
                            shard_root_path,
                            shard_id,
                            &state_kv_db_config,
                            readonly,
                            /* is_hot = */ true,
                        )
                        .unwrap_or_else(|e| {
                            panic!("Failed to open hot state kv db shard {shard_id}: {e:?}.")
                        });
                        Arc::new(db)
                    })
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap(),
            )
        };

        let state_kv_db = Self {
            state_kv_metadata_db,
            state_kv_db_shards,
            hot_state_kv_db_shards,
            enabled_sharding: true,
        };

        if !readonly {
            if let Some(overall_kv_commit_progress) = get_state_kv_commit_progress(&state_kv_db)? {
                truncate_state_kv_db_shards(&state_kv_db, overall_kv_commit_progress)?;
            }
        }

        Ok(state_kv_db)
    }

    pub(crate) fn new_sharded_native_batches(&self) -> ShardedStateKvSchemaBatch {
        (0..NUM_STATE_SHARDS)
            .map(|shard_id| self.db_shard(shard_id).new_native_batch())
            .collect_vec()
            .try_into()
            .expect("known to be 16 shards")
    }

    pub(crate) fn commit(
        &self,
        version: Version,
        state_kv_metadata_batch: Option<SchemaBatch>,
        sharded_state_kv_batches: ShardedStateKvSchemaBatch,
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
                        self.commit_single_shard(version, shard_id, state_kv_batch)
                            .unwrap_or_else(|err| {
                                panic!("Failed to commit shard {shard_id}: {err}.")
                            });
                    });
                }
            });
        }
        if let Some(batch) = state_kv_metadata_batch {
            let _timer = OTHER_TIMERS_SECONDS.timer_with(&["state_kv_db__commit_metadata"]);
            self.state_kv_metadata_db.write_schemas(batch)?;
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
        let state_kv_db = Self::open_sharded(
            &StorageDirPaths::from_path(db_root_path),
            RocksdbConfig::default(),
            false,
        )?;
        let cp_state_kv_db_path = cp_root_path.as_ref().join(STATE_KV_DB_FOLDER_NAME);

        info!("Creating state_kv_db checkpoint at: {cp_state_kv_db_path:?}");

        std::fs::remove_dir_all(&cp_state_kv_db_path).unwrap_or(());
        std::fs::create_dir_all(&cp_state_kv_db_path).unwrap_or(());

        state_kv_db
            .metadata_db()
            .create_checkpoint(Self::metadata_db_path(cp_root_path.as_ref()))?;

        // TODO(HotState): should handle hot state as well.
        for shard_id in 0..NUM_STATE_SHARDS {
            state_kv_db
                .db_shard(shard_id)
                .create_checkpoint(Self::db_shard_path(
                    cp_root_path.as_ref(),
                    shard_id,
                    /* is_hot = */ false,
                ))?;
        }

        Ok(())
    }

    pub(crate) fn metadata_db(&self) -> &DB {
        &self.state_kv_metadata_db
    }

    pub(crate) fn metadata_db_arc(&self) -> Arc<DB> {
        Arc::clone(&self.state_kv_metadata_db)
    }

    pub(crate) fn db_shard(&self, shard_id: usize) -> &DB {
        &self.state_kv_db_shards[shard_id]
    }

    pub(crate) fn db_shard_arc(&self, shard_id: usize) -> Arc<DB> {
        Arc::clone(&self.state_kv_db_shards[shard_id])
    }

    pub(crate) fn enabled_sharding(&self) -> bool {
        self.enabled_sharding
    }

    pub(crate) fn num_shards(&self) -> usize {
        NUM_STATE_SHARDS
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
        shard_id: usize,
        mut batch: impl WriteBatch,
    ) -> Result<()> {
        batch.put::<DbMetadataSchema>(
            &DbMetadataKey::StateKvShardCommitProgress(shard_id),
            &DbMetadataValue::Version(version),
        )?;
        self.state_kv_db_shards[shard_id].write_schemas(batch)
    }

    fn open_shard<P: AsRef<Path>>(
        db_root_path: P,
        shard_id: usize,
        state_kv_db_config: &RocksdbConfig,
        readonly: bool,
        is_hot: bool,
    ) -> Result<DB> {
        let db_name = if is_hot {
            format!("hot_state_kv_db_shard_{}", shard_id)
        } else {
            format!("state_kv_db_shard_{}", shard_id)
        };
        Self::open_db(
            Self::db_shard_path(db_root_path, shard_id, is_hot),
            &db_name,
            state_kv_db_config,
            readonly,
            is_hot,
        )
    }

    fn open_db(
        path: PathBuf,
        name: &str,
        state_kv_db_config: &RocksdbConfig,
        readonly: bool,
        is_hot: bool,
    ) -> Result<DB> {
        let open_func = if readonly {
            DB::open_cf_readonly
        } else {
            DB::open_cf
        };
        let rocksdb_opts = gen_rocksdb_options(state_kv_db_config, readonly);
        let cfds = if is_hot {
            gen_hot_state_kv_shard_cfds
        } else {
            gen_state_kv_shard_cfds
        }(state_kv_db_config);

        open_func(&rocksdb_opts, path, name, cfds)
    }

    fn db_shard_path<P: AsRef<Path>>(db_root_path: P, shard_id: usize, is_hot: bool) -> PathBuf {
        let shard_sub_path = format!(
            "{}_{}",
            if is_hot { "hot_shard" } else { "shard" },
            shard_id
        );
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

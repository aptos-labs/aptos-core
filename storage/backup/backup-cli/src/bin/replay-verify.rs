// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_config::config::{
    DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD, NO_OP_STORAGE_PRUNER_CONFIG, TARGET_SNAPSHOT_SIZE,
};
use aptos_logger::{prelude::*, Level, Logger};
use aptos_types::transaction::Version;
use aptosdb::{AptosDB, GetRestoreHandler};
use backup_cli::{
    coordinators::replay_verify::ReplayVerifyCoordinator,
    metadata::cache::MetadataCacheOpt,
    storage::StorageOpt,
    utils::{ConcurrentDownloadsOpt, RocksdbOpt, TrustedWaypointOpt},
};
use clap::Parser;
use std::{path::PathBuf, sync::Arc};

#[derive(Parser)]
struct Opt {
    #[clap(flatten)]
    metadata_cache_opt: MetadataCacheOpt,
    #[clap(flatten)]
    trusted_waypoints_opt: TrustedWaypointOpt,
    #[clap(subcommand)]
    storage: StorageOpt,
    #[clap(flatten)]
    concurrent_downloads: ConcurrentDownloadsOpt,
    #[clap(long = "target-db-dir", parse(from_os_str))]
    pub db_dir: PathBuf,
    #[clap(flatten)]
    pub rocksdb_opt: RocksdbOpt,
    #[clap(
        long,
        help = "The first transaction version required to be replayed and verified. [Defaults to 0]"
    )]
    start_version: Option<Version>,
    #[clap(
        long,
        help = "The last transaction version required to be replayed and verified (if present \
        in the backup). [Defaults to the latest version available] "
    )]
    end_version: Option<Version>,
}

#[tokio::main]
async fn main() -> Result<()> {
    main_impl().await.map_err(|e| {
        error!("main_impl() failed: {}", e);
        e
    })
}

async fn main_impl() -> Result<()> {
    Logger::new().level(Level::Info).read_env().init();

    let opt = Opt::from_args();
    let restore_handler = Arc::new(AptosDB::open(
        opt.db_dir,
        false,                       /* read_only */
        NO_OP_STORAGE_PRUNER_CONFIG, /* pruner config */
        opt.rocksdb_opt.into(),
        false,
        TARGET_SNAPSHOT_SIZE,
        DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
    )?)
    .get_restore_handler();
    ReplayVerifyCoordinator::new(
        opt.storage.init_storage().await?,
        opt.metadata_cache_opt,
        opt.trusted_waypoints_opt,
        opt.concurrent_downloads.get(),
        restore_handler,
        opt.start_version.unwrap_or(0),
        opt.end_version.unwrap_or(Version::MAX),
    )?
    .run()
    .await
}

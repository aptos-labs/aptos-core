use std::{path::PathBuf, sync::Arc};

use clap::Parser;

use anyhow::Result;

use crate::commands::backup::{
    coordinators::restore::{RestoreCoordinator, RestoreCoordinatorOpt},
    metadata::cache::MetadataCacheOpt,
    storage::command_adapter::{config::CommandAdapterConfig, CommandAdapter},
    utils::{ConcurrentDownloadsOpt, GlobalRestoreOpt, ReplayConcurrencyLevelOpt, RocksdbOpt},
};
///
///
/// Enables users to load from a backup to catch their node's DB up to a known state.
#[derive(Parser, Clone)]
#[clap(about = "Bootstrap AptosDB from a backup")]
pub struct Bootstrapper {
    /// Config file for the source backup
    ///
    /// This file configures if we should use local files or cloud storage, and how to access
    /// the backup.
    #[clap(long, parse(from_os_str))]
    config_path: PathBuf,

    /// Target database directory
    ///
    /// The directory to create the AptosDB with snapshots and transactions from the backup.
    /// The data folder can later be used to start an Aptos node. e.g. /opt/aptos/data/db
    #[clap(long = "target-db-dir", parse(from_os_str))]
    pub db_dir: PathBuf,

    #[clap(flatten)]
    pub metadata_cache_opt: MetadataCacheOpt,

    #[clap(flatten)]
    pub concurrent_downloads: ConcurrentDownloadsOpt,

    #[clap(flatten)]
    pub replay_concurrency_level: ReplayConcurrencyLevelOpt,
}

impl Bootstrapper {
    pub async fn process(&self) -> Result<()> {
        let opt = RestoreCoordinatorOpt {
            metadata_cache_opt: self.metadata_cache_opt.clone(),
            replay_all: false,
            ledger_history_start_version: None,
            skip_epoch_endings: false,
        };
        let global_opt = GlobalRestoreOpt {
            dry_run: false,
            db_dir: Some(self.db_dir.clone()),
            target_version: None,
            trusted_waypoints: Default::default(),
            rocksdb_opt: RocksdbOpt::default(),
            concurrent_downloads: self.concurrent_downloads,
            replay_concurrency_level: self.replay_concurrency_level,
        }
        .try_into()?;
        let storage = Arc::new(CommandAdapter::new(
            CommandAdapterConfig::load_from_file(&self.config_path).await?,
        ));

        tokio::task::spawn_blocking(|| {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(RestoreCoordinator::new(opt, global_opt, storage).run())
        })
        .await
        .unwrap()?;

        Ok(())
    }
}

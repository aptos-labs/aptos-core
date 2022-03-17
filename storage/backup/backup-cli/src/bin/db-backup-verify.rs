// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_logger::{prelude::*, Level, Logger};
use aptos_secure_push_metrics::MetricsPusher;
use backup_cli::{
    coordinators::verify::VerifyCoordinator,
    metadata::cache::MetadataCacheOpt,
    storage::StorageOpt,
    utils::{ConcurrentDownloadsOpt, TrustedWaypointOpt},
};
use structopt::StructOpt;

#[derive(StructOpt)]
struct Opt {
    #[structopt(flatten)]
    metadata_cache_opt: MetadataCacheOpt,
    #[structopt(flatten)]
    trusted_waypoints_opt: TrustedWaypointOpt,
    #[structopt(subcommand)]
    storage: StorageOpt,
    #[structopt(flatten)]
    concurrent_downloads: ConcurrentDownloadsOpt,
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
    let _mp = MetricsPusher::start();

    let opt = Opt::from_args();
    VerifyCoordinator::new(
        opt.storage.init_storage().await?,
        opt.metadata_cache_opt,
        opt.trusted_waypoints_opt,
        opt.concurrent_downloads.get(),
    )?
    .run()
    .await
}

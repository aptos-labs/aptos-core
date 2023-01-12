use anyhow::Result;
use clap::Parser;

use crate::commands::backup::{
    coordinators::verify::VerifyCoordinator,
    metadata::cache::MetadataCacheOpt,
    storage::StorageOpt,
    utils::{ConcurrentDownloadsOpt, TrustedWaypointOpt},
};

#[derive(Parser, Clone)]
#[clap(about = "Verify aptos backup succeed or failed")]
pub struct BackupVerified {
    #[clap(flatten)]
    metadata_cache_opt: MetadataCacheOpt,
    #[clap(flatten)]
    trusted_waypoints_opt: TrustedWaypointOpt,
    #[clap(subcommand)]
    storage: StorageOpt,
    #[clap(flatten)]
    concurrent_downloads: ConcurrentDownloadsOpt,
}

impl BackupVerified {
    pub async fn process(&self) -> Result<()> {
        VerifyCoordinator::new(
            self.storage.clone().init_storage().await?,
            self.metadata_cache_opt.clone(),
            self.trusted_waypoints_opt.clone(),
            self.concurrent_downloads.get(),
        )?
        .run()
        .await
    }
}

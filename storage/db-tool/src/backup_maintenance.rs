// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use anyhow::Result;
use aptos_backup_cli::{
    coordinators::backup::BackupCompactor, metadata::cache::MetadataCacheOpt,
    storage::DBToolStorageOpt, utils::ConcurrentDownloadsOpt,
};
use clap::{Parser, Subcommand};

/// Support compacting and cleaning obsolete metadata files
#[derive(Subcommand)]
pub enum Command {
    #[clap(about = "Compact metdata files")]
    Compact(CompactionOpt),
    #[clap(about = "Cleanup the backup metadata files")]
    Cleanup(CleanupOpt),
}

#[derive(Parser)]
pub struct CompactionOpt {
    /// Specify how many epoch files to be merged in one compacted epoch ending metadata file
    #[clap(long, default_value = "1")]
    pub epoch_ending_file_compact_factor: usize,
    /// Specify how many state snapshot files to be merged in one compacted state snapshot metadata file
    #[clap(long, default_value = "1")]
    pub state_snapshot_file_compact_factor: usize,
    /// Specify how many transaction files to be merged in one transaction metadata file
    #[clap(long, default_value = "1")]
    pub transaction_file_compact_factor: usize,
    #[clap(flatten)]
    pub metadata_cache_opt: MetadataCacheOpt,
    #[clap(flatten)]
    pub storage: DBToolStorageOpt,
    #[clap(flatten)]
    concurrent_downloads: ConcurrentDownloadsOpt,
}

#[derive(Parser)]
pub struct CleanupOpt {
    #[clap(flatten)]
    pub storage: DBToolStorageOpt,
}

impl Command {
    pub async fn run(self) -> Result<()> {
        match self {
            Command::Compact(opt) => {
                let compactor = BackupCompactor::new(
                    opt.epoch_ending_file_compact_factor,
                    opt.state_snapshot_file_compact_factor,
                    opt.transaction_file_compact_factor,
                    opt.metadata_cache_opt,
                    opt.storage.init_storage().await?,
                    opt.concurrent_downloads.get(),
                );
                compactor.run().await?
            },
            Command::Cleanup(_) => {
                // TODO: add cleanup logic for removing obsolete metadata files
            },
        }
        Ok(())
    }
}

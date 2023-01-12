use anyhow::Result;
use clap::Subcommand;

use crate::commands::backup::{
    backup_types::{
        epoch_ending::restore::{EpochEndingRestoreController, EpochEndingRestoreOpt},
        state_snapshot::restore::{StateSnapshotRestoreController, StateSnapshotRestoreOpt},
        transaction::restore::{TransactionRestoreController, TransactionRestoreOpt},
    },
    coordinators::restore::{RestoreCoordinator, RestoreCoordinatorOpt},
    storage::StorageOpt,
    utils::GlobalRestoreOptions,
};

#[derive(Subcommand)]
pub enum Restore {
    EpochEnding {
        #[clap(flatten)]
        opt: EpochEndingRestoreOpt,
        #[clap(subcommand)]
        storage: StorageOpt,
    },
    StateSnapshot {
        #[clap(flatten)]
        opt: StateSnapshotRestoreOpt,
        #[clap(subcommand)]
        storage: StorageOpt,
    },
    Transaction {
        #[clap(flatten)]
        opt: TransactionRestoreOpt,
        #[clap(subcommand)]
        storage: StorageOpt,
    },
    Auto {
        #[clap(flatten)]
        opt: RestoreCoordinatorOpt,
        #[clap(subcommand)]
        storage: StorageOpt,
    },
}

impl Restore {
    pub async fn process(&self, global_opt: GlobalRestoreOptions) -> Result<()> {
        match self {
            Restore::EpochEnding { opt, storage } => {
                EpochEndingRestoreController::new(
                    opt.clone(),
                    global_opt,
                    storage.clone().init_storage().await?,
                )
                .run(None)
                .await?;
            },
            Restore::StateSnapshot { opt, storage } => {
                StateSnapshotRestoreController::new(
                    opt.clone(),
                    global_opt,
                    storage.clone().init_storage().await?,
                    None, /* epoch_history */
                )
                .run()
                .await?;
            },
            Restore::Transaction { opt, storage } => {
                TransactionRestoreController::new(
                    opt.clone(),
                    global_opt,
                    storage.clone().init_storage().await?,
                    None, /* epoch_history */
                    vec![],
                )
                .run()
                .await?;
            },
            Restore::Auto { opt, storage } => {
                RestoreCoordinator::new(
                    opt.clone(),
                    global_opt,
                    storage.clone().init_storage().await?,
                )
                .run()
                .await?;
            },
        }
        Ok(())
    }
}

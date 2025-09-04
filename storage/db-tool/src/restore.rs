// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use velor_backup_cli::{
    backup_types::{
        epoch_ending::restore::{EpochEndingRestoreController, EpochEndingRestoreOpt},
        state_snapshot::restore::{StateSnapshotRestoreController, StateSnapshotRestoreOpt},
        transaction::restore::{TransactionRestoreController, TransactionRestoreOpt},
    },
    coordinators::restore::{RestoreCoordinator, RestoreCoordinatorOpt},
    storage::DBToolStorageOpt,
    utils::GlobalRestoreOpt,
};
use velor_executor_types::VerifyExecutionMode;
use clap::{Parser, Subcommand};

/// Restore the database using either a one-time or continuous backup.
#[derive(Subcommand)]
pub enum Command {
    #[clap(about = "run continuously to restore the DB")]
    BootstrapDB(BootstrapDB),
    #[clap(subcommand)]
    Oneoff(Oneoff),
}

#[derive(Parser)]
pub struct BootstrapDB {
    #[clap(flatten)]
    storage: DBToolStorageOpt,
    #[clap(flatten)]
    opt: RestoreCoordinatorOpt,
    #[clap(flatten)]
    global: GlobalRestoreOpt,
}

#[derive(Parser)]
pub enum Oneoff {
    EpochEnding {
        #[clap(flatten)]
        storage: DBToolStorageOpt,
        #[clap(flatten)]
        opt: EpochEndingRestoreOpt,
        #[clap(flatten)]
        global: GlobalRestoreOpt,
    },
    StateSnapshot {
        #[clap(flatten)]
        storage: DBToolStorageOpt,
        #[clap(flatten)]
        opt: StateSnapshotRestoreOpt,
        #[clap(flatten)]
        global: GlobalRestoreOpt,
    },
    Transaction {
        #[clap(flatten)]
        storage: DBToolStorageOpt,
        #[clap(flatten)]
        opt: TransactionRestoreOpt,
        #[clap(flatten)]
        global: GlobalRestoreOpt,
    },
}

impl Command {
    pub async fn run(self) -> Result<()> {
        match self {
            Command::Oneoff(oneoff) => {
                match oneoff {
                    Oneoff::EpochEnding {
                        storage,
                        opt,
                        global,
                    } => {
                        EpochEndingRestoreController::new(
                            opt,
                            global.try_into()?,
                            storage.init_storage().await?,
                        )
                        .run(None)
                        .await?;
                    },
                    Oneoff::StateSnapshot {
                        storage,
                        opt,
                        global,
                    } => {
                        StateSnapshotRestoreController::new(
                            opt,
                            global.try_into()?,
                            storage.init_storage().await?,
                            None, /* epoch_history */
                        )
                        .run()
                        .await?;
                    },
                    Oneoff::Transaction {
                        storage,
                        opt,
                        global,
                    } => {
                        TransactionRestoreController::new(
                            opt,
                            global.try_into()?,
                            storage.init_storage().await?,
                            None, /* epoch_history */
                            VerifyExecutionMode::NoVerify,
                        )
                        .run()
                        .await?;
                    },
                }
            },
            Command::BootstrapDB(bootstrap) => {
                RestoreCoordinator::new(
                    bootstrap.opt,
                    bootstrap.global.try_into()?,
                    bootstrap.storage.init_storage().await?,
                )
                .run()
                .await?;
            },
        }

        Ok(())
    }
}

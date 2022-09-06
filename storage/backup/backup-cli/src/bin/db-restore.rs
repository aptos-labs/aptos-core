// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_logger::{prelude::*, Level, Logger};
use aptos_push_metrics::MetricsPusher;
use backup_cli::{
    backup_types::{
        epoch_ending::restore::{EpochEndingRestoreController, EpochEndingRestoreOpt},
        state_snapshot::restore::{StateSnapshotRestoreController, StateSnapshotRestoreOpt},
        transaction::restore::{TransactionRestoreController, TransactionRestoreOpt},
    },
    coordinators::restore::{RestoreCoordinator, RestoreCoordinatorOpt},
    storage::StorageOpt,
    utils::{GlobalRestoreOpt, GlobalRestoreOptions},
};
use clap::Parser;
use std::convert::TryInto;

#[derive(Parser)]
struct Opt {
    #[clap(flatten)]
    global: GlobalRestoreOpt,

    #[clap(subcommand)]
    restore_type: RestoreType,
}

#[derive(Parser)]
enum RestoreType {
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

#[tokio::main]
async fn main() -> Result<()> {
    main_impl().await.map_err(|e| {
        error!("main_impl() failed: {}", e);
        e
    })
}

async fn main_impl() -> Result<()> {
    Logger::new().level(Level::Info).read_env().init();
    #[allow(deprecated)]
    let _mp = MetricsPusher::start();

    let opt = Opt::from_args();
    let global_opt: GlobalRestoreOptions = opt.global.clone().try_into()?;

    match opt.restore_type {
        RestoreType::EpochEnding { opt, storage } => {
            EpochEndingRestoreController::new(opt, global_opt, storage.init_storage().await?)
                .run(None)
                .await?;
        }
        RestoreType::StateSnapshot { opt, storage } => {
            StateSnapshotRestoreController::new(
                opt,
                global_opt,
                storage.init_storage().await?,
                None, /* epoch_history */
            )
            .run()
            .await?;
        }
        RestoreType::Transaction { opt, storage } => {
            TransactionRestoreController::new(
                opt,
                global_opt,
                storage.init_storage().await?,
                None, /* epoch_history */
            )
            .run()
            .await?;
        }
        RestoreType::Auto { opt, storage } => {
            RestoreCoordinator::new(opt, global_opt, storage.init_storage().await?)
                .run()
                .await?;
        }
    }

    Ok(())
}

// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use anyhow::Result;
use clap::Parser;

use aptos_logger::{prelude::*, Level, Logger};
use aptos_push_metrics::MetricsPusher;
use backup_cli::{
    backup_types::{
        epoch_ending::backup::{EpochEndingBackupController, EpochEndingBackupOpt},
        state_snapshot::backup::{StateSnapshotBackupController, StateSnapshotBackupOpt},
        transaction::backup::{TransactionBackupController, TransactionBackupOpt},
    },
    coordinators::backup::{BackupCoordinator, BackupCoordinatorOpt},
    metadata::{cache, cache::MetadataCacheOpt},
    storage::StorageOpt,
    utils::{
        backup_service_client::{BackupServiceClient, BackupServiceClientOpt},
        ConcurrentDownloadsOpt, GlobalBackupOpt,
    },
};

#[derive(Parser)]
#[clap(about = "Ledger backup tool.")]
enum Command {
    #[clap(subcommand, about = "Manually run one shot commands.")]
    OneShot(OneShotCommand),
    #[clap(
        subcommand,
        about = "Long running process backing up the chain continuously."
    )]
    Coordinator(CoordinatorCommand),
}

#[derive(Parser)]
enum OneShotCommand {
    #[clap(
        subcommand,
        about = "Query the backup service builtin in the local node."
    )]
    Query(OneShotQueryType),
    #[clap(about = "Do a one shot backup of either of the backup types.")]
    Backup(OneShotBackupOpt),
}

#[derive(Parser)]
enum OneShotQueryType {
    #[clap(
        about = "Queries the latest epoch, committed version and synced version of the local \
        node, via the backup service within it."
    )]
    NodeState(OneShotQueryNodeStateOpt),
    #[clap(
        about = "Queries the latest epoch and versions of the existing backups in the storage."
    )]
    BackupStorageState(OneShotQueryBackupStorageStateOpt),
}

#[derive(Parser)]
struct OneShotQueryNodeStateOpt {
    #[clap(flatten)]
    client: BackupServiceClientOpt,
}

#[derive(Parser)]
struct OneShotQueryBackupStorageStateOpt {
    #[clap(flatten)]
    metadata_cache: MetadataCacheOpt,
    #[clap(flatten)]
    concurrent_downloads: ConcurrentDownloadsOpt,
    #[clap(subcommand)]
    storage: StorageOpt,
}

#[derive(Parser)]
struct OneShotBackupOpt {
    #[clap(flatten)]
    global: GlobalBackupOpt,

    #[clap(flatten)]
    client: BackupServiceClientOpt,

    #[clap(subcommand)]
    backup_type: BackupType,
}

#[derive(Parser)]
enum BackupType {
    EpochEnding {
        #[clap(flatten)]
        opt: EpochEndingBackupOpt,
        #[clap(subcommand)]
        storage: StorageOpt,
    },
    StateSnapshot {
        #[clap(flatten)]
        opt: StateSnapshotBackupOpt,
        #[clap(subcommand)]
        storage: StorageOpt,
    },
    Transaction {
        #[clap(flatten)]
        opt: TransactionBackupOpt,
        #[clap(subcommand)]
        storage: StorageOpt,
    },
}

#[derive(Parser)]
enum CoordinatorCommand {
    #[clap(
        about = "Run the backup coordinator which backs up blockchain data continuously off \
    a Aptos Node."
    )]
    Run(CoordinatorRunOpt),
}

#[derive(Parser)]
struct CoordinatorRunOpt {
    #[clap(flatten)]
    global: GlobalBackupOpt,

    #[clap(flatten)]
    client: BackupServiceClientOpt,

    #[clap(flatten)]
    coordinator: BackupCoordinatorOpt,

    #[clap(subcommand)]
    storage: StorageOpt,
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

    let cmd = Command::from_args();
    match cmd {
        Command::OneShot(one_shot_cmd) => match one_shot_cmd {
            OneShotCommand::Query(typ) => match typ {
                OneShotQueryType::NodeState(opt) => {
                    let client = BackupServiceClient::new_with_opt(opt.client);
                    if let Some(db_state) = client.get_db_state().await? {
                        println!("{}", db_state)
                    } else {
                        println!("DB not bootstrapped.")
                    }
                }
                OneShotQueryType::BackupStorageState(opt) => {
                    let view = cache::sync_and_load(
                        &opt.metadata_cache,
                        opt.storage.init_storage().await?,
                        opt.concurrent_downloads.get(),
                    )
                    .await?;
                    println!("{}", view.get_storage_state()?)
                }
            },
            OneShotCommand::Backup(opt) => {
                let client = Arc::new(BackupServiceClient::new_with_opt(opt.client));
                let global_opt = opt.global;

                match opt.backup_type {
                    BackupType::EpochEnding { opt, storage } => {
                        EpochEndingBackupController::new(
                            opt,
                            global_opt,
                            client,
                            storage.init_storage().await?,
                        )
                        .run()
                        .await?;
                    }
                    BackupType::StateSnapshot { opt, storage } => {
                        StateSnapshotBackupController::new(
                            opt,
                            global_opt,
                            client,
                            storage.init_storage().await?,
                        )
                        .run()
                        .await?;
                    }
                    BackupType::Transaction { opt, storage } => {
                        TransactionBackupController::new(
                            opt,
                            global_opt,
                            client,
                            storage.init_storage().await?,
                        )
                        .run()
                        .await?;
                    }
                }
            }
        },
        Command::Coordinator(coordinator_cmd) => match coordinator_cmd {
            CoordinatorCommand::Run(opt) => {
                BackupCoordinator::new(
                    opt.coordinator,
                    opt.global,
                    Arc::new(BackupServiceClient::new_with_opt(opt.client)),
                    opt.storage.init_storage().await?,
                )
                .run()
                .await?;
            }
        },
    }
    Ok(())
}

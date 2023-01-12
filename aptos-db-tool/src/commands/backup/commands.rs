use anyhow::Result;
use std::sync::Arc;

use super::{
    backup_types::{
        epoch_ending::backup::{EpochEndingBackupController, EpochEndingBackupOpt},
        state_snapshot::backup::{StateSnapshotBackupController, StateSnapshotBackupOpt},
        transaction::backup::{TransactionBackupController, TransactionBackupOpt},
    },
    coordinators::backup::{BackupCoordinator, BackupCoordinatorOpt},
    metadata::cache,
    storage::StorageOpt,
    utils::{
        backup_service_client::{BackupServiceClient, BackupServiceClientOpt},
        ConcurrentDownloadsOpt, GlobalBackupOpt,
    },
};
use crate::commands::backup::metadata::cache::MetadataCacheOpt;
use clap::{Parser, Subcommand};

#[derive(Subcommand)]
pub enum Backup {
    #[clap(subcommand, about = "Manually run one shot commands.")]
    OneShot(OneShotCommand),
    #[clap(
        subcommand,
        about = "Long running process backing up the chain continuously."
    )]
    Coordinator(CoordinatorCommand),
}

#[derive(Subcommand)]
pub enum OneShotCommand {
    #[clap(
        subcommand,
        about = "Query the backup service builtin in the local node."
    )]
    Query(OneShotQueryType),
    #[clap(about = "Do a one shot backup of either of the backup types.")]
    Backup(OneShotBackupOpt),
}

#[derive(Parser, Clone)]
pub enum OneShotQueryType {
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

#[derive(Parser, Clone)]
pub struct OneShotQueryNodeStateOpt {
    #[clap(flatten)]
    client: BackupServiceClientOpt,
}

#[derive(Parser, Clone)]
pub struct OneShotQueryBackupStorageStateOpt {
    #[clap(flatten)]
    metadata_cache: MetadataCacheOpt,
    #[clap(flatten)]
    concurrent_downloads: ConcurrentDownloadsOpt,
    #[clap(subcommand)]
    storage: StorageOpt,
}

#[derive(Parser, Clone)]
pub struct OneShotBackupOpt {
    #[clap(flatten)]
    global: GlobalBackupOpt,

    #[clap(flatten)]
    client: BackupServiceClientOpt,

    #[clap(subcommand)]
    backup_type: BackupType,
}

#[derive(Parser, Clone)]
pub enum BackupType {
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
pub enum CoordinatorCommand {
    #[clap(
        about = "Run the backup coordinator which backs up blockchain data continuously off \
    a Aptos Node."
    )]
    Run(CoordinatorRunOpt),
}

#[derive(Parser, Clone)]
pub struct CoordinatorRunOpt {
    #[clap(flatten)]
    global: GlobalBackupOpt,

    #[clap(flatten)]
    client: BackupServiceClientOpt,

    #[clap(flatten)]
    coordinator: BackupCoordinatorOpt,

    #[clap(subcommand)]
    storage: StorageOpt,
}

impl Backup {
    pub async fn process(&self) -> Result<()> {
        match self {
            Backup::OneShot(one_shot_cmd) => match &one_shot_cmd {
                OneShotCommand::Query(typ) => match &typ {
                    OneShotQueryType::NodeState(opt) => {
                        let client = BackupServiceClient::new_with_opt(opt.clone().client);
                        if let Some(db_state) = client.get_db_state().await? {
                            println!("{}", db_state)
                        } else {
                            println!("DB not bootstrapped.")
                        }
                    },
                    OneShotQueryType::BackupStorageState(opt) => {
                        let view = cache::sync_and_load(
                            &opt.metadata_cache,
                            opt.clone().storage.init_storage().await?,
                            opt.concurrent_downloads.get(),
                        )
                        .await?;
                        println!("{}", view.get_storage_state()?)
                    },
                },
                OneShotCommand::Backup(opt) => {
                    let client = Arc::new(BackupServiceClient::new_with_opt(opt.clone().client));
                    let global_opt = opt.clone().global;

                    match &opt.clone().backup_type {
                        BackupType::EpochEnding { opt, storage } => {
                            EpochEndingBackupController::new(
                                opt.clone(),
                                global_opt,
                                client,
                                storage.clone().init_storage().await?,
                            )
                            .run()
                            .await?;
                        },
                        BackupType::StateSnapshot { opt, storage } => {
                            StateSnapshotBackupController::new(
                                opt.clone(),
                                global_opt,
                                client,
                                storage.clone().init_storage().await?,
                            )
                            .run()
                            .await?;
                        },
                        BackupType::Transaction { opt, storage } => {
                            TransactionBackupController::new(
                                opt.clone(),
                                global_opt,
                                client,
                                storage.clone().init_storage().await?,
                            )
                            .run()
                            .await?;
                        },
                    }
                },
            },
            Backup::Coordinator(coordinator_cmd) => match &coordinator_cmd {
                CoordinatorCommand::Run(opt) => {
                    BackupCoordinator::new(
                        opt.clone().coordinator,
                        opt.clone().global,
                        Arc::new(BackupServiceClient::new_with_opt(opt.clone().client)),
                        opt.clone().storage.init_storage().await?,
                    )
                    .run()
                    .await?;
                },
            },
        }
        Ok(())
    }
}

// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::utils::parse_maxable_u64;
use anyhow::Result;
use aptos_backup_cli::{
    backup_types::{
        epoch_ending::backup::{EpochEndingBackupController, EpochEndingBackupOpt},
        state_snapshot::backup::{StateSnapshotBackupController, StateSnapshotBackupOpt},
        transaction::backup::{TransactionBackupController, TransactionBackupOpt},
    },
    coordinators::{
        backup::{BackupCoordinator, BackupCoordinatorOpt},
        verify::{EventModuleFilter, VerifyCoordinator},
    },
    metadata::{cache, cache::MetadataCacheOpt},
    storage::DBToolStorageOpt,
    utils::{
        backup_service_client::{BackupServiceClient, BackupServiceClientOpt},
        ConcurrentDownloadsOpt, GlobalBackupOpt, TrustedWaypointOpt,
    },
};
use aptos_types::transaction::Version;
use clap::{Parser, Subcommand};
use std::{path::PathBuf, sync::Arc};

/// Supports one-time and continuous backup, including querying the backup service and verifying the backup.
#[derive(Subcommand)]
pub enum Command {
    #[clap(about = "Manually run one shot commands.")]
    Oneoff(OneoffBackupOpt),
    #[clap(
        about = "Run the backup coordinator which backs up blockchain data continuously off \
    a Aptos Node."
    )]
    Continuously(CoordinatorRunOpt),
    #[clap(
        subcommand,
        about = "Query the backup service builtin in the local node."
    )]
    Query(OneShotQueryType),
    #[clap(about = "verify the backup through restoring with the backup files")]
    Verify(VerifyOpt),
}

#[derive(Parser)]
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

#[derive(Parser)]
pub struct OneShotQueryNodeStateOpt {
    #[clap(flatten)]
    client: BackupServiceClientOpt,
}

#[derive(Parser)]
pub struct OneShotQueryBackupStorageStateOpt {
    #[clap(flatten)]
    metadata_cache: MetadataCacheOpt,
    #[clap(flatten)]
    concurrent_downloads: ConcurrentDownloadsOpt,
    #[clap(flatten)]
    storage: DBToolStorageOpt,
}

#[derive(Parser)]
pub struct OneoffBackupOpt {
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
        #[clap[flatten]]
        storage: DBToolStorageOpt,
    },
    StateSnapshot {
        #[clap(flatten)]
        opt: StateSnapshotBackupOpt,
        #[clap[flatten]]
        storage: DBToolStorageOpt,
    },
    Transaction {
        #[clap(flatten)]
        opt: TransactionBackupOpt,
        #[clap[flatten]]
        storage: DBToolStorageOpt,
    },
}

#[derive(Parser)]
pub struct CoordinatorRunOpt {
    #[clap(flatten)]
    global: GlobalBackupOpt,

    #[clap(flatten)]
    client: BackupServiceClientOpt,

    #[clap(flatten)]
    coordinator: BackupCoordinatorOpt,

    #[clap[flatten]]
    storage: DBToolStorageOpt,
}

#[derive(Parser)]
pub struct VerifyOpt {
    #[clap(flatten)]
    metadata_cache_opt: MetadataCacheOpt,
    #[clap(flatten)]
    trusted_waypoints_opt: TrustedWaypointOpt,
    #[clap(flatten)]
    storage: DBToolStorageOpt,
    #[clap(flatten)]
    concurrent_downloads: ConcurrentDownloadsOpt,
    #[clap(
        long,
        value_parser = parse_maxable_u64,
        help = "The first transaction version required to be verified. Pass \"max\" to skip \
        transaction verification. [Defaults to 0]"
    )]
    start_version: Option<Version>,
    #[clap(
        long,
        help = "The last transaction version required to be verified (if present \
        in the backup). [Defaults to the latest version available]"
    )]
    end_version: Option<Version>,
    #[clap(
        long,
        help = "Verify the last state snapshot strictly before this version. Pass 0 to disable \
        state snapshot verification. [Defaults to the latest snapshot]"
    )]
    state_snapshot_before_version: Option<Version>,
    #[clap(long, help = "Skip verifying epoch ending info.")]
    skip_epoch_endings: bool,
    #[clap(
        long,
        help = "Optionally, while verifying a snapshot, run module validation."
    )]
    validate_modules: bool,
    #[clap(
        long,
        value_parser,
        help = "Optionally, while verifying transactions, output analysis files to specified dir."
    )]
    output_transaction_analysis: Option<PathBuf>,
    #[clap(
        long,
        help = "Count ContractEvents whose struct type lives in the given Move module, e.g. \
        `0x1::confidential_asset`. Reported (and asserted, if --assert-event-count is set) \
        only over the cryptographically verified txn range."
    )]
    count_events_in_module: Option<EventModuleFilter>,
    #[clap(
        long,
        help = "If set together with --count-events-in-module, exit non-zero unless the \
        counted total exactly matches this value."
    )]
    assert_event_count: Option<usize>,
}

impl Command {
    pub async fn run(self) -> Result<()> {
        match self {
            Command::Oneoff(opt) => {
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
                    },
                    BackupType::StateSnapshot { opt, storage } => {
                        StateSnapshotBackupController::new(
                            opt,
                            global_opt,
                            client,
                            storage.init_storage().await?,
                        )
                        .run()
                        .await?;
                    },
                    BackupType::Transaction { opt, storage } => {
                        TransactionBackupController::new(
                            opt,
                            global_opt,
                            client,
                            storage.init_storage().await?,
                        )
                        .run()
                        .await?;
                    },
                }
            },
            Command::Continuously(opt) => {
                BackupCoordinator::new(
                    opt.coordinator,
                    opt.global,
                    Arc::new(BackupServiceClient::new_with_opt(opt.client)),
                    opt.storage.init_storage().await?,
                )
                .run()
                .await?;
            },
            Command::Query(typ) => match typ {
                OneShotQueryType::NodeState(opt) => {
                    let client = BackupServiceClient::new_with_opt(opt.client);
                    if let Some(db_state) = client.get_db_state().await? {
                        println!("{}", db_state)
                    } else {
                        println!("DB not bootstrapped.")
                    }
                },
                OneShotQueryType::BackupStorageState(opt) => {
                    let view = cache::sync_and_load(
                        &opt.metadata_cache,
                        opt.storage.init_storage().await?,
                        opt.concurrent_downloads.get(),
                    )
                    .await?;
                    println!("{}", view.get_storage_state()?)
                },
            },
            Command::Verify(opt) => {
                let assert_count = opt.assert_event_count;
                let has_filter = opt.count_events_in_module.is_some();
                if assert_count.is_some() && !has_filter {
                    anyhow::bail!(
                        "--assert-event-count requires --count-events-in-module to be set"
                    );
                }
                let coordinator = VerifyCoordinator::new(
                    opt.storage.init_storage().await?,
                    opt.metadata_cache_opt,
                    opt.trusted_waypoints_opt,
                    opt.concurrent_downloads.get(),
                    opt.start_version.unwrap_or(0),
                    opt.end_version.unwrap_or(Version::MAX),
                    opt.state_snapshot_before_version.unwrap_or(Version::MAX),
                    opt.skip_epoch_endings,
                    opt.validate_modules,
                    opt.output_transaction_analysis,
                    opt.count_events_in_module,
                )?;
                let count_handle = coordinator.event_match_count_handle();
                coordinator.run().await?;
                if let Some(expected) = assert_count {
                    let actual = count_handle.load(std::sync::atomic::Ordering::Relaxed);
                    anyhow::ensure!(
                        actual == expected,
                        "Event count mismatch: counted {} events matching the filter, \
                         expected {}.",
                        actual,
                        expected,
                    );
                }
            },
        }
        Ok(())
    }
}

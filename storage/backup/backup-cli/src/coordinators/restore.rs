// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    backup_types::{
        epoch_ending::restore::EpochHistoryRestoreController,
        state_snapshot::restore::{StateSnapshotRestoreController, StateSnapshotRestoreOpt},
        transaction::restore::TransactionRestoreBatchController,
    },
    metadata,
    metadata::{cache::MetadataCacheOpt, TransactionBackupMeta},
    metrics::restore::{
        COORDINATOR_FAIL_TS, COORDINATOR_START_TS, COORDINATOR_SUCC_TS, COORDINATOR_TARGET_VERSION,
    },
    storage::BackupStorage,
    utils::{unix_timestamp_sec, GlobalRestoreOptions},
};
use anyhow::{anyhow, bail, Result};
use aptos_logger::prelude::*;
use aptos_types::transaction::Version;
use clap::Parser;
use std::sync::Arc;

#[derive(Parser)]
pub struct RestoreCoordinatorOpt {
    #[clap(flatten)]
    pub metadata_cache_opt: MetadataCacheOpt,
    #[clap(
        long,
        help = "Replay all transactions, don't try to use a state snapshot."
    )]
    pub replay_all: bool,
    #[clap(
        long,
        help = "[default to only start ledger history after selected state snapshot] \
        Ignore restoring the ledger history (transactions and events) before this version \
        if possible, set 0 for full ledger history."
    )]
    pub ledger_history_start_version: Option<Version>,
    #[clap(long, help = "Skip restoring epoch ending info, used for debugging.")]
    pub skip_epoch_endings: bool,
}

pub struct RestoreCoordinator {
    storage: Arc<dyn BackupStorage>,
    global_opt: GlobalRestoreOptions,
    metadata_cache_opt: MetadataCacheOpt,
    replay_all: bool,
    ledger_history_start_version: Option<Version>,
    skip_epoch_endings: bool,
}

impl RestoreCoordinator {
    pub fn new(
        opt: RestoreCoordinatorOpt,
        global_opt: GlobalRestoreOptions,
        storage: Arc<dyn BackupStorage>,
    ) -> Self {
        Self {
            storage,
            global_opt,
            metadata_cache_opt: opt.metadata_cache_opt,
            replay_all: opt.replay_all,
            ledger_history_start_version: opt.ledger_history_start_version,
            skip_epoch_endings: opt.skip_epoch_endings,
        }
    }

    pub async fn run(self) -> Result<()> {
        info!("Restore coordinator started.");
        COORDINATOR_START_TS.set(unix_timestamp_sec());

        let ret = self.run_impl().await;

        if let Err(e) = &ret {
            error!(
                error = ?e,
                "Restore coordinator failed."
            );
            COORDINATOR_FAIL_TS.set(unix_timestamp_sec());
        } else {
            info!("Restore coordinator exiting with success.");
            COORDINATOR_SUCC_TS.set(unix_timestamp_sec());
        }

        ret
    }

    async fn run_impl(self) -> Result<()> {
        // N.b.
        // The coordinator now focuses on doing one procedure, ignoring the combination of options
        // supported before:
        //   1. a most recent state snapshot
        //   2. a only transaction and its output, at the state snapshot version
        //   3. the epoch history from 0 up until the latest closed epoch preceding the state
        //      snapshot version.
        // And it does so in a resume-able way.

        if self.replay_all {
            bail!("--replay--all not supported in this version.");
        }
        if self.ledger_history_start_version.is_some() {
            bail!("--ledger-history-start-version not supported in this version.");
        }

        let metadata_view = metadata::cache::sync_and_load(
            &self.metadata_cache_opt,
            Arc::clone(&self.storage),
            self.global_opt.concurrent_downloads,
        )
        .await?;

        let next_txn_version = self
            .global_opt
            .run_mode
            .get_next_expected_transaction_version()?;
        if next_txn_version != 0 {
            // DB is already in workable state
            info!(
                next_txn_version = next_txn_version,
                "DB is ready to accept transactions, start the node to catch up with the chain. \
                If the node is unable to catch up because the DB is too old, delete the data folder \
                and bootstrap again.",
            );
            return Ok(());
        }

        let state_snapshot_backup =
            if let Some(version) = self.global_opt.run_mode.get_in_progress_state_snapshot()? {
                info!(
                    version = version,
                    "Found in progress state snapshot restore",
                );
                metadata_view.expect_state_snapshot(version)?
            } else {
                let max_txn_ver = metadata_view
                    .max_transaction_version()?
                    .ok_or_else(|| anyhow!("No transaction backup found."))?;
                metadata_view
                    .select_state_snapshot(std::cmp::min(self.target_version(), max_txn_ver))?
                    .ok_or_else(|| anyhow!("No usable state snapshot."))?
            };
        let version = state_snapshot_backup.version;
        let epoch_ending_backups = metadata_view.select_epoch_ending_backups(version)?;
        let transaction_backup = metadata_view
            .select_transaction_backups(version, version)?
            .pop()
            .unwrap();
        COORDINATOR_TARGET_VERSION.set(version as i64);
        info!(version = version, "Restore target decided.");

        let epoch_history = if !self.skip_epoch_endings {
            Some(Arc::new(
                EpochHistoryRestoreController::new(
                    epoch_ending_backups
                        .into_iter()
                        .map(|backup| backup.manifest)
                        .collect(),
                    self.global_opt.clone(),
                    self.storage.clone(),
                )
                .run()
                .await?,
            ))
        } else {
            None
        };

        StateSnapshotRestoreController::new(
            StateSnapshotRestoreOpt {
                manifest_handle: state_snapshot_backup.manifest,
                version,
            },
            self.global_opt.clone(),
            Arc::clone(&self.storage),
            epoch_history.clone(),
        )
        .run()
        .await?;

        let txn_manifests = vec![transaction_backup.manifest];
        TransactionRestoreBatchController::new(
            self.global_opt,
            self.storage,
            txn_manifests,
            Some(version + 1),
            epoch_history,
        )
        .run()
        .await?;

        Ok(())
    }
}

impl RestoreCoordinator {
    fn target_version(&self) -> Version {
        self.global_opt.target_version
    }

    #[allow(dead_code)]
    fn get_actual_target_version(
        &self,
        transaction_backups: &[TransactionBackupMeta],
    ) -> Result<Version> {
        if let Some(b) = transaction_backups.last() {
            if b.last_version > self.target_version() {
                Ok(self.target_version())
            } else {
                warn!(
                    "Can't find transaction backup containing the target version, \
                    will restore as much as possible"
                );
                Ok(b.last_version)
            }
        } else {
            bail!("No transaction backup found.")
        }
    }
}

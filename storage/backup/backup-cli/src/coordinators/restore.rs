// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
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
use anyhow::{anyhow, bail, ensure, Result};
use aptos_db::state_restore::StateSnapshotRestoreMode;
use aptos_executor_types::VerifyExecutionMode;
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

    /// Support two modes
    /// 1. restore to target version
    /// 2. restore a DB with all data ranging from start_version to target_version
    /// We basically introduce a ledger history start version (lhs), a replay start version (rs) and target version
    /// We directly store the write set and key values between (lhs, rs) and replay txn from (rs, target]
    async fn run_impl(self) -> Result<()> {
        // N.b.
        // The coordinator now focuses on doing one procedure, ignoring the combination of options
        // supported before:
        //   1. a most recent state snapshot before --target-version
        //   2. a only transaction and its output, at the state snapshot version
        //   3. the epoch history from 0 up until the latest closed epoch preceding the state
        //      snapshot version.
        // And it does so in a resume-able way.

        if self.replay_all {
            bail!("--replay--all not supported in this version.");
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

        let target_version = self.global_opt.target_version;
        COORDINATOR_TARGET_VERSION.set(target_version as i64);

        // calculate the start_version and replay_version
        let max_txn_ver = metadata_view
            .max_transaction_version()?
            .ok_or_else(|| anyhow!("No transaction backup found."))?;
        let lhs = self.ledger_history_start_version();
        let snapshot_before_lhs =
            metadata_view.select_state_snapshot(std::cmp::min(lhs, max_txn_ver))?;

        let snapshot_before_target = metadata_view
            .select_state_snapshot(std::cmp::min(self.target_version(), max_txn_ver))?;
        ensure!(
            snapshot_before_lhs.is_some() && snapshot_before_target.is_some(),
            "No snapshot exists before the target version({}) including genesis",
            target_version
        );
        let snapshot_before_lhs = snapshot_before_lhs.unwrap();
        let snapshot_before_target = snapshot_before_target.unwrap();
        ensure!(
            snapshot_before_lhs.version <= snapshot_before_target.version,
            "snapshot_before_target {} should be larger than or equal to snapshot_before_lhs {}",
            snapshot_before_target.version,
            snapshot_before_lhs.version
        );

        if let Some(version) = self.global_opt.run_mode.get_in_progress_state_snapshot()? {
            info!(
                version = version,
                "Found in progress state snapshot restore",
            );
            ensure!(snapshot_before_lhs.version == version, "Inconsistent in-progress state snapshot version, please delete the data folder and try again.");
        }
        info!(target_version = target_version, "Restore target decided.");
        let transaction_backups = metadata_view
            .select_transaction_backups(snapshot_before_lhs.version, target_version)?;
        let epoch_ending_backups = metadata_view.select_epoch_ending_backups(target_version)?;
        // This is used to t
        let mut expected_txn_history_so_far = None;

        // Restore the the state kv between lhs and rs
        if snapshot_before_lhs.version < snapshot_before_target.version {
            let start_version = snapshot_before_lhs.version + 1;
            let epoch_handles = epoch_ending_backups
                .iter()
                .filter(|e| e.first_version <= snapshot_before_target.version)
                .map(|backup| backup.manifest.clone())
                .collect();
            let epoch_history = if !self.skip_epoch_endings {
                Some(Arc::new(
                    EpochHistoryRestoreController::new(
                        epoch_handles,
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
                    manifest_handle: snapshot_before_lhs.manifest,
                    version: snapshot_before_lhs.version,
                    validate_modules: false,
                    restore_mode: StateSnapshotRestoreMode::KvOnly,
                },
                self.global_opt.clone(),
                Arc::clone(&self.storage),
                epoch_history.clone(),
            )
            .run()
            .await?;
            let txn_manifests = transaction_backups
                .iter()
                .filter(|e| {
                    e.last_version >= start_version
                        && e.first_version < snapshot_before_target.version
                })
                .map(|e| e.manifest.clone())
                .collect();
            // update the kv to the kv db
            let mut transaction_restore_opt = self.global_opt.clone();
            transaction_restore_opt.target_version = snapshot_before_target.version;
            TransactionRestoreBatchController::new(
                transaction_restore_opt,
                Arc::clone(&self.storage),
                txn_manifests,
                None,
                epoch_history.clone(),
                VerifyExecutionMode::NoVerify,
                None,
                None,
            )
            .run()
            .await?;

            // We already save txn till snapshot_before_target.version. We should not need to save them again.
            expected_txn_history_so_far = Some(snapshot_before_target.version + 1);
        }

        // Restore the full snapshot and replay till the target version
        {
            let epoch_handles = epoch_ending_backups
                .iter()
                .filter(|e| e.first_version <= target_version)
                .map(|backup| backup.manifest.clone())
                .collect();
            let epoch_history = if !self.skip_epoch_endings {
                Some(Arc::new(
                    EpochHistoryRestoreController::new(
                        epoch_handles,
                        self.global_opt.clone(),
                        self.storage.clone(),
                    )
                    .run()
                    .await?,
                ))
            } else {
                None
            };
            // For boostrap DB to latest version, we want to use default mode
            let restore_mode = if expected_txn_history_so_far.is_some() {
                StateSnapshotRestoreMode::TreeOnly
            } else {
                StateSnapshotRestoreMode::Default
            };
            StateSnapshotRestoreController::new(
                StateSnapshotRestoreOpt {
                    manifest_handle: snapshot_before_target.manifest.clone(),
                    version: snapshot_before_target.version,
                    validate_modules: false,
                    restore_mode,
                },
                self.global_opt.clone(),
                Arc::clone(&self.storage),
                epoch_history.clone(),
            )
            .run()
            .await?;

            let txn_manifests = transaction_backups
                .iter()
                .filter(|e| e.last_version >= snapshot_before_target.version)
                .map(|e| e.manifest.clone())
                .collect();

            TransactionRestoreBatchController::new(
                self.global_opt,
                self.storage,
                txn_manifests,
                Some(snapshot_before_target.version + 1),
                epoch_history,
                VerifyExecutionMode::NoVerify,
                None,
                expected_txn_history_so_far,
            )
            .run()
            .await?;
        }

        Ok(())
    }
}

impl RestoreCoordinator {
    fn target_version(&self) -> Version {
        self.global_opt.target_version
    }

    fn ledger_history_start_version(&self) -> Version {
        self.ledger_history_start_version
            .unwrap_or_else(|| self.target_version())
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

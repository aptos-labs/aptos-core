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
    metadata::cache::MetadataCacheOpt,
    metrics::verify::{
        VERIFY_COORDINATOR_FAIL_TS, VERIFY_COORDINATOR_START_TS, VERIFY_COORDINATOR_SUCC_TS,
    },
    storage::BackupStorage,
    utils::{unix_timestamp_sec, GlobalRestoreOptions, RestoreRunMode, TrustedWaypointOpt},
};
use anyhow::Result;
use aptos_db::state_restore::StateSnapshotRestoreMode;
use aptos_executor_types::VerifyExecutionMode;
use aptos_logger::prelude::*;
use aptos_types::transaction::Version;
use std::{path::PathBuf, sync::Arc};

pub struct VerifyCoordinator {
    storage: Arc<dyn BackupStorage>,
    metadata_cache_opt: MetadataCacheOpt,
    trusted_waypoints_opt: TrustedWaypointOpt,
    concurrent_downloads: usize,
    start_version: Version,
    end_version: Version,
    state_snapshot_before_version: Version,
    skip_epoch_endings: bool,
    validate_modules: bool,
    output_transaction_analysis: Option<PathBuf>,
}

impl VerifyCoordinator {
    pub fn new(
        storage: Arc<dyn BackupStorage>,
        metadata_cache_opt: MetadataCacheOpt,
        trusted_waypoints_opt: TrustedWaypointOpt,
        concurrent_downloads: usize,
        start_version: Version,
        end_version: Version,
        state_snapshot_before_version: Version,
        skip_epoch_endings: bool,
        validate_modules: bool,
        output_transaction_analysis: Option<PathBuf>,
    ) -> Result<Self> {
        Ok(Self {
            storage,
            metadata_cache_opt,
            trusted_waypoints_opt,
            concurrent_downloads,
            start_version,
            end_version,
            state_snapshot_before_version,
            skip_epoch_endings,
            validate_modules,
            output_transaction_analysis,
        })
    }

    pub async fn run(self) -> Result<()> {
        info!("Verify coordinator started.");
        VERIFY_COORDINATOR_START_TS.set(unix_timestamp_sec());

        let ret = self.run_impl().await;

        if let Err(e) = &ret {
            error!(
                error = ?e,
                "Verify coordinator failed."
            );
            VERIFY_COORDINATOR_FAIL_TS.set(unix_timestamp_sec());
        } else {
            info!("Verify coordinator exiting with success.");
            VERIFY_COORDINATOR_SUCC_TS.set(unix_timestamp_sec());
        }
        ret
    }

    async fn run_impl(self) -> Result<()> {
        let metadata_view = metadata::cache::sync_and_load(
            &self.metadata_cache_opt,
            Arc::clone(&self.storage),
            self.concurrent_downloads,
        )
        .await?;
        let ver_max = Version::MAX;
        let state_snapshot =
            metadata_view.select_state_snapshot(self.state_snapshot_before_version)?;
        let transactions =
            metadata_view.select_transaction_backups(self.start_version, self.end_version)?;
        let epoch_endings = metadata_view.select_epoch_ending_backups(ver_max)?;

        let global_opt = GlobalRestoreOptions {
            target_version: ver_max,
            trusted_waypoints: Arc::new(self.trusted_waypoints_opt.verify()?),
            run_mode: Arc::new(RestoreRunMode::Verify),
            concurrent_downloads: self.concurrent_downloads,
            replay_concurrency_level: 0, // won't replay, doesn't matter
        };

        let epoch_history = if self.skip_epoch_endings {
            None
        } else {
            Some(Arc::new(
                EpochHistoryRestoreController::new(
                    epoch_endings
                        .into_iter()
                        .map(|backup| backup.manifest)
                        .collect(),
                    global_opt.clone(),
                    self.storage.clone(),
                )
                .run()
                .await?,
            ))
        };

        if let Some(backup) = state_snapshot {
            info!(
                epoch = backup.epoch,
                version = backup.version,
                "State snapshot selected for verification."
            );
            StateSnapshotRestoreController::new(
                StateSnapshotRestoreOpt {
                    manifest_handle: backup.manifest,
                    version: backup.version,
                    validate_modules: self.validate_modules,
                    restore_mode: StateSnapshotRestoreMode::Default,
                },
                global_opt.clone(),
                Arc::clone(&self.storage),
                epoch_history.clone(),
            )
            .run()
            .await?;
        }

        let txn_manifests = transactions.into_iter().map(|b| b.manifest).collect();
        TransactionRestoreBatchController::new(
            global_opt,
            self.storage,
            txn_manifests,
            None,
            None, /* replay_from_version */
            epoch_history,
            VerifyExecutionMode::NoVerify,
            self.output_transaction_analysis,
        )
        .run()
        .await?;

        Ok(())
    }
}

// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    backup_types::{
        state_snapshot::restore::{StateSnapshotRestoreController, StateSnapshotRestoreOpt},
        transaction::restore::TransactionRestoreBatchController,
    },
    metadata,
    metadata::cache::MetadataCacheOpt,
    storage::BackupStorage,
    utils::{GlobalRestoreOptions, RestoreRunMode, TrustedWaypointOpt},
};
use anyhow::Result;
use velor_db::backup::restore_handler::RestoreHandler;
use velor_executor_types::VerifyExecutionMode;
use velor_logger::prelude::*;
use velor_storage_interface::VelorDbError;
use velor_types::{on_chain_config::TimedFeatureOverride, transaction::Version};
use velor_vm::VelorVM;
use velor_vm_environment::prod_configs::set_timed_feature_override;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ReplayError {
    #[error("Txn mismatch error")]
    TxnMismatch,
    #[error("Other Replay error {0}")]
    OtherError(String),
}

impl From<anyhow::Error> for ReplayError {
    fn from(error: anyhow::Error) -> Self {
        ReplayError::OtherError(error.to_string())
    }
}

impl From<VelorDbError> for ReplayError {
    fn from(error: VelorDbError) -> Self {
        ReplayError::OtherError(error.to_string())
    }
}
pub struct ReplayVerifyCoordinator {
    storage: Arc<dyn BackupStorage>,
    metadata_cache_opt: MetadataCacheOpt,
    trusted_waypoints_opt: TrustedWaypointOpt,
    concurrent_downloads: usize,
    replay_concurrency_level: usize,
    restore_handler: RestoreHandler,
    start_version: Version,
    end_version: Version,
    validate_modules: bool,
    verify_execution_mode: VerifyExecutionMode,
}

impl ReplayVerifyCoordinator {
    pub fn new(
        storage: Arc<dyn BackupStorage>,
        metadata_cache_opt: MetadataCacheOpt,
        trusted_waypoints_opt: TrustedWaypointOpt,
        concurrent_downloads: usize,
        replay_concurrency_level: usize,
        restore_handler: RestoreHandler,
        start_version: Version,
        end_version: Version,
        validate_modules: bool,
        verify_execution_mode: VerifyExecutionMode,
    ) -> Result<Self> {
        Ok(Self {
            storage,
            metadata_cache_opt,
            trusted_waypoints_opt,
            concurrent_downloads,
            replay_concurrency_level,
            restore_handler,
            start_version,
            end_version,
            validate_modules,
            verify_execution_mode,
        })
    }

    pub async fn run(self) -> Result<(), ReplayError> {
        info!("ReplayVerify coordinator started.");
        let ret = self.run_impl().await;

        if let Err(e) = &ret {
            error!(
                error = ?e,
                "ReplayVerify coordinator failed."
            );
        } else {
            info!("ReplayVerify coordinator exiting with success.");
        }

        ret
    }

    async fn run_impl(self) -> Result<(), ReplayError> {
        VelorVM::set_concurrency_level_once(self.replay_concurrency_level);
        set_timed_feature_override(TimedFeatureOverride::Replay);

        let metadata_view = metadata::cache::sync_and_load(
            &self.metadata_cache_opt,
            Arc::clone(&self.storage),
            self.concurrent_downloads,
        )
        .await?;
        if self.start_version > self.end_version {
            return Err(ReplayError::OtherError(format!(
                "start_version {} should precede end_version {}.",
                self.start_version, self.end_version
            )));
        }

        let run_mode = Arc::new(RestoreRunMode::Restore {
            restore_handler: self.restore_handler,
        });
        let mut next_txn_version = run_mode.get_next_expected_transaction_version()?;
        let (state_snapshot, snapshot_version) = if let Some(version) =
            run_mode.get_in_progress_state_kv_snapshot()?
        {
            info!(
                version = version,
                "Found in progress state snapshot restore",
            );
            (
                Some(metadata_view.expect_state_snapshot(version)?),
                Some(version),
            )
        } else if let Some(snapshot) = metadata_view.select_state_snapshot(self.start_version)? {
            let snapshot_version = snapshot.version;
            info!(
                "Found state snapshot backup at epoch {}, will replay from version {}.",
                snapshot.epoch,
                snapshot_version + 1
            );
            (Some(snapshot), Some(snapshot_version))
        } else {
            (None, None)
        };

        let skip_snapshot: bool =
            snapshot_version.is_none() || next_txn_version > snapshot_version.unwrap();
        if skip_snapshot {
            info!(
                next_txn_version = next_txn_version,
                snapshot_version = snapshot_version,
                "found in progress replay and skip the state snapshot restore",
            );
        }

        // Once it begins replay, we want to directly start from the version that failed
        let save_start_version = (next_txn_version > 0).then_some(next_txn_version);

        next_txn_version = std::cmp::max(next_txn_version, snapshot_version.map_or(0, |v| v + 1));

        let transactions = metadata_view.select_transaction_backups(
            // transaction info at the snapshot must be restored otherwise the db will be confused
            // about the latest version after snapshot is restored.
            next_txn_version.saturating_sub(1),
            self.end_version,
        )?;
        let global_opt = GlobalRestoreOptions {
            target_version: self.end_version,
            trusted_waypoints: Arc::new(self.trusted_waypoints_opt.verify()?),
            run_mode,
            concurrent_downloads: self.concurrent_downloads,
            replay_concurrency_level: 0, // won't replay, doesn't matter
        };

        if !skip_snapshot {
            if let Some(backup) = state_snapshot {
                StateSnapshotRestoreController::new(
                    StateSnapshotRestoreOpt {
                        manifest_handle: backup.manifest,
                        version: backup.version,
                        validate_modules: self.validate_modules,
                        restore_mode: Default::default(),
                    },
                    global_opt.clone(),
                    Arc::clone(&self.storage),
                    None, /* epoch_history */
                )
                .run()
                .await?;
            }
        }

        TransactionRestoreBatchController::new(
            global_opt,
            self.storage,
            transactions
                .into_iter()
                .map(|t| t.manifest)
                .collect::<Vec<_>>(),
            save_start_version,
            Some((next_txn_version, false)), /* replay_from_version */
            None,                            /* epoch_history */
            self.verify_execution_mode.clone(),
            None,
        )
        .run()
        .await?;

        if self.verify_execution_mode.seen_error() {
            Err(ReplayError::TxnMismatch)
        } else {
            Ok(())
        }
    }
}

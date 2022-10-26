// Copyright (c) Aptos
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
use anyhow::{ensure, Result};
use aptos_logger::prelude::*;
use aptos_types::transaction::Version;
use aptos_vm::AptosVM;
use aptosdb::backup::restore_handler::RestoreHandler;
use std::sync::Arc;

pub struct ReplayVerifyCoordinator {
    storage: Arc<dyn BackupStorage>,
    metadata_cache_opt: MetadataCacheOpt,
    trusted_waypoints_opt: TrustedWaypointOpt,
    concurrent_downloads: usize,
    replay_concurrency_level: usize,
    restore_handler: RestoreHandler,
    start_version: Version,
    end_version: Version,
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
        })
    }

    pub async fn run(self) -> Result<()> {
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

    async fn run_impl(self) -> Result<()> {
        AptosVM::set_concurrency_level_once(self.replay_concurrency_level);

        let metadata_view = metadata::cache::sync_and_load(
            &self.metadata_cache_opt,
            Arc::clone(&self.storage),
            self.concurrent_downloads,
        )
        .await?;
        ensure!(
            self.start_version <= self.end_version,
            "start_version should precede end_version."
        );

        let state_snapshot = if self.start_version == 0 {
            None
        } else {
            metadata_view.select_state_snapshot(self.start_version.wrapping_sub(1))?
        };
        let replay_transactions_from_version =
            state_snapshot.as_ref().map(|b| b.version + 1).unwrap_or(0);
        let transactions = metadata_view.select_transaction_backups(
            // transaction info at the snapshot must be restored otherwise the db will be confused
            // about the latest version after snapshot is restored.
            replay_transactions_from_version.saturating_sub(1),
            self.end_version,
        )?;

        let global_opt = GlobalRestoreOptions {
            target_version: self.end_version,
            trusted_waypoints: Arc::new(self.trusted_waypoints_opt.verify()?),
            run_mode: Arc::new(RestoreRunMode::Restore {
                restore_handler: self.restore_handler,
            }),
            concurrent_downloads: self.concurrent_downloads,
            replay_concurrency_level: 0, // won't replay, doesn't matter
        };

        if let Some(backup) = state_snapshot {
            StateSnapshotRestoreController::new(
                StateSnapshotRestoreOpt {
                    manifest_handle: backup.manifest,
                    version: backup.version,
                },
                global_opt.clone(),
                Arc::clone(&self.storage),
                None, /* epoch_history */
            )
            .run()
            .await?;
        }

        let txn_manifests = transactions.into_iter().map(|b| b.manifest).collect();
        TransactionRestoreBatchController::new(
            global_opt,
            self.storage,
            txn_manifests,
            Some(replay_transactions_from_version), /* replay_from_version */
            None,                                   /* epoch_history */
        )
        .run()
        .await?;

        Ok(())
    }
}

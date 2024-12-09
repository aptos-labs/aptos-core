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
    /// 1. restore to target version when do_phase_1 is false. We restore a closest snapshot and replay txns till the target version
    /// 2. restore a DB with all data ranging from start_version to target_version with all KV restored between ledger_history_start_version and target_version along with the latest tree at target version.

    /// The overall flow is as follows:
    /// The first phase is restore till the tree snapshot before the target version. It includes the following work
    /// a. restore the KV snapshot before ledger history start version, which also restore StateStorageUsage at the version
    /// b. start from the first transaction of loaded chunk, save the txn accumualator, and apply transactions till the KV snapshot. We don't restore state KVs here since we can't calculate StateStorageUsage before the KV snapshot.
    /// we start save transaction and restore KV after kv_snapshot version till the tree_snapshot before target version
    ///
    /// The second phase is restore the tree snapshot and replay txns till the target version
    /// a. restore the tree snapshot
    /// b. replay the txn till the target version
    ///
    /// we are support the resume from any point when the restore is interrupted.
    async fn run_impl(self) -> Result<()> {
        // if replay_all is set, we will replay all transactions from the lhs to the target version
        let mut replay_all_mode = false;
        if self.replay_all {
            info!("Replay all mode is enabled.");
            replay_all_mode = true;
        }

        info!("This tool only guarantees resume from previous in-progress restore. \
        If you want to restore a new DB, please either specify a new target db dir or delete previous in-progress DB in the target db dir.");

        let metadata_view = metadata::cache::sync_and_load(
            &self.metadata_cache_opt,
            Arc::clone(&self.storage),
            self.global_opt.concurrent_downloads,
        )
        .await?;

        // calculate the start_version and replay_version
        let max_txn_ver = metadata_view
            .max_transaction_version()?
            .ok_or_else(|| anyhow!("No transaction backup found."))?;
        let target_version = 6559979983;
        info!(
            "User specified target version: {}, max transaction version: {}, Target version is set to {}",
            self.global_opt.target_version, max_txn_ver, target_version
        );

        COORDINATOR_TARGET_VERSION.set(target_version as i64);
        let lhs = self.ledger_history_start_version();

        let latest_tree_version = self
            .global_opt
            .run_mode
            .get_state_snapshot_before(Version::MAX);
        let tree_completed = {
            match latest_tree_version {
                Some((ver, _)) => self
                    .global_opt
                    .run_mode
                    .get_state_snapshot_before(ver)
                    .is_some(),
                None => false,
            }
        };

        let mut db_next_version = self
            .global_opt
            .run_mode
            .get_next_expected_transaction_version()?;

        let kv_snapshot = match self.global_opt.run_mode.get_in_progress_state_kv_snapshot() {
            Ok(Some(ver)) => {
                if db_next_version >= ver {
                    // already restored the kv snapshot, no need to restore again
                    None
                } else {
                    let snapshot = metadata_view.select_state_snapshot(ver)?;
                    ensure!(
                        snapshot.is_some() && snapshot.as_ref().unwrap().version == ver,
                        "cannot find in-progress state snapshot {}",
                        ver
                    );
                    snapshot
                }
            },
            Ok(None) | Err(_) => {
                assert_eq!(
                    db_next_version, 0,
                    "DB should be empty if no in-progress state snapshot found"
                );
                metadata_view
                    .select_state_snapshot(std::cmp::min(lhs, max_txn_ver))
                    .expect("Cannot find any snapshot before ledger history start version")
            },
        };

        let epoch_ending_backups = metadata_view.select_epoch_ending_backups(target_version)?;
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

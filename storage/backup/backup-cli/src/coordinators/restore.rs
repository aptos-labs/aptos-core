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
        if self.replay_all {
            bail!("--replay--all not supported in this version.");
        }

        info!("This tool only guarantees resume from previous in-progress restore. \
        If you want to restore a new DB, please either specify a new target db dir or delete previous in-progress DB in the target db dir.");

        let metadata_view = metadata::cache::sync_and_load(
            &self.metadata_cache_opt,
            Arc::clone(&self.storage),
            self.global_opt.concurrent_downloads,
        )
        .await?;

        let target_version = self.global_opt.target_version;
        COORDINATOR_TARGET_VERSION.set(target_version as i64);

        // calculate the start_version and replay_version
        let max_txn_ver = metadata_view
            .max_transaction_version()?
            .ok_or_else(|| anyhow!("No transaction backup found."))?;
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

        let kv_snapshot = if db_next_version == 0 {
            match self.global_opt.run_mode.get_in_progress_state_kv_snapshot() {
                Ok(Some(ver)) => {
                    let snapshot = metadata_view.select_state_snapshot(ver)?;
                    ensure!(
                        snapshot.is_some() && snapshot.as_ref().unwrap().version == ver,
                        "cannot find in-progress state snapshot {}",
                        ver
                    );
                    snapshot
                },
                Ok(None) | Err(_) => {
                    metadata_view.select_state_snapshot(std::cmp::min(lhs, max_txn_ver))?
                },
            }
        } else {
            None
        };

        let tree_snapshot = metadata_view
            .select_state_snapshot(std::cmp::min(self.target_version(), max_txn_ver))?
            .expect("Cannot find tree snapshot before target version");

        let two_phase_restore = if let Some(kv_snapshot) = kv_snapshot.as_ref() {
            // if we have a kv snapshot, we need to restore the state between lhs and rs
            kv_snapshot.version < tree_snapshot.version
        } else {
            // if we don't have a kv snapshot, we need to restore the state between db_next_version and rs
            db_next_version < tree_snapshot.version && db_next_version > 0
        };
        let txn_start_version = if kv_snapshot.is_some() {
            kv_snapshot.as_ref().unwrap().version
        } else {
            db_next_version
        };
        let transaction_backups =
            metadata_view.select_transaction_backups(txn_start_version, target_version)?;
        let epoch_ending_backups = metadata_view.select_epoch_ending_backups(target_version)?;

        // Restore the the state kv between lhs and rs
        if two_phase_restore {
            let start_version = if let Some(ref kv_snapshot) = kv_snapshot {
                kv_snapshot.version
            } else {
                db_next_version
            };
            info!(
                "Start restoring DB from version {} to tree snapshot version {}",
                start_version, tree_snapshot.version,
            );
            let epoch_handles = epoch_ending_backups
                .iter()
                .filter(|e| e.first_version <= tree_snapshot.version)
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

            if kv_snapshot.is_some() {
                let kv_snapshot = kv_snapshot.unwrap();
                info!("Start restoring KV snapshot at {}", kv_snapshot.version);

                StateSnapshotRestoreController::new(
                    StateSnapshotRestoreOpt {
                        manifest_handle: kv_snapshot.manifest,
                        version: kv_snapshot.version,
                        validate_modules: false,
                        restore_mode: StateSnapshotRestoreMode::KvOnly,
                    },
                    self.global_opt.clone(),
                    Arc::clone(&self.storage),
                    epoch_history.clone(),
                )
                .run()
                .await?;
            }
            let txn_manifests = transaction_backups
                .iter()
                .filter(|e| e.first_version < tree_snapshot.version)
                .map(|e| e.manifest.clone())
                .collect();
            // update the kv to the kv db
            let mut transaction_restore_opt = self.global_opt.clone();
            transaction_restore_opt.target_version = tree_snapshot.version - 1;
            TransactionRestoreBatchController::new(
                transaction_restore_opt,
                Arc::clone(&self.storage),
                txn_manifests,
                None,
                None,
                epoch_history.clone(),
                VerifyExecutionMode::NoVerify,
                None,
            )
            .run()
            .await?;
            // update the expected version for the first phase restore
            db_next_version = tree_snapshot.version;
        }

        // Restore the full tree snapshot and replay till the target version
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

            let first_version = if db_next_version == 0 {
                None
            } else {
                Some(db_next_version)
            };
            let mut replay_version = first_version;

            info!(
                "Starting restore DB from version {} to target version {}",
                db_next_version, target_version,
            );
            // If the tree is not completed, we directly restore from the latest snapshot before target
            if !tree_completed {
                // For boostrap DB to latest version, we want to use default mode
                let restore_mode = if db_next_version > 0 {
                    StateSnapshotRestoreMode::TreeOnly
                } else {
                    StateSnapshotRestoreMode::Default
                };
                info!(
                    "Start restoring tree snapshot at {} with db_next_version {}",
                    tree_snapshot.version, db_next_version
                );

                StateSnapshotRestoreController::new(
                    StateSnapshotRestoreOpt {
                        manifest_handle: tree_snapshot.manifest.clone(),
                        version: tree_snapshot.version,
                        validate_modules: false,
                        restore_mode,
                    },
                    self.global_opt.clone(),
                    Arc::clone(&self.storage),
                    epoch_history.clone(),
                )
                .run()
                .await?;
                if restore_mode == StateSnapshotRestoreMode::Default {
                    replay_version = Some(tree_snapshot.version + 1);
                }
            }

            let txn_manifests = transaction_backups
                .iter()
                .filter(|e| e.last_version >= db_next_version)
                .map(|e| e.manifest.clone())
                .collect();
            TransactionRestoreBatchController::new(
                self.global_opt,
                self.storage,
                txn_manifests,
                first_version,
                replay_version,
                epoch_history,
                VerifyExecutionMode::NoVerify,
                None,
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

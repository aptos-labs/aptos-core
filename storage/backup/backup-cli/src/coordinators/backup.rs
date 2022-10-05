// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    backup_types::{
        epoch_ending::backup::{EpochEndingBackupController, EpochEndingBackupOpt},
        state_snapshot::backup::{StateSnapshotBackupController, StateSnapshotBackupOpt},
        transaction::backup::{TransactionBackupController, TransactionBackupOpt},
    },
    metadata,
    metadata::cache::MetadataCacheOpt,
    metrics::backup::{
        EPOCH_ENDING_EPOCH, HEARTBEAT_TS, STATE_SNAPSHOT_EPOCH, TRANSACTION_VERSION,
    },
    storage::BackupStorage,
    utils::{
        backup_service_client::BackupServiceClient, unix_timestamp_sec, ConcurrentDownloadsOpt,
        GlobalBackupOpt,
    },
};
use anyhow::{anyhow, ensure, Result};
use aptos_logger::prelude::*;
use aptos_types::transaction::Version;
use aptosdb::backup::backup_handler::DbState;
use clap::Parser;
use futures::{stream, Future, StreamExt};
use std::{fmt::Debug, sync::Arc};
use tokio::{
    sync::watch,
    time::{interval, Duration},
};
use tokio_stream::wrappers::IntervalStream;

#[derive(Parser)]
pub struct BackupCoordinatorOpt {
    #[clap(flatten)]
    pub metadata_cache_opt: MetadataCacheOpt,
    // Defaulting to 1 to try to always have the latest state snapshot.
    #[clap(
        long,
        default_value = "1",
        help = "Frequency (in number of epochs) to take state snapshots at epoch ending versions. \
        Adjacent epochs share much of the state, so it's inefficient storage-wise and bandwidth-wise \
        to take it too frequently. However, a recent snapshot is obviously desirable if one intends \
        to recover a snapshot and catch up with the chain by replaying transactions on top of it. \
        Notice: If, while a snapshot is being taken, the chain advanced several epoch, past several \
        new points where a snapshot is eligible according to this setting, we will skip those in the \
        middle and take only at the newest epoch among them. For example, if the setting is 5, \
        then the snapshots will be at at 0, 5, 10 ... If when the snapshot at 5 ends the chain \
        is already at 19, then snapshot at 15 will be taken instead of at 10 (not at 18)."
    )]
    pub state_snapshot_interval_epochs: usize,
    // Defaulting to 1M, which converts to a 20 minutes delay of a transaction showing up in a backup,
    // from a 1K TPS chain, and a few minutes replay time.
    #[clap(
        long,
        default_value = "1000000",
        help = "The frequency (in transaction versions) to take an incremental transaction backup. \
        Making a transaction backup every 10 Million versions will result in the latest transaction \
        to appear in the backup potentially 10 Million versions later. If the net work is running \
        at 1 thousand transactions per second, that is roughly 3 hours. On the other hand, if \
        backups are too frequent and hence small, it slows down loading the backup metadata by too \
        many small files. "
    )]
    pub transaction_batch_size: usize,
    #[clap(flatten)]
    pub concurrent_downloads: ConcurrentDownloadsOpt,
}

impl BackupCoordinatorOpt {
    fn validate(&self) -> Result<()> {
        ensure!(
            self.state_snapshot_interval_epochs > 0 && self.transaction_batch_size > 0,
            "Backup interval and batch size must be greater than 0."
        );
        Ok(())
    }
}

pub struct BackupCoordinator {
    client: Arc<BackupServiceClient>,
    storage: Arc<dyn BackupStorage>,
    global_opt: GlobalBackupOpt,
    metadata_cache_opt: MetadataCacheOpt,
    state_snapshot_interval_epochs: usize,
    transaction_batch_size: usize,
    concurrent_downloads: usize,
}

impl BackupCoordinator {
    pub fn new(
        opt: BackupCoordinatorOpt,
        global_opt: GlobalBackupOpt,
        client: Arc<BackupServiceClient>,
        storage: Arc<dyn BackupStorage>,
    ) -> Self {
        opt.validate().unwrap();
        Self {
            client,
            storage,
            global_opt,
            metadata_cache_opt: opt.metadata_cache_opt,
            state_snapshot_interval_epochs: opt.state_snapshot_interval_epochs,
            transaction_batch_size: opt.transaction_batch_size,
            concurrent_downloads: opt.concurrent_downloads.get(),
        }
    }

    pub async fn run(&self) -> Result<()> {
        // Connect to both the local node and the backup storage.
        let backup_state = metadata::cache::sync_and_load(
            &self.metadata_cache_opt,
            Arc::clone(&self.storage),
            self.concurrent_downloads,
        )
        .await?
        .get_storage_state()?;

        // On new DbState retrieved:
        // `watch_db_state` informs `backup_epoch_endings` via channel 1,
        // and the latter informs the other backup type workers via channel 2, after epoch
        // ending is properly backed up, if necessary. This way, the epoch ending LedgerInfo needed
        // for proof verification is always available in the same backup storage.
        let (tx1, rx1) = watch::channel::<Option<DbState>>(None);
        let (tx2, rx2) = watch::channel::<Option<DbState>>(None);

        // Schedule work streams.
        let watch_db_state = IntervalStream::new(interval(Duration::from_secs(1)))
            .then(|_| self.try_refresh_db_state(&tx1))
            .boxed_local();

        let backup_epoch_endings = self
            .backup_work_stream(
                backup_state.latest_epoch_ending_epoch,
                &rx1,
                |slf, last_epoch, db_state| {
                    Self::backup_epoch_endings(slf, last_epoch, db_state, &tx2)
                },
            )
            .boxed_local();
        let backup_state_snapshots = self
            .backup_work_stream(
                backup_state.latest_state_snapshot_epoch,
                &rx2,
                Self::backup_state_snapshot,
            )
            .boxed_local();
        let backup_transactions = self
            .backup_work_stream(
                backup_state.latest_transaction_version,
                &rx2,
                Self::backup_transactions,
            )
            .boxed_local();

        info!("Backup coordinator started.");
        let mut all_work = stream::select_all(vec![
            watch_db_state,
            backup_epoch_endings,
            backup_state_snapshots,
            backup_transactions,
        ]);

        loop {
            all_work
                .next()
                .await
                .ok_or_else(|| anyhow!("Must be a bug: we never returned None."))?
        }
    }
}

impl BackupCoordinator {
    async fn try_refresh_db_state(&self, db_state_broadcast: &watch::Sender<Option<DbState>>) {
        match self.client.get_db_state().await {
            Ok(s) => {
                HEARTBEAT_TS.set(unix_timestamp_sec());
                if s.is_none() {
                    warn!("DB not bootstrapped.");
                } else {
                    db_state_broadcast
                        .send(s)
                        .map_err(|e| anyhow!("Receivers should not be cancelled: {}", e))
                        .unwrap()
                }
            }
            Err(e) => warn!(
                "Failed pulling DbState from local node: {}. Will keep trying.",
                e
            ),
        };
    }

    async fn backup_epoch_endings(
        &self,
        mut last_epoch_ending_epoch_in_backup: Option<u64>,
        db_state: DbState,
        downstream_db_state_broadcaster: &watch::Sender<Option<DbState>>,
    ) -> Result<Option<u64>> {
        loop {
            if let Some(epoch) = last_epoch_ending_epoch_in_backup {
                EPOCH_ENDING_EPOCH.set(epoch as i64);
            }
            let (first, last) = get_batch_range(last_epoch_ending_epoch_in_backup, 1);

            if db_state.epoch <= last {
                // "<=" because `db_state.epoch` hasn't ended yet, wait for the next db_state update
                break;
            }

            EpochEndingBackupController::new(
                EpochEndingBackupOpt {
                    start_epoch: first,
                    end_epoch: last + 1,
                },
                self.global_opt.clone(),
                Arc::clone(&self.client),
                Arc::clone(&self.storage),
            )
            .run()
            .await?;
            last_epoch_ending_epoch_in_backup = Some(last)
        }

        downstream_db_state_broadcaster
            .send(Some(db_state))
            .map_err(|e| anyhow!("Receivers should not be cancelled: {}", e))
            .unwrap();
        Ok(last_epoch_ending_epoch_in_backup)
    }

    async fn backup_state_snapshot(
        &self,
        last_snapshot_epoch_in_backup: Option<Version>,
        db_state: DbState,
    ) -> Result<Option<Version>> {
        if let Some(epoch) = last_snapshot_epoch_in_backup {
            STATE_SNAPSHOT_EPOCH.set(epoch as i64);
        }
        let epoch = get_next_snapshot(
            last_snapshot_epoch_in_backup,
            db_state,
            self.state_snapshot_interval_epochs,
        );

        // <= becuse db_state.epoch is still open
        if db_state.epoch <= epoch {
            // wait for the next db_state update
            return Ok(last_snapshot_epoch_in_backup);
        }

        StateSnapshotBackupController::new(
            StateSnapshotBackupOpt { epoch },
            self.global_opt.clone(),
            Arc::clone(&self.client),
            Arc::clone(&self.storage),
        )
        .run()
        .await?;

        Ok(Some(epoch))
    }

    async fn backup_transactions(
        &self,
        mut last_transaction_version_in_backup: Option<Version>,
        db_state: DbState,
    ) -> Result<Option<u64>> {
        loop {
            if let Some(version) = last_transaction_version_in_backup {
                TRANSACTION_VERSION.set(version as i64);
            }
            let (first, last) = get_batch_range(
                last_transaction_version_in_backup,
                self.transaction_batch_size,
            );

            if db_state.committed_version < last {
                // wait for the next db_state update
                return Ok(last_transaction_version_in_backup);
            }

            TransactionBackupController::new(
                TransactionBackupOpt {
                    start_version: first,
                    num_transactions: (last + 1 - first) as usize,
                },
                self.global_opt.clone(),
                Arc::clone(&self.client),
                Arc::clone(&self.storage),
            )
            .run()
            .await?;

            last_transaction_version_in_backup = Some(last);
        }
    }

    fn backup_work_stream<'a, S, W, Fut>(
        &'a self,
        initial_state: S,
        db_state_rx: &'a watch::Receiver<Option<DbState>>,
        worker: W,
    ) -> impl StreamExt<Item = ()> + 'a
    where
        S: Copy + Debug + 'a,
        W: Worker<'a, S, Fut> + Copy + 'a,
        Fut: Future<Output = Result<S>> + 'a,
    {
        stream::unfold(
            (initial_state, db_state_rx.clone()),
            move |(s, mut rx)| async move {
                rx.changed().await.unwrap();
                let db_state = *rx.borrow();
                if let Some(db_state) = db_state {
                    let next_state = worker(self, s, db_state).await.unwrap_or_else(|e| {
                        warn!("backup failed: {}. Keep trying with state {:?}.", e, s);
                        s
                    });
                    Some(((), (next_state, rx)))
                } else {
                    // initial state
                    Some(((), (s, rx)))
                }
            },
        )
    }
}

trait Worker<'a, S, Fut: Future<Output = Result<S>> + 'a>:
    Fn(&'a BackupCoordinator, S, DbState) -> Fut
{
}

impl<'a, T, S, Fut> Worker<'a, S, Fut> for T
where
    T: Fn(&'a BackupCoordinator, S, DbState) -> Fut,
    Fut: Future<Output = Result<S>> + 'a,
{
}

fn get_batch_range(last_in_backup: Option<u64>, batch_size: usize) -> (u64, u64) {
    // say, 7 is already in backup, and we target batches of size 10, we will return (8, 10) in this
    // case, so 8, 9, 10 will be in this batch, and next time the backup worker will pass in 10,
    // and we will return (11, 20). The transaction 0 will be in it's own batch.
    last_in_backup.map_or((0, 0), |n| {
        let first = n + 1;
        let batch = n / batch_size as u64 + 1;
        let last = batch * batch_size as u64;
        (first, last)
    })
}

fn get_next_snapshot(last_in_backup: Option<u64>, db_state: DbState, interval: usize) -> u64 {
    // We don't try to guarantee snapshots are taken at each applicable interval: when the backup
    // progress can't keep up with the ledger growth, we favor timeliness over completeness.
    // For example, with interval 100, when we finished taking a snapshot at version 700, if we
    // found the latest version is already 1250, the next snapshot we take will be at 1200, not 800.

    let next_for_storage = match last_in_backup {
        Some(last) => (last / interval as u64 + 1) * interval as u64,
        None => 0,
    };

    // Notice that db_state.epoch is not closed yet.
    let last_for_db: u64 = db_state.epoch.saturating_sub(1) / interval as u64 * interval as u64;

    std::cmp::max(next_for_storage, last_for_db)
}

#[cfg(test)]
mod tests {
    use crate::coordinators::backup::{get_batch_range, get_next_snapshot};
    use aptosdb::backup::backup_handler::DbState;

    #[test]
    fn test_get_batch_range() {
        assert_eq!(get_batch_range(None, 100), (0, 0));
        assert_eq!(get_batch_range(Some(0), 100), (1, 100));
        assert_eq!(get_batch_range(Some(100), 50), (101, 150));
        assert_eq!(get_batch_range(Some(150), 100), (151, 200));
        assert_eq!(get_batch_range(Some(200), 100), (201, 300));
    }

    #[test]
    fn test_get_next_snapshot() {
        let _state = |epoch| DbState {
            epoch,
            committed_version: 0,
        };

        assert_eq!(get_next_snapshot(None, _state(90), 100), 0);
        assert_eq!(get_next_snapshot(Some(0), _state(90), 100), 100);
        assert_eq!(get_next_snapshot(Some(0), _state(100), 100), 100);
        assert_eq!(get_next_snapshot(Some(0), _state(101), 100), 100);
        assert_eq!(get_next_snapshot(Some(0), _state(190), 100), 100);
        // Notice that epoch 200 is not closed yet.
        assert_eq!(get_next_snapshot(Some(0), _state(200), 100), 100);
        assert_eq!(get_next_snapshot(Some(0), _state(201), 100), 200);
        assert_eq!(get_next_snapshot(Some(0), _state(250), 100), 200);
        assert_eq!(get_next_snapshot(Some(200), _state(250), 100), 300);
    }
}

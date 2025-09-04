// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    backup_types::state_snapshot::{
        backup::{StateSnapshotBackupController, StateSnapshotBackupOpt},
        restore::{StateSnapshotRestoreController, StateSnapshotRestoreOpt},
    },
    storage::{local_fs::LocalFs, BackupStorage},
    utils::{
        backup_service_client::BackupServiceClient,
        test_utils::{start_local_backup_service, tmp_db_with_random_content},
        ConcurrentDownloadsOpt, GlobalBackupOpt, GlobalRestoreOpt, ReplayConcurrencyLevelOpt,
        RocksdbOpt, TrustedWaypointOpt,
    },
};
use velor_db::{state_restore::StateSnapshotRestoreMode, VelorDB};
use velor_storage_interface::DbReader;
use velor_temppath::TempPath;
use std::{convert::TryInto, sync::Arc};
use tokio::time::Duration;

#[test]
fn end_to_end() {
    let (_src_db_dir, src_db, _blocks) = tmp_db_with_random_content();
    let tgt_db_dir = TempPath::new();
    tgt_db_dir.create_as_dir().unwrap();
    let backup_dir = TempPath::new();
    backup_dir.create_as_dir().unwrap();
    let store: Arc<dyn BackupStorage> = Arc::new(LocalFs::new(backup_dir.path().to_path_buf()));

    let epoch = src_db
        .get_latest_ledger_info()
        .unwrap()
        .ledger_info()
        .next_block_epoch()
        - 1;
    let latest_epoch_ending_li = src_db
        .get_epoch_ending_ledger_infos(epoch, epoch + 1)
        .unwrap()
        .ledger_info_with_sigs
        .pop()
        .unwrap();
    let version = latest_epoch_ending_li.ledger_info().version();
    let state_root_hash = src_db
        .get_transactions(version, 1, version, false)
        .unwrap()
        .consume_transaction_list_with_proof()
        .proof
        .transaction_infos
        .pop()
        .unwrap()
        .state_checkpoint_hash()
        .unwrap();

    let (rt, port) = start_local_backup_service(src_db);
    let client = Arc::new(BackupServiceClient::new(format!(
        "http://localhost:{}",
        port
    )));
    let manifest_handle = rt
        .block_on(
            StateSnapshotBackupController::new(
                StateSnapshotBackupOpt { epoch },
                GlobalBackupOpt {
                    max_chunk_size: 500,
                    concurrent_data_requests: 2,
                },
                client,
                Arc::clone(&store),
            )
            .run(),
        )
        .unwrap();

    rt.block_on(
        StateSnapshotRestoreController::new(
            StateSnapshotRestoreOpt {
                manifest_handle,
                version,
                validate_modules: false,
                restore_mode: StateSnapshotRestoreMode::Default,
            },
            GlobalRestoreOpt {
                dry_run: false,
                db_dir: Some(tgt_db_dir.path().to_path_buf()),
                target_version: None, // max
                trusted_waypoints: TrustedWaypointOpt::default(),
                rocksdb_opt: RocksdbOpt::default(),
                concurrent_downloads: ConcurrentDownloadsOpt::default(),
                replay_concurrency_level: ReplayConcurrencyLevelOpt::default(),
                enable_state_indices: false,
            }
            .try_into()
            .unwrap(),
            store,
            None, /* epoch_history */
        )
        .run(),
    )
    .unwrap();

    let tgt_db = VelorDB::new_readonly_for_test(&tgt_db_dir);
    assert_eq!(
        tgt_db
            .get_state_snapshot_before(version + 1) // We cannot use get_latest_snapshot() because it searches backward from the latest txn_info version
            .unwrap()
            .unwrap(),
        (version, state_root_hash)
    );

    rt.shutdown_timeout(Duration::from_secs(1));
}

// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    backup_types::state_snapshot::{
        backup::{StateSnapshotBackupController, StateSnapshotBackupOpt},
        manifest::StateSnapshotBackup,
        restore::{StateSnapshotRestoreController, StateSnapshotRestoreOpt},
    },
    storage::{local_fs::LocalFs, BackupStorage, FileHandle},
    utils::{
        backup_service_client::BackupServiceClient,
        test_utils::{
            open_with_positions, start_local_backup_service, tmp_db_with_random_content,
            tmp_db_with_random_positions,
        },
        ConcurrentDownloadsOpt, GlobalBackupOpt, GlobalRestoreOpt, ReplayConcurrencyLevelOpt,
        RocksdbOpt, TrustedWaypointOpt,
    },
};
use aptos_db::{
    native_state_reader::NativeStateReader, state_restore::StateSnapshotRestoreMode, AptosDB,
};
use aptos_storage_interface::{DbReader, StateKind};
use aptos_temppath::TempPath;
use aptos_types::{
    account_address::AccountAddress,
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::Version,
};
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

    let tgt_db = AptosDB::new_readonly_for_test(&tgt_db_dir);
    assert_eq!(
        tgt_db
            .get_state_snapshot_before(version + 1) // We cannot use get_latest_snapshot() because it searches backward from the latest txn_info version
            .unwrap()
            .unwrap(),
        (version, state_root_hash)
    );

    rt.shutdown_timeout(Duration::from_secs(1));
}

/// Backs up the state snapshot at the latest epoch ending of `src_db` into
/// `backup_dir`, returning the manifest and the snapshot version.
fn backup(src_db: Arc<AptosDB>, backup_dir: &TempPath) -> (FileHandle, Version) {
    let epoch = src_db
        .get_latest_ledger_info()
        .unwrap()
        .ledger_info()
        .next_block_epoch()
        - 1;
    backup_at_epoch(src_db, backup_dir, epoch)
}

fn backup_at_epoch(
    src_db: Arc<AptosDB>,
    backup_dir: &TempPath,
    epoch: u64,
) -> (FileHandle, Version) {
    backup_at_epoch_with_chunk_size(src_db, backup_dir, epoch, 500)
}

fn backup_at_epoch_with_chunk_size(
    src_db: Arc<AptosDB>,
    backup_dir: &TempPath,
    epoch: u64,
    max_chunk_size: usize,
) -> (FileHandle, Version) {
    let store: Arc<dyn BackupStorage> = Arc::new(LocalFs::new(backup_dir.path().to_path_buf()));
    let version = src_db
        .get_epoch_ending_ledger_infos(epoch, epoch + 1)
        .unwrap()
        .ledger_info_with_sigs
        .pop()
        .unwrap()
        .ledger_info()
        .version();

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
                    max_chunk_size,
                    concurrent_data_requests: 2,
                },
                client,
                store,
            )
            .run(),
        )
        .unwrap();
    rt.shutdown_timeout(Duration::from_secs(1));
    (manifest_handle, version)
}

/// Restores the snapshot into `tgt_db_dir`, or verifies it when `None`.
fn restore(
    backup_dir: &TempPath,
    manifest_handle: FileHandle,
    version: Version,
    tgt_db_dir: Option<&TempPath>,
    rocksdb_opt: RocksdbOpt,
) -> anyhow::Result<()> {
    let store: Arc<dyn BackupStorage> = Arc::new(LocalFs::new(backup_dir.path().to_path_buf()));
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(
        StateSnapshotRestoreController::new(
            StateSnapshotRestoreOpt {
                manifest_handle,
                version,
                validate_modules: false,
                restore_mode: StateSnapshotRestoreMode::Default,
            },
            GlobalRestoreOpt {
                dry_run: tgt_db_dir.is_none(),
                db_dir: tgt_db_dir.map(|dir| dir.path().to_path_buf()),
                target_version: None, // max
                trusted_waypoints: TrustedWaypointOpt::default(),
                rocksdb_opt,
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
}

fn backup_and_restore(
    src_db: Arc<AptosDB>,
    tgt_db_dir: Option<&TempPath>,
    rocksdb_opt: RocksdbOpt,
) -> Version {
    let backup_dir = TempPath::new();
    backup_dir.create_as_dir().unwrap();
    let (manifest_handle, version) = backup(src_db, &backup_dir);
    restore(
        &backup_dir,
        manifest_handle,
        version,
        tgt_db_dir,
        rocksdb_opt,
    )
    .unwrap();
    version
}

fn positions_at(db: &AptosDB, version: Version) -> Vec<(StateKey, StateValue)> {
    db.get_state_value_chunk_iter(version, 0, usize::MAX, StateKind::Position)
        .unwrap()
        .collect::<aptos_storage_interface::Result<_>>()
        .unwrap()
}

#[test]
fn end_to_end_with_positions() {
    let (_src_db_dir, src_db, _blocks) = tmp_db_with_random_positions();
    let tgt_db_dir = TempPath::new();
    tgt_db_dir.create_as_dir().unwrap();

    let version = backup_and_restore(
        Arc::clone(&src_db),
        Some(&tgt_db_dir),
        RocksdbOpt::default_with_trading_native(),
    );

    let expected = positions_at(&src_db, version);
    assert!(!expected.is_empty(), "generated blocks write positions");
    let committed_root = src_db
        .get_transactions(version, 1, version, false)
        .unwrap()
        .consume_transaction_list_with_proof()
        .proof
        .transaction_infos
        .pop()
        .unwrap()
        .position_state_checkpoint_hash()
        .expect("checkpoint commits a position root");
    drop(src_db);

    // Reopening as a node runs the open-time root check against the ledger.
    let tgt_db = open_with_positions(tgt_db_dir.path());
    assert_eq!(positions_at(&tgt_db, version), expected);
    assert_eq!(
        tgt_db
            .get_pre_committed_position_state_summary()
            .unwrap()
            .latest()
            .root_hash(),
        committed_root
    );
    // The generated blocks write every position under exchange 0x1.
    let reader = tgt_db.native_state_reader().unwrap();
    assert_eq!(
        reader.count_positions_for_exchange(AccountAddress::ONE),
        expected.len()
    );
}

#[test]
fn verify_with_positions() {
    let (_src_db_dir, src_db, _blocks) = tmp_db_with_random_positions();
    backup_and_restore(src_db, None, RocksdbOpt::default());
}

#[test]
fn restore_without_position_storage_skips_positions() {
    let (_src_db_dir, src_db, _blocks) = tmp_db_with_random_positions();
    let tgt_db_dir = TempPath::new();
    tgt_db_dir.create_as_dir().unwrap();
    let version = backup_and_restore(src_db, Some(&tgt_db_dir), RocksdbOpt::default());
    let tgt_db = AptosDB::new_readonly_for_test(&tgt_db_dir);
    assert_eq!(
        tgt_db
            .get_state_snapshot_before(version + 1)
            .unwrap()
            .unwrap()
            .0,
        version
    );
}

/// A restore interrupted partway through the position chunks resumes from
/// where it stopped and ends with the same positions.
#[test]
fn position_restore_resumes() {
    let (_src_db_dir, src_db, _blocks) = tmp_db_with_random_positions();
    let backup_dir = TempPath::new();
    backup_dir.create_as_dir().unwrap();
    let epoch = src_db
        .get_latest_ledger_info()
        .unwrap()
        .ledger_info()
        .next_block_epoch()
        - 1;
    // Force one record per chunk so the interruption test always has a later chunk.
    let (manifest_handle, version) =
        backup_at_epoch_with_chunk_size(Arc::clone(&src_db), &backup_dir, epoch, 1);
    let expected = positions_at(&src_db, version);
    drop(src_db);

    let manifest: StateSnapshotBackup = serde_json::from_slice(
        &std::fs::read(backup_dir.path().join(manifest_handle.as_str())).unwrap(),
    )
    .unwrap();
    let position_chunks = &manifest.positions.as_ref().unwrap().chunks;
    assert!(position_chunks.len() >= 2, "need a chunk to fail on");
    let last_chunk = backup_dir
        .path()
        .join(position_chunks.last().unwrap().blobs.as_str());
    let stashed = std::fs::read(&last_chunk).unwrap();
    std::fs::remove_file(&last_chunk).unwrap();

    let tgt_db_dir = TempPath::new();
    tgt_db_dir.create_as_dir().unwrap();
    let rocksdb_opt = RocksdbOpt::default_with_trading_native;
    restore(
        &backup_dir,
        manifest_handle.clone(),
        version,
        Some(&tgt_db_dir),
        rocksdb_opt(),
    )
    .expect_err("missing position chunk");
    // The main tree is finished last, so a resumed run does not skip the
    // snapshot and leave positions half-restored.
    assert!(AptosDB::new_readonly_for_test(&tgt_db_dir)
        .get_state_snapshot_before(version + 1)
        .unwrap()
        .is_none());

    std::fs::write(&last_chunk, stashed).unwrap();
    restore(
        &backup_dir,
        manifest_handle,
        version,
        Some(&tgt_db_dir),
        rocksdb_opt(),
    )
    .unwrap();

    let tgt_db = open_with_positions(tgt_db_dir.path());
    assert_eq!(positions_at(&tgt_db, version), expected);
}

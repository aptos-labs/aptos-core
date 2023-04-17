// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::DBTool;
use aptos_backup_cli::{
    coordinators::backup::BackupCompactor,
    metadata,
    metadata::{cache::MetadataCacheOpt, view::MetadataView},
    storage::{local_fs::LocalFs, BackupStorage},
};
use aptos_backup_service::start_backup_service;
use aptos_executor_test_helpers::integration_test_impl::test_execution_with_storage_impl;
use aptos_temppath::TempPath;
use aptos_types::transaction::Version;
use clap::Parser;
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
    time::Duration,
};

#[test]
fn test_various_cmd_parsing() {
    run_cmd(&[
        "aptos-db-tool",
        "restore",
        "oneoff",
        "epoch-ending",
        "--epoch-ending-manifest",
        ".",
        "--local-fs-dir",
        ".",
        "--target-db-dir",
        ".",
    ]);
    run_cmd(&[
        "aptos-db-tool",
        "backup",
        "oneoff",
        "transaction",
        "--start-version",
        "100",
        "--num_transactions",
        "100",
        "--local-fs-dir",
        ".",
    ]);
    run_cmd(&[
        "aptos-db-tool",
        "backup",
        "continuously",
        "--local-fs-dir",
        ".",
    ]);
    run_cmd(&[
        "aptos-db-tool",
        "debug",
        "state-tree",
        "get-snapshots",
        "--db-dir",
        ".",
    ]);

    run_cmd(&["aptos-db-tool", "backup", "verify", "--local-fs-dir", "."]);
    run_cmd(&[
        "aptos-db-tool",
        "replay-verify",
        "--target-db-dir",
        ".",
        "--local-fs-dir",
        ".",
    ]);
    run_cmd(&[
        "aptos-db-tool",
        "backup",
        "verify",
        "--local-fs-dir",
        ".",
        "--start-version",
        "Max",
    ]);
}

fn run_cmd(args: &[&str]) {
    DBTool::try_parse_from(args).expect("command parse unsuccessful");
}

fn assert_metadata_view_eq(view1: &MetadataView, view2: &MetadataView) {
    assert!(
        view1.select_transaction_backups(0, Version::MAX).unwrap()
            == view2.select_transaction_backups(0, Version::MAX).unwrap()
            && view1.select_epoch_ending_backups(Version::MAX).unwrap()
                == view2.select_epoch_ending_backups(Version::MAX).unwrap()
            && view1.select_state_snapshot(Version::MAX).unwrap()
                == view2.select_state_snapshot(Version::MAX).unwrap(),
        "Metadata views are not equal"
    );
}

#[test]
fn test_backup_compaction() {
    let db = test_execution_with_storage_impl();
    let backup_dir = TempPath::new();
    backup_dir.create_as_dir().unwrap();
    let local_fs = LocalFs::new(backup_dir.path().to_path_buf());
    let store: Arc<dyn BackupStorage> = Arc::new(local_fs);
    let rt = start_backup_service(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 6186), db);

    // Backup the local_test DB
    rt.block_on(
        DBTool::try_parse_from([
            "aptos-db-tool",
            "backup",
            "oneoff",
            "epoch-ending",
            "--start-epoch",
            "0",
            "--end-epoch",
            "1",
            "--local-fs-dir",
            backup_dir.path().to_str().unwrap(),
        ])
        .unwrap()
        .run(),
    )
    .unwrap();

    rt.block_on(
        DBTool::try_parse_from([
            "aptos-db-tool",
            "backup",
            "oneoff",
            "epoch-ending",
            "--start-epoch",
            "1",
            "--end-epoch",
            "2",
            "--local-fs-dir",
            backup_dir.path().to_str().unwrap(),
        ])
        .unwrap()
        .run(),
    )
    .unwrap();

    rt.block_on(
        DBTool::try_parse_from([
            "aptos-db-tool",
            "backup",
            "oneoff",
            "state-snapshot",
            "--state-snapshot-epoch",
            "1",
            "--local-fs-dir",
            backup_dir.path().to_str().unwrap(),
        ])
        .unwrap()
        .run(),
    )
    .unwrap();

    rt.block_on(
        DBTool::try_parse_from([
            "aptos-db-tool",
            "backup",
            "oneoff",
            "state-snapshot",
            "--state-snapshot-epoch",
            "2",
            "--local-fs-dir",
            backup_dir.path().to_str().unwrap(),
        ])
        .unwrap()
        .run(),
    )
    .unwrap();
    rt.block_on(
        DBTool::try_parse_from([
            "aptos-db-tool",
            "backup",
            "oneoff",
            "transaction",
            "--start-version",
            "0",
            "--num_transactions",
            "15",
            "--local-fs-dir",
            backup_dir.path().to_str().unwrap(),
        ])
        .unwrap()
        .run(),
    )
    .unwrap();
    rt.block_on(
        DBTool::try_parse_from([
            "aptos-db-tool",
            "backup",
            "oneoff",
            "transaction",
            "--start-version",
            "15",
            "--num_transactions",
            "15",
            "--local-fs-dir",
            backup_dir.path().to_str().unwrap(),
        ])
        .unwrap()
        .run(),
    )
    .unwrap();
    // assert the metadata views are same before and after compaction
    let metadata_opt = MetadataCacheOpt::new(Some(TempPath::new().path().to_path_buf()));
    let old_metaview = rt
        .block_on(metadata::cache::sync_and_load(
            &metadata_opt,
            Arc::clone(&store),
            1,
        ))
        .unwrap();
    let compactor = BackupCompactor::new(2, 2, 2, metadata_opt.clone(), Arc::clone(&store), 1);
    rt.block_on(compactor.run()).unwrap();

    // run compaction again
    rt.block_on(compactor.run()).unwrap();

    let new_metaview = rt
        .block_on(metadata::cache::sync_and_load(
            &metadata_opt,
            Arc::clone(&store),
            1,
        ))
        .unwrap();

    assert_metadata_view_eq(&old_metaview, &new_metaview);
    rt.shutdown_timeout(Duration::from_secs(1));
}

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::DBTool;
use clap::Parser;

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

#[cfg(test)]
mod dbtool_tests {
    use crate::DBTool;
    use aptos_backup_cli::{
        coordinators::backup::BackupCompactor,
        metadata,
        metadata::{cache::MetadataCacheOpt, view::MetadataView},
        storage::{local_fs::LocalFs, BackupStorage},
        utils::test_utils::start_local_backup_service,
    };
    use aptos_db::AptosDB;
    use aptos_executor_test_helpers::integration_test_impl::{
        test_execution_with_storage_impl, test_execution_with_storage_impl_inner,
    };
    use aptos_storage_interface::DbReader;
    use aptos_temppath::TempPath;
    use aptos_types::{
        state_store::state_key::{inner::StateKeyTag::AccessPath, prefix::StateKeyPrefix},
        transaction::Version,
    };
    use clap::Parser;
    use std::{
        default::Default,
        fs,
        ops::Deref,
        path::{Path, PathBuf},
        sync::Arc,
        time::Duration,
    };
    use tokio::runtime::Runtime;

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
        let (rt, port) = start_local_backup_service(db);
        let server_addr = format!(" http://localhost:{}", port);

        // Backup the local_test DB
        rt.block_on(
            DBTool::try_parse_from([
                "aptos-db-tool",
                "backup",
                "oneoff",
                "--backup-service-address",
                server_addr.as_str(),
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
                "--backup-service-address",
                server_addr.as_str(),
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
                "--backup-service-address",
                server_addr.as_str(),
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
                "--backup-service-address",
                server_addr.as_str(),
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
                "--backup-service-address",
                server_addr.as_str(),
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
                "--backup-service-address",
                server_addr.as_str(),
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
        let metadata_cache_dir = TempPath::new();
        let metadata_opt = MetadataCacheOpt::new(Some(metadata_cache_dir.path().to_path_buf()));
        let old_metaview = rt
            .block_on(metadata::cache::sync_and_load(
                &metadata_opt,
                Arc::clone(&store),
                1,
            ))
            .unwrap();
        let og_list = rt.block_on(store.list_metadata_files()).unwrap();
        let compactor =
            BackupCompactor::new(2, 2, 2, metadata_opt.clone(), Arc::clone(&store), 1, 1);
        rt.block_on(compactor.run()).unwrap();
        // assert the original files are still present
        let mut after_list = rt.block_on(store.list_metadata_files()).unwrap();
        // remove any file matching str "compaction_timestamps" from after_list
        after_list.retain(|x| !x.contains("compaction_timestamps"));
        assert!(og_list.iter().all(|x| after_list.contains(x)));
        // wait 2 seconds to ensure the compaction waiting time expires
        std::thread::sleep(std::time::Duration::from_secs(2));
        // run the compaction again
        let compactor =
            BackupCompactor::new(2, 2, 2, metadata_opt.clone(), Arc::clone(&store), 1, 1);
        rt.block_on(compactor.run()).unwrap();
        let final_list = rt.block_on(store.list_metadata_files()).unwrap();
        // assert og list has no overlap with final list
        assert!(!og_list.iter().any(|x| final_list.contains(x)));

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

    #[cfg(test)]
    fn db_restore_test_setup(
        start: Version,
        end: Version,
        backup_dir: PathBuf,
        old_db_dir: PathBuf,
        new_db_dir: PathBuf,
    ) -> (Runtime, String) {
        use aptos_config::config::{
            RocksdbConfigs, StorageDirPaths, BUFFERED_STATE_TARGET_ITEMS_FOR_TEST,
            NO_OP_STORAGE_PRUNER_CONFIG,
        };
        use aptos_db_indexer::utils::PrefixedStateValueIterator as IndexerPrefixedStateValueIterator;
        use aptos_indexer_grpc_table_info::internal_indexer_db_service::InternalIndexerDBService;
        let db = test_execution_with_storage_impl_inner(false, old_db_dir.as_path());
        let (rt, port) = start_local_backup_service(Arc::clone(&db));
        let server_addr = format!(" http://localhost:{}", port);
        // Backup the local_test DB
        rt.block_on(
            DBTool::try_parse_from([
                "aptos-db-tool",
                "backup",
                "oneoff",
                "--backup-service-address",
                server_addr.as_str(),
                "epoch-ending",
                "--start-epoch",
                "0",
                "--end-epoch",
                "1",
                "--local-fs-dir",
                backup_dir.as_path().to_str().unwrap(),
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
                "--backup-service-address",
                server_addr.as_str(),
                "epoch-ending",
                "--start-epoch",
                "1",
                "--end-epoch",
                "2",
                "--local-fs-dir",
                backup_dir.as_path().to_str().unwrap(),
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
                "--backup-service-address",
                server_addr.as_str(),
                "state-snapshot",
                "--state-snapshot-epoch",
                "0",
                "--local-fs-dir",
                backup_dir.as_path().to_str().unwrap(),
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
                "--backup-service-address",
                server_addr.as_str(),
                "state-snapshot",
                "--state-snapshot-epoch",
                "1",
                "--local-fs-dir",
                backup_dir.as_path().to_str().unwrap(),
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
                "--backup-service-address",
                server_addr.as_str(),
                "state-snapshot",
                "--state-snapshot-epoch",
                "2",
                "--local-fs-dir",
                backup_dir.as_path().to_str().unwrap(),
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
                "--backup-service-address",
                server_addr.as_str(),
                "transaction",
                "--start-version",
                "0",
                "--num_transactions",
                "15",
                "--local-fs-dir",
                backup_dir.as_path().to_str().unwrap(),
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
                "--backup-service-address",
                server_addr.as_str(),
                "transaction",
                "--start-version",
                "15",
                "--num_transactions",
                "15",
                "--local-fs-dir",
                backup_dir.as_path().to_str().unwrap(),
            ])
            .unwrap()
            .run(),
        )
        .unwrap();

        let start_string = format!("{}", start);
        let end_string = format!("{}", end);
        let mut restore_args = vec![
            "aptos-db-tool".to_string(),
            "restore".to_string(),
            "bootstrap-db".to_string(),
            "--ledger-history-start-version".to_string(),
            start_string, // use start_string here
            "--target-version".to_string(),
            end_string, // use end_string here
            "--target-db-dir".to_string(),
            new_db_dir.as_path().to_str().unwrap().to_string(),
            "--local-fs-dir".to_string(),
            backup_dir.as_path().to_str().unwrap().to_string(),
        ];
        let additional_args = vec!["--enable-storage-sharding", "--enable-state-indices"]
            .into_iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();
        restore_args.extend(additional_args);
        rt.block_on(DBTool::try_parse_from(restore_args).unwrap().run())
            .unwrap();

        // assert the kv are the same in db and new_db
        // current all the kv are still stored in the ledger db
        //

        let internal_indexer_db =
            InternalIndexerDBService::get_indexer_db_for_restore(new_db_dir.as_path()).unwrap();

        let aptos_db: Arc<dyn DbReader> = Arc::new(
            AptosDB::open(
                StorageDirPaths::from_path(new_db_dir),
                false,
                NO_OP_STORAGE_PRUNER_CONFIG,
                RocksdbConfigs::default(),
                false,
                BUFFERED_STATE_TARGET_ITEMS_FOR_TEST,
                1000,
                Some(internal_indexer_db.clone()),
            )
            .unwrap(),
        );

        // Only state key at and by the snapshot version are restored in internal indexer
        let snapshot_version = if start == 0 {
            0
        } else if start > 0 && start < 15 {
            1
        } else {
            15
        };

        let new_iter = IndexerPrefixedStateValueIterator::new(
            aptos_db.clone(),
            internal_indexer_db.get_inner_db_ref(),
            StateKeyPrefix::new(AccessPath, b"".to_vec()),
            None,
            snapshot_version,
        )
        .unwrap();

        let old_iter = db
            .deref()
            .get_prefixed_state_value_iterator(
                &StateKeyPrefix::new(AccessPath, b"".to_vec()),
                None,
                snapshot_version,
            )
            .unwrap();

        // collect all the keys in the new_iter
        let mut new_keys = new_iter.map(|e| e.unwrap().0).collect::<Vec<_>>();
        new_keys.sort();
        let mut old_keys = old_iter.map(|e| e.unwrap().0).collect::<Vec<_>>();
        old_keys.sort();
        assert_eq!(new_keys, old_keys);

        let ledger_version = aptos_db.get_latest_ledger_info_version().unwrap();
        for ver in start..=ledger_version {
            let old_block_res = db.get_block_info_by_version(ver);
            let new_block_res = aptos_db.get_block_info_by_version(ver);
            let (old_block_version, old_block_height, _) = old_block_res.unwrap();
            let (new_block_version, new_block_height, _) = new_block_res.unwrap();
            assert_eq!(old_block_version, new_block_version);
            assert_eq!(old_block_height, new_block_height);
        }

        (rt, server_addr)
    }
    #[test]
    fn test_restore_db_with_replay() {
        let backup_dir = TempPath::new();
        backup_dir.create_as_dir().unwrap();
        let new_db_dir = TempPath::new();
        let old_db_dir = TempPath::new();
        // Test the basic db boostrap that replays from previous snapshot to the target version
        let (rt, _) = db_restore_test_setup(
            16,
            16,
            PathBuf::from(backup_dir.path()),
            PathBuf::from(old_db_dir.path()),
            PathBuf::from(new_db_dir.path()),
        );
        let backup_size = dir_size(backup_dir.path());
        let db_size = dir_size(new_db_dir.path());
        let old_db_size = dir_size(old_db_dir.path());
        println!(
            "backup size: {}, old db size: {}, new db size: {}",
            backup_size, old_db_size, db_size
        );

        rt.shutdown_timeout(Duration::from_secs(1));
    }
    #[test]
    fn test_restore_archive_db() {
        let backup_dir = TempPath::new();
        backup_dir.create_as_dir().unwrap();
        let new_db_dir = TempPath::new();
        let old_db_dir = TempPath::new();
        // Test the db boostrap in some historical range with all the kvs restored
        let (rt, _) = db_restore_test_setup(
            1,
            16,
            PathBuf::from(backup_dir.path()),
            PathBuf::from(old_db_dir.path()),
            PathBuf::from(new_db_dir.path()),
        );
        rt.shutdown_timeout(Duration::from_secs(1));
    }

    #[test]
    fn test_resume_db_from_kv_replay() {
        let backup_dir = TempPath::new();
        backup_dir.create_as_dir().unwrap();
        let new_db_dir = TempPath::new();
        new_db_dir.create_as_dir().unwrap();
        let old_db_dir = TempPath::new();
        // Test the basic db boostrap that replays from previous snapshot to the target version
        let (rt, _) = db_restore_test_setup(
            1,
            16,
            PathBuf::from(backup_dir.path()),
            PathBuf::from(old_db_dir.path()),
            PathBuf::from(new_db_dir.path()),
        );
        // boostrap a historical DB starting from version 1 to version 18
        // This only replays the txn from txn 17 to 18
        rt.block_on(
            DBTool::try_parse_from([
                "aptos-db-tool",
                "restore",
                "bootstrap-db",
                "--ledger-history-start-version",
                "1",
                "--target-version",
                "18",
                "--target-db-dir",
                new_db_dir.path().to_str().unwrap(),
                "--local-fs-dir",
                backup_dir.path().to_str().unwrap(),
            ])
            .unwrap()
            .run(),
        )
        .unwrap();
        rt.shutdown_timeout(Duration::from_secs(1));
    }

    fn dir_size<P: AsRef<Path>>(path: P) -> u64 {
        let mut size = 0;

        for entry in fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            let metadata = entry.metadata().unwrap();

            if metadata.is_dir() {
                size += dir_size(entry.path());
            } else {
                size += metadata.len();
            }
        }

        size
    }
}

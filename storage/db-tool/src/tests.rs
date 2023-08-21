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
    use aptos_config::config::RocksdbConfigs;
    use aptos_db::AptosDB;
    use aptos_executor_test_helpers::integration_test_impl::{
        test_execution_with_storage_impl, test_execution_with_storage_impl_inner,
    };
    use aptos_temppath::TempPath;
    use aptos_types::{
        state_store::{state_key::StateKeyTag::AccessPath, state_key_prefix::StateKeyPrefix},
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
        force_sharding: bool,
    ) -> (Runtime, String) {
        use aptos_db::utils::iterators::PrefixedStateValueIterator;
        use aptos_storage_interface::DbReader;
        use itertools::zip_eq;

        let db = test_execution_with_storage_impl_inner(force_sharding, old_db_dir.as_path());
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
        if force_sharding {
            let additional_args = vec!["--split-ledger-db", "--use-sharded-state-merkle-db"]
                .into_iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>();
            restore_args.extend(additional_args);
        }
        rt.block_on(DBTool::try_parse_from(restore_args).unwrap().run())
            .unwrap();

        // verify the new DB has the same data as the original DB
        let db_config = if !force_sharding {
            RocksdbConfigs::default()
        } else {
            RocksdbConfigs {
                use_sharded_state_merkle_db: true,
                split_ledger_db: true,
                ..Default::default()
            }
        };
        let (_ledger_db, tree_db, state_kv_db) =
            AptosDB::open_dbs(new_db_dir, db_config, false, 0).unwrap();

        // assert the kv are the same in db and new_db
        // current all the kv are still stored in the ledger db
        //
        for ver in start..=end {
            let new_iter = PrefixedStateValueIterator::new(
                &state_kv_db,
                StateKeyPrefix::new(AccessPath, b"".to_vec()),
                None,
                ver,
                force_sharding,
            )
            .unwrap();
            let old_iter = db
                .deref()
                .get_prefixed_state_value_iterator(
                    &StateKeyPrefix::new(AccessPath, b"".to_vec()),
                    None,
                    ver,
                )
                .unwrap();

            zip_eq(new_iter, old_iter).for_each(|(new, old)| {
                let (new_key, new_value) = new.unwrap();
                let (old_key, old_value) = old.unwrap();
                assert_eq!(new_key, old_key);
                assert_eq!(new_value, old_value);
            });
        }
        // first snapshot tree not recovered
        assert!(
            tree_db.get_root_hash(0).is_err() || tree_db.get_leaf_count(0).unwrap() == 0,
            "tree at version 0 should not be restored"
        );
        // second snapshot tree recovered
        let second_snapshot_version: Version = 13;
        assert!(
            tree_db.get_root_hash(second_snapshot_version).is_ok(),
            "root hash at version {} doesn't exist",
            second_snapshot_version,
        );
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
            false,
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
            false,
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
            false,
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

    #[test]
    fn test_restore_with_sharded_db() {
        let backup_dir = TempPath::new();
        backup_dir.create_as_dir().unwrap();
        let new_db_dir = TempPath::new();
        let old_db_dir = TempPath::new();

        let (rt, _) = db_restore_test_setup(
            16,
            16,
            PathBuf::from(backup_dir.path()),
            PathBuf::from(old_db_dir.path()),
            PathBuf::from(new_db_dir.path()),
            true,
        );
        let backup_size = dir_size(backup_dir.path());
        let db_size = dir_size(new_db_dir.path());
        let old_db_size = dir_size(old_db_dir.path());
        println!(
            "backup size: {}, old db size: {}, new db size: {}",
            backup_size, old_db_size, db_size
        );

        println!(
            "backup size: {:?}, old db size: {:?}, new db size: {:?}",
            backup_dir.path(),
            old_db_dir.path(),
            new_db_dir.path()
        );
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

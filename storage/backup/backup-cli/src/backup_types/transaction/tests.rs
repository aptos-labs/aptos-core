// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    backup_types::transaction::{
        backup::{TransactionBackupController, TransactionBackupOpt},
        restore::TransactionRestoreBatchController,
    },
    storage::{local_fs::LocalFs, BackupStorage},
    utils::{
        backup_service_client::BackupServiceClient,
        test_utils::{start_local_backup_service, tmp_db_with_random_content},
        ConcurrentDownloadsOpt, GlobalBackupOpt, GlobalRestoreOpt, ReplayConcurrencyLevelOpt,
        RocksdbOpt, TrustedWaypointOpt,
    },
};
use aptos_db::AptosDB;
use aptos_executor_types::VerifyExecutionMode;
use aptos_storage_interface::DbReader;
use aptos_temppath::TempPath;
use aptos_types::transaction::Version;
use itertools::zip_eq;
use std::{convert::TryInto, mem::size_of, sync::Arc};
use tokio::time::Duration;

#[test]
fn end_to_end() {
    let (_src_db_dir, src_db, blocks) = tmp_db_with_random_content();
    let tgt_db_dir = TempPath::new();
    tgt_db_dir.create_as_dir().unwrap();
    let backup_dir = TempPath::new();
    backup_dir.create_as_dir().unwrap();
    let store: Arc<dyn BackupStorage> = Arc::new(LocalFs::new(backup_dir.path().to_path_buf()));

    let (rt, port) = start_local_backup_service(Arc::clone(&src_db));
    let client = Arc::new(BackupServiceClient::new(format!(
        "http://localhost:{}",
        port
    )));

    let latest_version = blocks.last().unwrap().1.ledger_info().version();
    let total_txns = blocks.iter().fold(0, |x, b| x + b.0.len());
    assert_eq!(latest_version as usize + 1, total_txns);
    let txns = blocks
        .iter()
        .flat_map(|(txns, _li)| txns)
        .map(|txn_to_commit| txn_to_commit.transaction())
        .collect::<Vec<_>>();
    let max_chunk_size = txns
        .iter()
        .map(|t| bcs::serialized_size(t).unwrap())
        .max()
        .unwrap() // biggest txn
        + 115 // size of a serialized TransactionInfo
        + size_of::<u32>(); // record len header
    let first_ver_to_backup = (total_txns / 4) as Version;
    let num_txns_to_backup = total_txns - first_ver_to_backup as usize;
    let target_version = first_ver_to_backup + total_txns as Version / 2;
    let mut backup_handles = vec![];
    if first_ver_to_backup > 0 {
        let transaction_backup_before_first_ver = rt
            .block_on(
                TransactionBackupController::new(
                    TransactionBackupOpt {
                        start_version: 0,
                        num_transactions: first_ver_to_backup as usize,
                    },
                    GlobalBackupOpt {
                        max_chunk_size,
                        concurrent_data_requests: 2,
                    },
                    client.clone(),
                    Arc::clone(&store),
                )
                .run(),
            )
            .unwrap();
        backup_handles.push(transaction_backup_before_first_ver);
    }

    let transaction_backup_after_first_ver = rt
        .block_on(
            TransactionBackupController::new(
                TransactionBackupOpt {
                    start_version: first_ver_to_backup,
                    num_transactions: num_txns_to_backup,
                },
                GlobalBackupOpt {
                    max_chunk_size,
                    concurrent_data_requests: 2,
                },
                client,
                Arc::clone(&store),
            )
            .run(),
        )
        .unwrap();
    backup_handles.push(transaction_backup_after_first_ver);
    rt.block_on(
        TransactionRestoreBatchController::new(
            GlobalRestoreOpt {
                dry_run: false,
                db_dir: Some(tgt_db_dir.path().to_path_buf()),
                target_version: Some(target_version),
                trusted_waypoints: TrustedWaypointOpt::default(),
                rocksdb_opt: RocksdbOpt::default(),
                concurrent_downloads: ConcurrentDownloadsOpt::default(),
                replay_concurrency_level: ReplayConcurrencyLevelOpt::default(),
                enable_state_indices: false,
            }
            .try_into()
            .unwrap(),
            store,
            backup_handles,
            None,
            None,
            None,
            VerifyExecutionMode::verify_all(),
            None,
        )
        .run(),
    )
    .unwrap();
    // We don't write down any ledger infos when recovering transactions. State-sync needs to take
    // care of it before running consensus. The latest transactions are deemed "synced" instead of
    // "committed" most likely.
    let tgt_db = AptosDB::new_readonly_for_test(&tgt_db_dir);
    let ouptputlist = tgt_db
        .get_transaction_outputs(0, target_version, target_version)
        .unwrap();

    for (restore_ws, org_ws) in zip_eq(
        ouptputlist
            .transactions_and_outputs
            .iter()
            .map(|(_, output)| output.write_set().clone()),
        blocks
            .iter()
            .flat_map(|(txns, _li)| txns)
            .take(target_version as usize)
            .map(|txn_to_commit| txn_to_commit.write_set().clone()),
    ) {
        assert_eq!(restore_ws, org_ws);
    }

    assert_eq!(tgt_db.expect_synced_version(), target_version);
    let recovered_transactions = tgt_db
        .get_transactions(
            0,
            target_version,
            target_version,
            true, /* fetch_events */
        )
        .unwrap();

    assert_eq!(
        recovered_transactions.transactions,
        txns.into_iter()
            .take(target_version as usize)
            .cloned()
            .collect::<Vec<_>>()
    );

    assert_eq!(
        recovered_transactions.events.unwrap(),
        blocks
            .iter()
            .flat_map(|(txns, _li)| {
                txns.iter()
                    .map(|txn_to_commit| txn_to_commit.events().to_vec())
            })
            .take(target_version as usize)
            .collect::<Vec<_>>()
    );

    rt.shutdown_timeout(Duration::from_secs(1));
}

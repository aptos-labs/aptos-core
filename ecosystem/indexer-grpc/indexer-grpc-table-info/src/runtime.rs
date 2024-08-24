// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    backup_restore::{fs_ops::rename_db_folders_and_cleanup, gcs::GcsBackupRestoreOperator},
    table_info_service::TableInfoService,
};
use anyhow::Error;
use aptos_api::context::Context;
use aptos_config::config::NodeConfig;
use aptos_db_indexer::{
    db_indexer::{DBIndexer, InternalIndexerDB},
    db_ops::open_db,
    db_v2::IndexerAsyncV2,
};
use aptos_mempool::MempoolClientSender;
use aptos_schemadb::DB;
use aptos_storage_interface::DbReaderWriter;
use aptos_types::chain_id::ChainId;
use std::{
    sync::Arc,
    thread::sleep,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::runtime::Runtime;

const INDEX_ASYNC_V2_DB_NAME: &str = "index_indexer_async_v2_db";
/// if last restore timestamp is less than RESTORE_TIME_DIFF_SECS, do not restore to avoid gcs download spam
const RESTORE_TIME_DIFF_SECS: u64 = 600;
const DB_OPERATION_INTERVAL_MS: u64 = 500;

pub fn bootstrap_internal_indexer_db(
    config: &NodeConfig,
    db_rw: DbReaderWriter,
    internal_indexer_db: Option<InternalIndexerDB>,
) -> Option<(Runtime, Arc<DBIndexer>)> {
    if !config.indexer_db_config.is_internal_indexer_db_enabled() || internal_indexer_db.is_none() {
        return None;
    }
    let runtime = aptos_runtimes::spawn_named_runtime("index-db".to_string(), None);
    // Set up db config and open up the db initially to read metadata
    let mut indexer_service =
        InternalIndexerDBService::new(db_rw.reader, internal_indexer_db.unwrap());
    let db_indexer = indexer_service.get_db_indexer();
    // Spawn task for db indexer
    let config_clone = config.to_owned();
    runtime.spawn(async move {
        indexer_service.run(&config_clone).await.unwrap();
    });

    Some((runtime, db_indexer))
}

/// Creates a runtime which creates a thread pool which sets up fullnode indexer table info service
/// Returns corresponding Tokio runtime
pub fn bootstrap(
    config: &NodeConfig,
    chain_id: ChainId,
    db_rw: DbReaderWriter,
    mp_sender: MempoolClientSender,
) -> Option<(Runtime, Arc<IndexerAsyncV2>)> {
    if !config.indexer_table_info.enabled {
        return None;
    }

    let runtime = aptos_runtimes::spawn_named_runtime("table-info".to_string(), None);

    // Set up db config and open up the db initially to read metadata
    let node_config = config.clone();
    let db_path = node_config
        .storage
        .get_dir_paths()
        .default_root_path()
        .join(INDEX_ASYNC_V2_DB_NAME);
    let rocksdb_config = node_config.storage.rocksdb_configs.index_db_config;
    let db =
        open_db(db_path, &rocksdb_config).expect("Failed to open up indexer async v2 db initially");

    let table_info_gcs_bucket_name = node_config.indexer_table_info.gcs_bucket_name.clone();
    let indexer_async_v2 =
        Arc::new(IndexerAsyncV2::new(db).expect("Failed to initialize indexer async v2"));
    let indexer_async_v2_clone = Arc::clone(&indexer_async_v2);

    // Spawn the runtime for table info parsing
    runtime.spawn(async move {
        let backup_restore_operator: Arc<GcsBackupRestoreOperator> =
            Arc::new(GcsBackupRestoreOperator::new(table_info_gcs_bucket_name).await);
        let context = Arc::new(Context::new(
            chain_id,
            db_rw.reader.clone(),
            mp_sender,
            node_config.clone(),
            None,
            None,
        ));
        // DB backup is optional
        let backup_restore_operator = if node_config.indexer_table_info.db_backup_enabled {
            Some(backup_restore_operator)
        } else {
            None
        };

        let mut parser = TableInfoService::new(
            context,
            next_version,
            node_config.indexer_table_info.parser_task_count,
            node_config.indexer_table_info.parser_batch_size,
            node_config.indexer_table_info.enable_expensive_logging,
            backup_restore_operator,
            indexer_async_v2_clone,
        );

        parser.run().await;
    });

    Some((runtime, indexer_async_v2))
}

/// This function handles the conditional restoration of the database from a GCS snapshot.
/// It checks if the database needs to be restored based on metadata file existence
/// and metadata epoch and the time since the last restore and the version differences.
/// If a restore is needed, it:
/// 1. close the db
/// 2. performs the restore to a different folder
/// 3. rename the folder to atomically move restored db snapshot to the right db path
/// 4. re-open the db
/// 5. update the last restore timestamp in the restored db
/// If a restore is not needed, it:
/// 1. returns the original db
async fn handle_db_restore(
    node_config: &NodeConfig,
    chain_id: ChainId,
    db: DB,
    db_rw: DbReaderWriter,
    version_diff: u64,
    rocksdb_config: RocksdbConfig,
) -> Result<DB, Error> {
    let binding = node_config.storage.get_dir_paths();
    let db_root_path = binding.default_root_path();
    let db_path = db_root_path.join(INDEX_ASYNC_V2_DB_NAME);
    // Set up backup and restore config
    let gcs_bucket_name = node_config.indexer_table_info.gcs_bucket_name.clone();
    let named_chain = match NamedChain::from_chain_id(&chain_id) {
        Ok(named_chain) => format!("{}", named_chain).to_lowercase(),
        Err(_err) => {
            info!("Getting chain name from not named chains");
            chain_id.id().to_string()
        },
    };

    let backup_restore_operator: Arc<GcsBackupRestoreOperator> = Arc::new(
        GcsBackupRestoreOperator::new(format!("{}-{}", gcs_bucket_name.clone(), &named_chain))
            .await,
    );

    // If there's no metadata json file in gcs, we will create a default one with epoch 0 and return early since there's no snapshot to restore from, and early return.
    // If metadata epoch is 0, early return.
    let metadata = backup_restore_operator.get_metadata().await;
    if metadata.is_none() {
        backup_restore_operator
            .create_default_metadata_if_absent(chain_id.id() as u64)
            .await
            .expect("Failed to create default metadata");
        return Ok(db);
    } else if metadata.unwrap().epoch == 0 {
        return Ok(db);
    }

    // Check the time duration since the last restore
    let last_restored_timestamp = read_db::<MetadataKey, MetadataValue, IndexerMetadataSchema>(
        &db,
        &MetadataKey::RestoreTimestamp,
    )
    .unwrap()
    .map_or(0, |v| v.last_restored_timestamp());
    // Current timestamp will be used to compare duration from last restored timestamp, and to save db if restore is performed
    let current_timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    assert!(
        current_timestamp >= last_restored_timestamp,
        "Last restored timestamp from db should be less or equal to the current timestamp"
    );
    let should_restore_based_on_time_duration =
        current_timestamp - last_restored_timestamp > RESTORE_TIME_DIFF_SECS;

    // Check the version difference
    let latest_committed_version = db_rw.reader.get_latest_version().unwrap();
    let next_version = read_db::<MetadataKey, MetadataValue, IndexerMetadataSchema>(
        &db,
        &MetadataKey::LatestVersion,
    )
    .unwrap()
    .map_or(0, |v| v.expect_version());
    let should_restore_based_on_version = latest_committed_version > next_version
        && latest_committed_version - next_version > version_diff;

    if should_restore_based_on_time_duration && should_restore_based_on_version {
        // after reading db metadata info and deciding to restore, drop the db so that we could re-open it later
        close_db(db);

        sleep(Duration::from_millis(DB_OPERATION_INTERVAL_MS));

        backup_restore_operator
            .verify_storage_bucket_existence()
            .await;

        // a different path to restore backup db snapshot to, to avoid db corruption
        let restore_db_path = node_config
            .storage
            .get_dir_paths()
            .default_root_path()
            .join("restore");
        backup_restore_operator
            .restore_db_snapshot(
                chain_id.id() as u64,
                metadata.unwrap(),
                restore_db_path.clone(),
                node_config.get_data_dir().to_path_buf(),
            )
            .await
            .expect("Failed to restore snapshot");

        // Restore to a different folder and replace the target folder atomically
        let tmp_db_path = db_root_path.join("tmp");
        rename_db_folders_and_cleanup(&db_path, &tmp_db_path, &restore_db_path)
            .expect("Failed to operate atomic restore in file system.");

        sleep(Duration::from_millis(DB_OPERATION_INTERVAL_MS));

        let db = open_db(&db_path, &rocksdb_config).expect("Failed to reopen db after restore");
        write_db::<MetadataKey, MetadataValue, IndexerMetadataSchema>(
            &db,
            MetadataKey::RestoreTimestamp,
            MetadataValue::Timestamp(current_timestamp),
        )
        .expect("Failed to write restore timestamp to indexer async v2");

        info!(
            should_restore_based_on_time_duration = should_restore_based_on_time_duration,
            should_restore_based_on_version = should_restore_based_on_version,
            latest_committed_version = latest_committed_version,
            db_next_version = next_version,
            last_restored_timestamp = last_restored_timestamp,
            "[Table Info] Table info restored successfully"
        );
        return Ok(db);
    }
    Ok(db)
}

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{backup_restore::gcs::GcsBackupRestoreOperator, table_info_service::TableInfoService};
use aptos_api::context::Context;
use aptos_config::config::NodeConfig;
use aptos_logger::info;
use aptos_mempool::MempoolClientSender;
use aptos_storage_interface::DbReaderWriter;
use aptos_types::chain_id::{ChainId, NamedChain};
use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::runtime::Runtime;

const INDEX_ASYNC_V2_DB_NAME: &str = "index_indexer_async_v2_db";
/// if last restore timestamp is less than RESTORE_TIME_DIFF_SECS, do not restore to avoid gcs read spam
const RESTORE_TIME_DIFF_SECS: u64 = 600;

/// Creates a runtime which creates a thread pool which sets up fullnode indexer table info service
/// Returns corresponding Tokio runtime
pub fn bootstrap(
    config: &NodeConfig,
    chain_id: ChainId,
    db: DbReaderWriter,
    mp_sender: MempoolClientSender,
) -> Option<Runtime> {
    if !config.indexer_table_info.enabled {
        return None;
    }

    let runtime = aptos_runtimes::spawn_named_runtime("table-info".to_string(), None);

    let node_config = config.clone();
    let parser_task_count = node_config.indexer_table_info.parser_task_count;
    let parser_batch_size = node_config.indexer_table_info.parser_batch_size;
    let enable_expensive_logging = node_config.indexer_table_info.enable_expensive_logging;
    let next_version = db.reader.get_indexer_async_v2_next_version().unwrap();
    let db_backup_enabled = node_config.indexer_table_info.db_backup_enabled.clone();
    let version_diff = node_config.indexer_table_info.version_diff.clone();

    // Set up backup and restore config
    let gcs_bucket_name = node_config.indexer_table_info.gcs_bucket_name.clone();
    let named_chain = match NamedChain::from_chain_id(&chain_id) {
        Ok(named_chain) => format!("{}", named_chain).to_lowercase(),
        Err(_err) => {
            info!("Getting chain name from not named chains");
            chain_id.id().to_string()
        },
    };

    // Before runtime's spawned, conditionally restore db snapshot from gcs
    runtime.block_on(async {
        let backup_restore_operator: Arc<GcsBackupRestoreOperator> = Arc::new(
            GcsBackupRestoreOperator::new(gcs_bucket_name.clone() + "-" + &named_chain).await,
        );
        let latest_committed_version = db.reader.get_latest_version().unwrap();
        let current_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Check the time duration since the last restore
        let last_restored_timestamp = db.reader.get_indexer_async_v2_restore_timestamp().unwrap();
        assert!(
            current_timestamp >= last_restored_timestamp,
            "Last restored timestamp from db should be less or equal to the current timestamp"
        );
        let should_restore_based_on_time_duration = current_timestamp
            - db.reader.get_indexer_async_v2_restore_timestamp().unwrap()
            > RESTORE_TIME_DIFF_SECS;

        // Check the version difference
        let should_restore_based_on_version = latest_committed_version > next_version
            && latest_committed_version - next_version > version_diff;

        if should_restore_based_on_time_duration || should_restore_based_on_version {
            backup_restore_operator
                .verify_storage_bucket_existence()
                .await;
            // the indexer async v2 db file path to take snapshot from
            let db_path = node_config
                .storage
                .get_dir_paths()
                .default_root_path()
                .join(INDEX_ASYNC_V2_DB_NAME);
            let base_path = node_config.get_data_dir().to_path_buf();
            backup_restore_operator
                .restore_snapshot(chain_id.id() as u64, db_path.clone(), base_path.clone())
                .await
                .expect("Failed to restore snapshot");
            db.writer
                .clone()
                .update_last_restored_timestamp(current_timestamp)
                .expect("Failed to update last restored timestamp");
        }

        info!(
            should_restore_based_on_time_duration = should_restore_based_on_time_duration,
            should_restore_based_on_version = should_restore_based_on_version,
            latest_committed_version = latest_committed_version,
            db_next_version = next_version,
            last_restored_timestamp = last_restored_timestamp,
            "[Table Info] Table info conditional restore successfully"
        );
    });

    // Spawn the runtime for table info parsing
    runtime.spawn(async move {
        // Read the new next version after db restore
        let next_version = db.reader.get_indexer_async_v2_next_version().unwrap();
        let context = Arc::new(Context::new(
            chain_id,
            db.reader.clone(),
            mp_sender,
            node_config,
        ));
        // Backing up rocksdb is optional
        let backup_restore_operator: Arc<GcsBackupRestoreOperator> = Arc::new(
            GcsBackupRestoreOperator::new(gcs_bucket_name.clone() + "-" + &named_chain).await,
        );
        let backup_restore_operator = if db_backup_enabled {
            Some(backup_restore_operator)
        } else {
            None
        };
        let mut parser = TableInfoService::new(
            context,
            next_version,
            parser_task_count,
            parser_batch_size,
            enable_expensive_logging,
            backup_restore_operator,
        );
        parser.run(db.clone()).await
    });
    Some(runtime)
}

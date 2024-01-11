// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{table_info_service::TableInfoService, backup_restore::gcs::GcsBackupRestoreOperator};
use aptos_api::context::Context;
use aptos_config::config::NodeConfig;
use aptos_logger::info;
use aptos_mempool::MempoolClientSender;
use aptos_storage_interface::DbReaderWriter;
use aptos_types::chain_id::{ChainId, NamedChain};
use std::sync::Arc;
use tokio::runtime::Runtime;

const INDEX_ASYNC_V2_DB_NAME: &str = "index_indexer_async_v2_db";

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

    // Set up backup and restore
    let gcs_bucket_name = node_config.indexer_table_info.gcs_bucket_name.clone();
    let named_chain =
        match NamedChain::from_chain_id(&chain_id) {
            Ok(named_chain) => format!("{}", named_chain).to_lowercase(),
            Err(_err) => {
                info!("Getting chain name from not named chains");
                chain_id.id().to_string()
            },
        };
    let backup_restore_operator: Arc<GcsBackupRestoreOperator> =
        Arc::new(GcsBackupRestoreOperator::new(gcs_bucket_name.clone() + "-" + &named_chain));

    runtime.block_on(async {
        backup_restore_operator
            .verify_storage_bucket_existence()
            .await;
        let db_path = node_config
            .storage
            .get_dir_paths()
            .default_root_path()
            .join(INDEX_ASYNC_V2_DB_NAME);
        backup_restore_operator
            .restore_snapshot(chain_id.id() as u64, db_path.clone())
            .await
            .expect("Failed to restore snapshot");
        let _metadata = backup_restore_operator
            .create_default_metadata_if_absent(chain_id.id().into())
            .await;
    });

    // Spawn the runtime for table info parsing
    runtime.spawn(async move {
        let context = Arc::new(Context::new(
            chain_id,
            db.reader.clone(),
            mp_sender,
            node_config,
        ));
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

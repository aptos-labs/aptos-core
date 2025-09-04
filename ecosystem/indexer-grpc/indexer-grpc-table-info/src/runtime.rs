// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    backup_restore::gcs::GcsBackupRestoreOperator,
    internal_indexer_db_service::InternalIndexerDBService, table_info_service::TableInfoService,
};
use velor_api::context::Context;
use velor_config::config::{NodeConfig, TableInfoServiceMode};
use velor_db_indexer::{
    db_indexer::{DBIndexer, InternalIndexerDB},
    db_ops::open_db,
    db_v2::IndexerAsyncV2,
};
use velor_mempool::MempoolClientSender;
use velor_storage_interface::DbReaderWriter;
use velor_types::{chain_id::ChainId, transaction::Version};
use std::{sync::Arc, time::Instant};
use tokio::{runtime::Runtime, sync::watch::Receiver as WatchReceiver};

const INDEX_ASYNC_V2_DB_NAME: &str = "index_indexer_async_v2_db";

pub fn bootstrap_internal_indexer_db(
    config: &NodeConfig,
    db_rw: DbReaderWriter,
    internal_indexer_db: Option<InternalIndexerDB>,
    update_receiver: Option<WatchReceiver<(Instant, Version)>>,
) -> Option<(Runtime, Arc<DBIndexer>)> {
    if !config.indexer_db_config.is_internal_indexer_db_enabled() || internal_indexer_db.is_none() {
        return None;
    }
    let runtime = velor_runtimes::spawn_named_runtime("index-db".to_string(), None);
    // Set up db config and open up the db initially to read metadata
    let mut indexer_service = InternalIndexerDBService::new(
        db_rw.reader,
        internal_indexer_db.unwrap(),
        update_receiver.expect("Internal indexer db update receiver is missing"),
    );
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
    if !config
        .indexer_table_info
        .table_info_service_mode
        .is_enabled()
    {
        return None;
    }

    let runtime = velor_runtimes::spawn_named_runtime("table-info".to_string(), None);

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

    let indexer_async_v2 =
        Arc::new(IndexerAsyncV2::new(db).expect("Failed to initialize indexer async v2"));
    let indexer_async_v2_clone = Arc::clone(&indexer_async_v2);

    // Spawn the runtime for table info parsing
    runtime.spawn(async move {
        let context = Arc::new(Context::new(
            chain_id,
            db_rw.reader.clone(),
            mp_sender,
            node_config.clone(),
            None,
        ));

        // DB backup is optional
        let backup_restore_operator = match node_config.indexer_table_info.table_info_service_mode {
            TableInfoServiceMode::Backup(gcs_bucket_name) => Some(Arc::new(
                GcsBackupRestoreOperator::new(gcs_bucket_name).await,
            )),
            _ => None,
        };

        let mut parser = TableInfoService::new(
            context,
            indexer_async_v2_clone.next_version(),
            node_config.indexer_table_info.parser_task_count,
            node_config.indexer_table_info.parser_batch_size,
            backup_restore_operator,
            indexer_async_v2_clone,
        );

        parser.run().await;
    });

    Some((runtime, indexer_async_v2))
}

// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    db_indexer::{DBIndexer, InternalIndexerDB},
    db_ops::open_db,
    db_v2::IndexerAsyncV2,
    internal_indexer_db_service::InternalIndexerDBService,
    table_info_service::TableInfoService,
};
use aptos_config::config::NodeConfig;
use aptos_storage_interface::DbReaderWriter;
use aptos_types::transaction::Version;
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
    let runtime = aptos_runtimes::spawn_named_runtime(
        "index-db".to_string(),
        config.indexer_db_config.runtime_threads,
    );
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

/// Creates a runtime which runs the table info service, parsing table info from committed
/// write sets. Returns the corresponding Tokio runtime.
pub fn bootstrap_table_info(
    config: &NodeConfig,
    db_rw: DbReaderWriter,
) -> Option<(Runtime, Arc<IndexerAsyncV2>)> {
    if !config
        .indexer_table_info
        .table_info_service_mode
        .is_enabled()
    {
        return None;
    }

    let runtime = aptos_runtimes::spawn_named_runtime("table-info".to_string(), None);

    // Set up db config and open up the db initially to read metadata
    let db_path = config
        .storage
        .get_dir_paths()
        .default_root_path()
        .join(INDEX_ASYNC_V2_DB_NAME);
    let rocksdb_config = config.storage.rocksdb_configs.index_db_config;
    let db = open_db(db_path, &rocksdb_config, /*readonly=*/ false)
        .expect("Failed to open up indexer async v2 db initially");

    let indexer_async_v2 =
        Arc::new(IndexerAsyncV2::new(db).expect("Failed to initialize indexer async v2"));
    let indexer_async_v2_clone = Arc::clone(&indexer_async_v2);

    let parser_task_count = config.indexer_table_info.parser_task_count;
    let parser_batch_size = config.indexer_table_info.parser_batch_size;

    // Spawn the runtime for table info parsing
    runtime.spawn(async move {
        let parser = TableInfoService::new(
            db_rw.reader,
            indexer_async_v2_clone.next_version(),
            parser_task_count,
            parser_batch_size,
            indexer_async_v2_clone,
        );
        parser.run().await;
    });

    Some((runtime, indexer_async_v2))
}

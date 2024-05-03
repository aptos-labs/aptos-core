// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_config::config::NodeConfig;
use aptos_db_indexer::{db_indexer::DBIndexer, db_ops::open_internal_indexer_db};
use aptos_indexer_grpc_utils::counters::{log_grpc_step, IndexerGrpcStep};
use aptos_storage_interface::DbReader;
use std::sync::Arc;

const SERVICE_TYPE: &str = "internal_indexer_db_service";
const INTERNAL_INDEXER_DB: &str = "internal_indexer_db";

pub struct InternalIndexerDBService {
    pub db_indexer: Arc<DBIndexer>,
}

impl InternalIndexerDBService {
    pub fn new(db_reader: Arc<dyn DbReader>, node_config: &NodeConfig) -> Self {
        let db_path = node_config
            .storage
            .get_dir_paths()
            .default_root_path()
            .join(INTERNAL_INDEXER_DB);
        let rocksdb_config = node_config.storage.rocksdb_configs.index_db_config;
        let db = Arc::new(
            open_internal_indexer_db(db_path, &rocksdb_config)
                .expect("Failed to open up indexer db initially"),
        );

        let internal_db_indexer = Arc::new(DBIndexer::new(
            db,
            db_reader,
            &node_config.indexer_db_config,
        ));
        Self {
            db_indexer: internal_db_indexer,
        }
    }

    pub fn get_db_indexer(&self) -> Arc<DBIndexer> {
        Arc::clone(&self.db_indexer)
    }

    pub async fn run(&mut self) {
        let mut start_version = self.db_indexer.get_persisted_version().unwrap_or(0);
        loop {
            let start_time: std::time::Instant = std::time::Instant::now();
            let next_version = self
                .db_indexer
                .process_a_batch(Some(start_version))
                .expect("Failed to run internal db indexer");

            if next_version == start_version {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                continue;
            }
            log_grpc_step(
                SERVICE_TYPE,
                IndexerGrpcStep::InternalIndexerDBProcessed,
                Some(start_version as i64),
                Some(next_version as i64),
                None,
                None,
                Some(start_time.elapsed().as_secs_f64()),
                None,
                Some((next_version - start_version) as i64),
                None,
            );
            start_version = next_version;
        }
    }
}

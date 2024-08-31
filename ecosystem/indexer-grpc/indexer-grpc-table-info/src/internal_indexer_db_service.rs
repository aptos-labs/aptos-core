// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_config::config::{internal_indexer_db_config::InternalIndexerDBConfig, NodeConfig};
use aptos_db_indexer::{
    db_indexer::{DBIndexer, InternalIndexerDB},
    db_ops::open_internal_indexer_db,
    indexer_reader::IndexerReaders,
};
use aptos_indexer_grpc_utils::counters::{log_grpc_step, IndexerGrpcStep};
use aptos_storage_interface::DbReader;
use aptos_types::{indexer::indexer_db_reader::IndexerReader, transaction::Version};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::runtime::Handle;

const SERVICE_TYPE: &str = "internal_indexer_db_service";
const INTERNAL_INDEXER_DB: &str = "internal_indexer_db";

pub struct InternalIndexerDBService {
    pub db_indexer: Arc<DBIndexer>,
}

impl InternalIndexerDBService {
    pub fn new(db_reader: Arc<dyn DbReader>, internal_indexer_db: InternalIndexerDB) -> Self {
        let internal_db_indexer = Arc::new(DBIndexer::new(internal_indexer_db, db_reader));
        Self {
            db_indexer: internal_db_indexer,
        }
    }

    pub fn get_indexer_db_for_restore(db_dir: &Path) -> Option<InternalIndexerDB> {
        let db_path_buf = PathBuf::from(db_dir).join(INTERNAL_INDEXER_DB);
        let rocksdb_config = NodeConfig::default()
            .storage
            .rocksdb_configs
            .index_db_config;
        let arc_db = Arc::new(
            open_internal_indexer_db(db_path_buf.as_path(), &rocksdb_config)
                .expect("Failed to open internal indexer db"),
        );

        let internal_indexer_db_config = InternalIndexerDBConfig::new(false, false, true, 10_000);
        Some(InternalIndexerDB::new(arc_db, internal_indexer_db_config))
    }

    pub fn get_indexer_db(node_config: &NodeConfig) -> Option<InternalIndexerDB> {
        if !node_config
            .indexer_db_config
            .is_internal_indexer_db_enabled()
        {
            return None;
        }
        let db_path_buf = node_config
            .storage
            .get_dir_paths()
            .default_root_path()
            .join(INTERNAL_INDEXER_DB);
        let rocksdb_config = node_config.storage.rocksdb_configs.index_db_config;
        let db_path = db_path_buf.as_path();

        let arc_db = Arc::new(
            open_internal_indexer_db(db_path, &rocksdb_config)
                .expect("Failed to open internal indexer db"),
        );
        Some(InternalIndexerDB::new(
            arc_db,
            node_config.indexer_db_config,
        ))
    }

    pub fn get_db_indexer(&self) -> Arc<DBIndexer> {
        Arc::clone(&self.db_indexer)
    }

    pub async fn get_start_version(&self, node_config: &NodeConfig) -> Result<Version> {
        let fast_sync_enabled = node_config
            .state_sync
            .state_sync_driver
            .bootstrapping_mode
            .is_fast_sync();
        let mut main_db_synced_version = self.db_indexer.main_db_reader.ensure_synced_version()?;

        // Wait till fast sync is done
        while fast_sync_enabled && main_db_synced_version == 0 {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            main_db_synced_version = self.db_indexer.main_db_reader.ensure_synced_version()?;
        }

        let start_version = self
            .db_indexer
            .indexer_db
            .get_persisted_version()?
            .map_or(0, |v| v + 1);

        if node_config.indexer_db_config.enable_statekeys() {
            let state_start_version = self
                .db_indexer
                .indexer_db
                .get_state_version()?
                .map_or(0, |v| v + 1);
            if start_version != state_start_version {
                panic!("Cannot start state indexer because the progress doesn't match.");
            }
        }

        if node_config.indexer_db_config.enable_transaction() {
            let transaction_start_version = self
                .db_indexer
                .indexer_db
                .get_transaction_version()?
                .map_or(0, |v| v + 1);
            if start_version != transaction_start_version {
                panic!("Cannot start transaction indexer because the progress doesn't match.");
            }
        }

        if node_config.indexer_db_config.enable_event() {
            let event_start_version = self
                .db_indexer
                .indexer_db
                .get_event_version()?
                .map_or(0, |v| v + 1);
            if start_version != event_start_version {
                panic!("Cannot start event indexer because the progress doesn't match.");
            }
        }

        Ok(start_version)
    }

    pub async fn run(&mut self, node_config: &NodeConfig) -> Result<()> {
        let mut start_version = self.get_start_version(node_config).await?;

        loop {
            let start_time: std::time::Instant = std::time::Instant::now();
            let next_version = self.db_indexer.process_a_batch(start_version)?;

            if next_version == start_version {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
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

pub struct MockInternalIndexerDBService {
    pub indexer_readers: Option<IndexerReaders>,
    pub _handle: Option<Handle>,
}

impl MockInternalIndexerDBService {
    pub fn new_for_test(db_reader: Arc<dyn DbReader>, node_config: &NodeConfig) -> Self {
        if !node_config
            .indexer_db_config
            .is_internal_indexer_db_enabled()
        {
            return Self {
                indexer_readers: None,
                _handle: None,
            };
        }

        let db = InternalIndexerDBService::get_indexer_db(node_config).unwrap();
        let handle = Handle::current();
        let mut internal_indexer_db_service = InternalIndexerDBService::new(db_reader, db);
        let db_indexer = internal_indexer_db_service.get_db_indexer();
        let config_clone = node_config.to_owned();
        handle.spawn(async move {
            internal_indexer_db_service
                .run(&config_clone)
                .await
                .unwrap();
        });
        Self {
            indexer_readers: IndexerReaders::new(None, Some(db_indexer)),
            _handle: Some(handle),
        }
    }

    pub fn get_indexer_reader(&self) -> Option<Arc<dyn IndexerReader>> {
        if let Some(indexer_reader) = &self.indexer_readers {
            return Some(Arc::new(indexer_reader.to_owned()));
        }
        None
    }
}

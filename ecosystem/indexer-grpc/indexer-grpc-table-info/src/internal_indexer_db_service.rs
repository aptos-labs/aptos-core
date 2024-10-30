// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::INDEXER_DB_LATENCY;
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
    time::Instant,
};
use tokio::{runtime::Handle, sync::watch::Receiver as WatchReceiver};

const SERVICE_TYPE: &str = "internal_indexer_db_service";
const INTERNAL_INDEXER_DB: &str = "internal_indexer_db";

pub struct InternalIndexerDBService {
    pub db_indexer: Arc<DBIndexer>,
    pub update_receiver: WatchReceiver<(Instant, Version)>,
}

impl InternalIndexerDBService {
    pub fn new(
        db_reader: Arc<dyn DbReader>,
        internal_indexer_db: InternalIndexerDB,
        update_receiver: WatchReceiver<(Instant, Version)>,
    ) -> Self {
        let internal_db_indexer = Arc::new(DBIndexer::new(internal_indexer_db, db_reader));
        Self {
            db_indexer: internal_db_indexer,
            update_receiver,
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

        // TODO: Enable transaction summaries here after the feature is complete
        let internal_indexer_db_config =
            InternalIndexerDBConfig::new(true, true, true, 0, true, 10_000);
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

        if node_config.indexer_db_config.enable_event_v2_translation() {
            let event_v2_translation_start_version = self
                .db_indexer
                .indexer_db
                .get_event_v2_translation_version()?
                .map_or(0, |v| v + 1);
            if node_config
                .indexer_db_config
                .event_v2_translation_ignores_below_version()
                < start_version
                && start_version != event_v2_translation_start_version
            {
                panic!(
                    "Cannot start event v2 translation indexer because the progress doesn't match. \
                    start_version: {}, event_v2_translation_start_version: {}",
                    start_version, event_v2_translation_start_version
                );
            }
            if !node_config.indexer_db_config.enable_event() {
                panic!("Cannot start event v2 translation indexer because event indexer is not enabled.");
            }
        }

        Ok(start_version)
    }

    pub async fn run(&mut self, node_config: &NodeConfig) -> Result<()> {
        let mut start_version = self.get_start_version(node_config).await?;
        let mut target_version = self.db_indexer.main_db_reader.ensure_synced_version()?;
        let mut step_timer = std::time::Instant::now();

        loop {
            if target_version <= start_version {
                match self.update_receiver.changed().await {
                    Ok(_) => {
                        (step_timer, target_version) = *self.update_receiver.borrow();
                    },
                    Err(e) => {
                        panic!("Failed to get update from update_receiver: {}", e);
                    },
                }
            }
            let next_version = self.db_indexer.process(start_version, target_version)?;
            INDEXER_DB_LATENCY.set(step_timer.elapsed().as_millis() as i64);
            log_grpc_step(
                SERVICE_TYPE,
                IndexerGrpcStep::InternalIndexerDBProcessed,
                Some(start_version as i64),
                Some(next_version as i64),
                None,
                None,
                Some(step_timer.elapsed().as_secs_f64()),
                None,
                Some((next_version - start_version) as i64),
                None,
            );
            start_version = next_version;
        }
    }

    // For internal testing
    pub async fn run_with_end_version(
        &mut self,
        node_config: &NodeConfig,
        end_version: Option<Version>,
    ) -> Result<()> {
        let start_version = self.get_start_version(node_config).await?;
        let end_version = end_version.unwrap_or(std::u64::MAX);
        let mut next_version = start_version;
        while next_version < end_version {
            next_version = self.db_indexer.process(next_version, end_version)?;
            // We shouldn't stop the internal indexer so that internal indexer can catch up with the main DB
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
        Ok(())
    }
}

pub struct MockInternalIndexerDBService {
    pub indexer_readers: Option<IndexerReaders>,
    pub _handle: Option<Handle>,
}

impl MockInternalIndexerDBService {
    pub fn new_for_test(
        db_reader: Arc<dyn DbReader>,
        node_config: &NodeConfig,
        update_receiver: WatchReceiver<(Instant, Version)>,
        end_version: Option<Version>,
    ) -> Self {
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
        let mut internal_indexer_db_service =
            InternalIndexerDBService::new(db_reader, db, update_receiver);
        let db_indexer = internal_indexer_db_service.get_db_indexer();
        let config_clone = node_config.to_owned();
        handle.spawn(async move {
            internal_indexer_db_service
                .run_with_end_version(&config_clone, end_version)
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

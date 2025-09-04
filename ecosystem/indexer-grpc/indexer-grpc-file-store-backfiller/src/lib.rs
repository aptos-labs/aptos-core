// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod processor;

use anyhow::Result;
use velor_indexer_grpc_server_framework::RunnableConfig;
use velor_indexer_grpc_utils::config::IndexerGrpcFileStoreConfig;
use processor::Processor;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IndexerGrpcFileStoreBackfillerConfig {
    pub file_store_config: IndexerGrpcFileStoreConfig,
    pub fullnode_grpc_address: Url,
    pub progress_file_path: String,
    pub enable_expensive_logging: Option<bool>,
    pub chain_id: u64,
    #[serde(default = "default_enable_cache_compression")]
    pub enable_cache_compression: bool,
    pub starting_version: Option<u64>,
    pub transactions_count: Option<u64>,
    #[serde(default = "default_validation_mode")]
    pub validation_mode: bool,
    #[serde(default = "default_backfill_processing_task_count")]
    pub backfill_processing_task_count: usize,
    #[serde(default = "default_validating_task_count")]
    pub validating_task_count: usize,
}

const fn default_enable_cache_compression() -> bool {
    true
}

const fn default_validation_mode() -> bool {
    false
}

const fn default_backfill_processing_task_count() -> usize {
    20
}

const fn default_validating_task_count() -> usize {
    50
}

#[async_trait::async_trait]
impl RunnableConfig for IndexerGrpcFileStoreBackfillerConfig {
    async fn run(&self) -> Result<()> {
        let mut processor = Processor::new(
            self.fullnode_grpc_address.clone(),
            self.file_store_config.clone(),
            self.chain_id,
            self.enable_cache_compression,
            self.progress_file_path.clone(),
            self.starting_version,
            self.transactions_count,
            self.validation_mode,
            self.backfill_processing_task_count,
            self.validating_task_count,
        )
        .await
        .expect("Failed to create file store processor");
        processor
            .run()
            .await
            .expect("File store processor exited unexpectedly");
        Ok(())
    }

    fn get_server_name(&self) -> String {
        "idxfilestore".to_string()
    }
}

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
pub struct IndexerGrpcV2FileStoreBackfillerConfig {
    pub file_store_config: IndexerGrpcFileStoreConfig,
    pub fullnode_grpc_address: Url,
    pub progress_file_path: String,
    pub chain_id: u64,
    pub starting_version: u64,
    pub ending_version: u64,
    #[serde(default = "default_backfill_processing_task_count")]
    pub backfill_processing_task_count: usize,
}

const fn default_backfill_processing_task_count() -> usize {
    16
}

#[async_trait::async_trait]
impl RunnableConfig for IndexerGrpcV2FileStoreBackfillerConfig {
    async fn run(&self) -> Result<()> {
        let processor = Processor::new(
            self.fullnode_grpc_address.clone(),
            self.file_store_config.clone(),
            self.chain_id,
            self.progress_file_path.clone(),
            self.starting_version,
            self.ending_version,
            self.backfill_processing_task_count,
        )
        .await
        .expect("Failed to create file store backfill processor.");
        processor
            .run()
            .await
            .expect("File store backfill processor exited unexpectedly.");
        Ok(())
    }

    fn get_server_name(&self) -> String {
        "backfill".to_string()
    }
}

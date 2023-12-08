// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod metrics;
pub mod processor;

use anyhow::{Context, Result};
use aptos_indexer_grpc_server_framework::RunnableConfig;
use aptos_indexer_grpc_utils::{config::IndexerGrpcFileStoreConfig, types::RedisUrl};
use processor::Processor;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IndexerGrpcFileStoreWorkerConfig {
    pub file_store_config: IndexerGrpcFileStoreConfig,
    pub redis_main_instance_address: RedisUrl,
}

#[async_trait::async_trait]
impl RunnableConfig for IndexerGrpcFileStoreWorkerConfig {
    async fn run(&self) -> Result<()> {
        let mut processor = Processor::new(
            self.redis_main_instance_address.clone(),
            self.file_store_config.clone(),
        )
        .await
        .context("Failed to create processor for file store worker")?;
        processor
            .run()
            .await
            .expect("File store processor exited unexpectedly");
        Err(anyhow::anyhow!("File store processor exited unexpectedly"))
    }

    fn get_server_name(&self) -> String {
        "idxfilestore".to_string()
    }
}

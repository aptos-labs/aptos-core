// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod metrics;
pub mod processor;

use anyhow::Result;
use velor_indexer_grpc_server_framework::RunnableConfig;
use velor_indexer_grpc_utils::{config::IndexerGrpcFileStoreConfig, types::RedisUrl};
use processor::Processor;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IndexerGrpcFileStoreWorkerConfig {
    pub file_store_config: IndexerGrpcFileStoreConfig,
    pub redis_main_instance_address: RedisUrl,
    pub enable_expensive_logging: Option<bool>,
    pub chain_id: u64,
    #[serde(default = "default_enable_cache_compression")]
    pub enable_cache_compression: bool,
}

const fn default_enable_cache_compression() -> bool {
    false
}

impl IndexerGrpcFileStoreWorkerConfig {
    pub fn new(
        file_store_config: IndexerGrpcFileStoreConfig,
        redis_main_instance_address: RedisUrl,
        enable_expensive_logging: Option<bool>,
        chain_id: u64,
        enable_cache_compression: bool,
    ) -> Self {
        Self {
            file_store_config,
            redis_main_instance_address,
            enable_expensive_logging,
            chain_id,
            enable_cache_compression,
        }
    }
}

#[async_trait::async_trait]
impl RunnableConfig for IndexerGrpcFileStoreWorkerConfig {
    async fn run(&self) -> Result<()> {
        let mut processor = Processor::new(
            self.redis_main_instance_address.clone(),
            self.file_store_config.clone(),
            self.chain_id,
            self.enable_cache_compression,
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

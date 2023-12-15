// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod metrics;
pub mod worker;

use anyhow::Result;
use aptos_indexer_grpc_server_framework::RunnableConfig;
use aptos_indexer_grpc_utils::{
    config::IndexerGrpcFileStoreConfig, storage_format::StorageFormat, types::RedisUrl,
};
use serde::{Deserialize, Serialize};
use worker::Worker;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IndexerGrpcFileStoreWorkerConfig {
    pub file_store_config: IndexerGrpcFileStoreConfig,
    pub redis_main_instance_address: RedisUrl,
    pub enable_expensive_logging: Option<bool>,
    pub chain_id: u64,
    #[serde(default = "default_cacche_storage_format")]
    pub cache_storage_format: StorageFormat,
}

fn default_cacche_storage_format() -> StorageFormat {
    StorageFormat::Base64UncompressedProto
}

impl IndexerGrpcFileStoreWorkerConfig {
    pub fn new(
        file_store_config: IndexerGrpcFileStoreConfig,
        redis_main_instance_address: RedisUrl,
        enable_expensive_logging: Option<bool>,
        chain_id: u64,
        cache_storage_format: StorageFormat,
    ) -> Self {
        Self {
            file_store_config,
            redis_main_instance_address,
            enable_expensive_logging,
            chain_id,
            cache_storage_format,
        }
    }
}

#[async_trait::async_trait]
impl RunnableConfig for IndexerGrpcFileStoreWorkerConfig {
    async fn run(&self) -> Result<()> {
        Worker::run(
            self.redis_main_instance_address.clone(),
            self.file_store_config.clone(),
            self.enable_expensive_logging.unwrap_or(false),
            self.chain_id,
            self.cache_storage_format,
        )
        .await
        .expect("File store processor exited unexpectedly");
        Err(anyhow::anyhow!("File store processor exited unexpectedly"))
    }

    fn get_server_name(&self) -> String {
        "idxfilestore".to_string()
    }
}

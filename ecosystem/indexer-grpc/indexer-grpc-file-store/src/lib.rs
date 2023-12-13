// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod metrics;
pub mod processor;

use anyhow::{Context, Result};
use aptos_indexer_grpc_server_framework::RunnableConfig;
use aptos_indexer_grpc_utils::{
    config::IndexerGrpcFileStoreConfig, storage_format::StorageFormat, types::RedisUrl,
};
use processor::Processor;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IndexerGrpcFileStoreWorkerConfig {
    pub file_store_config: IndexerGrpcFileStoreConfig,
    pub redis_main_instance_address: RedisUrl,
    pub enable_verbose_logging: Option<bool>,
    #[serde(default = "default_storage_format")]
    pub storage_format: StorageFormat,
    #[serde(default = "default_cache_storage_format")]
    pub cache_storage_format: StorageFormat,
}

impl IndexerGrpcFileStoreWorkerConfig {
    pub fn new(
        file_store_config: IndexerGrpcFileStoreConfig,
        redis_main_instance_address: RedisUrl,
        enable_verbose_logging: Option<bool>,
        storage_format: StorageFormat,
        cache_storage_format: StorageFormat,
    ) -> Self {
        Self {
            file_store_config,
            redis_main_instance_address,
            enable_verbose_logging,
            storage_format,
            cache_storage_format,
        }
    }
}

fn default_storage_format() -> StorageFormat {
    StorageFormat::JsonBase64UncompressedProto
}

fn default_cache_storage_format() -> StorageFormat {
    StorageFormat::Base64UncompressedProto
}

#[async_trait::async_trait]
impl RunnableConfig for IndexerGrpcFileStoreWorkerConfig {
    async fn run(&self) -> Result<()> {
        let mut processor = Processor::new(
            self.redis_main_instance_address.clone(),
            self.file_store_config.clone(),
            self.enable_verbose_logging.unwrap_or(false),
            self.storage_format,
            self.cache_storage_format,
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

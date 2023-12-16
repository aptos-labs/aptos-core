// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod metrics;
pub mod worker;

use anyhow::{Context, Result};
use aptos_indexer_grpc_server_framework::RunnableConfig;
use aptos_indexer_grpc_utils::{
    config::IndexerGrpcFileStoreConfig, storage_format::StorageFormat, types::RedisUrl,
};
use serde::{Deserialize, Serialize};
use url::Url;
use worker::Worker;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IndexerGrpcCacheWorkerConfig {
    pub fullnode_grpc_address: Url,
    pub file_store_config: IndexerGrpcFileStoreConfig,
    pub redis_main_instance_address: RedisUrl,
    #[serde(default = "default_storage_format")]
    pub storage_format: StorageFormat,
}

fn default_storage_format() -> StorageFormat {
    StorageFormat::Base64UncompressedProto
}

impl IndexerGrpcCacheWorkerConfig {
    pub fn new(
        fullnode_grpc_address: Url,
        file_store_config: IndexerGrpcFileStoreConfig,
        redis_main_instance_address: RedisUrl,
        storage_format: StorageFormat,
    ) -> Self {
        Self {
            fullnode_grpc_address,
            file_store_config,
            redis_main_instance_address,
            storage_format,
        }
    }
}

#[async_trait::async_trait]
impl RunnableConfig for IndexerGrpcCacheWorkerConfig {
    async fn run(&self) -> Result<()> {
        let storage_format = self.storage_format;
        let mut worker = Worker::new(
            self.fullnode_grpc_address.clone(),
            self.redis_main_instance_address.clone(),
            self.file_store_config.clone(),
            storage_format,
        )
        .await
        .context("Failed to create cache worker")?;
        worker.run().await?;
        Ok(())
    }

    fn get_server_name(&self) -> String {
        "idxcachewrkr".to_string()
    }
}

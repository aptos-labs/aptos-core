// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod metrics;
pub mod worker;

use anyhow::{Context, Result};
use aptos_indexer_grpc_server_framework::RunnableConfig;
use aptos_indexer_grpc_utils::{config::IndexerGrpcFileStoreConfig, types::RedisUrl};
use serde::{Deserialize, Serialize};
use url::Url;
use worker::Worker;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IndexerGrpcCacheWorkerConfig {
    pub fullnode_grpc_address: Url,
    pub file_store_config: IndexerGrpcFileStoreConfig,
    pub redis_main_instance_address: RedisUrl,
    pub enable_verbose_logging: bool,
}

impl IndexerGrpcCacheWorkerConfig {
    pub fn new(
        fullnode_grpc_address: Url,
        file_store_config: IndexerGrpcFileStoreConfig,
        redis_main_instance_address: RedisUrl,
        enable_verbose_logging: Option<bool>,
    ) -> Self {
        Self {
            fullnode_grpc_address,
            file_store_config,
            redis_main_instance_address,
            enable_verbose_logging: enable_verbose_logging.unwrap_or(false),
        }
    }
}

#[async_trait::async_trait]
impl RunnableConfig for IndexerGrpcCacheWorkerConfig {
    async fn run(&self) -> Result<()> {
        let mut worker = Worker::new(
            self.fullnode_grpc_address.clone(),
            self.redis_main_instance_address.clone(),
            self.file_store_config.clone(),
            self.enable_verbose_logging,
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

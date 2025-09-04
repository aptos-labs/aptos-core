// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod metrics;
pub mod worker;

use anyhow::{Context, Result};
use velor_indexer_grpc_server_framework::RunnableConfig;
use velor_indexer_grpc_utils::{config::IndexerGrpcFileStoreConfig, types::RedisUrl};
use serde::{Deserialize, Serialize};
use url::Url;
use worker::Worker;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IndexerGrpcCacheWorkerConfig {
    pub fullnode_grpc_address: Url,
    pub file_store_config: IndexerGrpcFileStoreConfig,
    pub redis_main_instance_address: RedisUrl,
    #[serde(default = "default_enable_cache_compression")]
    pub enable_cache_compression: bool,
}

const fn default_enable_cache_compression() -> bool {
    false
}

impl IndexerGrpcCacheWorkerConfig {
    pub fn new(
        fullnode_grpc_address: Url,
        file_store_config: IndexerGrpcFileStoreConfig,
        redis_main_instance_address: RedisUrl,
        enable_cache_compression: bool,
    ) -> Self {
        Self {
            fullnode_grpc_address,
            file_store_config,
            redis_main_instance_address,
            enable_cache_compression,
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
            self.enable_cache_compression,
        )
        .await
        .context("Failed to create cache worker")?;
        worker
            .run()
            .await
            .context("Failed to run cache worker")
            .expect("Cache worker failed");
        Ok(())
    }

    fn get_server_name(&self) -> String {
        "idxcachewrkr".to_string()
    }
}

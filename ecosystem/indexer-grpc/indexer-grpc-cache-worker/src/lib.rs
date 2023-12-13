// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod metrics;
pub mod worker;

use anyhow::{Context, Result};
use aptos_indexer_grpc_server_framework::RunnableConfig;
use aptos_indexer_grpc_utils::{
    cache_operator::CacheOperator, config::IndexerGrpcFileStoreConfig,
    create_grpc_client_with_retry, types::RedisUrl,
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
        // If the worker ends unexpected, i.e., `run` finishes, restart the worker.
        loop {
            let file_store_operator = self.file_store_config.create();
            let redis_client = redis::Client::open(self.redis_main_instance_address.0.clone())
                .with_context(|| {
                    format!(
                        "Failed to create redis client for {}",
                        self.redis_main_instance_address
                    )
                })?;
            let conn = redis_client
                .get_tokio_connection_manager()
                .await
                .context("Get redis connection failed.")?;
            let cache_operator = CacheOperator::new(conn);
            let fullnode_grpc_client =
                create_grpc_client_with_retry(self.fullnode_grpc_address.clone()).await?;
            let mut worker = Worker::new(
                cache_operator,
                file_store_operator,
                fullnode_grpc_client,
                self.enable_verbose_logging,
            );
            worker.run().await?;
        }
    }

    fn get_server_name(&self) -> String {
        "idxcachewrkr".to_string()
    }
}

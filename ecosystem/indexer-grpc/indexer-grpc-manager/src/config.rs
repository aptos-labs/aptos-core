// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::grpc_manager::GrpcManager;
use anyhow::Result;
use velor_indexer_grpc_server_framework::RunnableConfig;
use velor_indexer_grpc_utils::config::IndexerGrpcFileStoreConfig;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tokio::sync::OnceCell;
use warp::{reply::Response, Rejection};

pub(crate) static GRPC_MANAGER: OnceCell<GrpcManager> = OnceCell::const_new();

pub(crate) const MAX_MESSAGE_SIZE: usize = 256 * (1 << 20);
pub(crate) type GrpcAddress = String;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct ServiceConfig {
    pub(crate) listen_address: SocketAddr,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct CacheConfig {
    pub(crate) max_cache_size: usize,
    pub(crate) target_cache_size: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IndexerGrpcManagerConfig {
    pub(crate) chain_id: u64,
    pub(crate) service_config: ServiceConfig,
    #[serde(default = "default_cache_config")]
    pub(crate) cache_config: CacheConfig,
    pub(crate) file_store_config: IndexerGrpcFileStoreConfig,
    pub(crate) self_advertised_address: GrpcAddress,
    pub(crate) grpc_manager_addresses: Vec<GrpcAddress>,
    pub(crate) fullnode_addresses: Vec<GrpcAddress>,
    pub(crate) is_master: bool,
}

const fn default_cache_config() -> CacheConfig {
    CacheConfig {
        max_cache_size: 5 * (1 << 30),
        target_cache_size: 4 * (1 << 30),
    }
}

#[async_trait::async_trait]
impl RunnableConfig for IndexerGrpcManagerConfig {
    async fn run(&self) -> Result<()> {
        GRPC_MANAGER
            .get_or_init(|| async { GrpcManager::new(self).await })
            .await
            .start(&self.service_config)
    }

    fn get_server_name(&self) -> String {
        "grpc_manager".to_string()
    }

    async fn status_page(&self) -> Result<Response, Rejection> {
        crate::status_page::status_page().await
    }
}

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::gateway::GrpcGateway;
use aptos_indexer_grpc_server_framework::RunnableConfig;
use serde::{Deserialize, Serialize};
use tokio::sync::OnceCell;

pub(crate) static GRPC_GATEWAY: OnceCell<GrpcGateway> = OnceCell::const_new();

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IndexerGrpcGatewayConfig {
    #[serde(default = "IndexerGrpcGatewayConfig::default_port")]
    pub(crate) port: u16,
    pub(crate) grpc_manager_address: String,
}

impl IndexerGrpcGatewayConfig {
    const fn default_port() -> u16 {
        8080
    }
}

#[async_trait::async_trait]
impl RunnableConfig for IndexerGrpcGatewayConfig {
    async fn run(&self) -> anyhow::Result<()> {
        GRPC_GATEWAY
            .get_or_init(|| async { GrpcGateway::new(self.clone()) })
            .await
            .start()
            .await
    }

    fn get_server_name(&self) -> String {
        "grpc_gateway".to_string()
    }
}

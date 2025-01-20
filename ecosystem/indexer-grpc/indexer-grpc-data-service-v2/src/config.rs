// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_indexer_grpc_server_framework::RunnableConfig;
use serde::{Deserialize, Serialize};

pub(crate) const MAX_MESSAGE_SIZE: usize = 256 * (1 << 20);

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IndexerGrpcDataServiceConfig {}

#[async_trait::async_trait]
impl RunnableConfig for IndexerGrpcDataServiceConfig {
    async fn run(&self) -> Result<()> {
        Ok(())
    }

    fn get_server_name(&self) -> String {
        "indexer_grpc_data_service_v2".to_string()
    }
}

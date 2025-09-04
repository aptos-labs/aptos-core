// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod processor;

use anyhow::Result;
use velor_indexer_grpc_server_framework::RunnableConfig;
use processor::Processor;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IndexerGrpcFileCheckerConfig {
    pub existing_bucket_name: String,
    pub new_bucket_name: String,
    pub starting_version: u64,
}

impl From<IndexerGrpcFileCheckerConfig> for Processor {
    fn from(val: IndexerGrpcFileCheckerConfig) -> Self {
        Processor {
            existing_bucket_name: val.existing_bucket_name,
            new_bucket_name: val.new_bucket_name,
            starting_version: val.starting_version,
        }
    }
}

#[async_trait::async_trait]
impl RunnableConfig for IndexerGrpcFileCheckerConfig {
    async fn run(&self) -> Result<()> {
        let processor: Processor = self.clone().into();

        processor
            .run()
            .await
            .expect("File checker exited unexpectedly");
        Ok(())
    }

    fn get_server_name(&self) -> String {
        "idxfilechk".to_string()
    }
}

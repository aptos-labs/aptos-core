// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

mod worker;

use crate::worker::Worker;
use anyhow::{Context, Result};
use aptos_indexer_grpc_server_framework::RunnableConfig;
use aptos_indexer_grpc_utils::config::IndexerGrpcFileStoreConfig;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IndexerGrpcFileStoreDataIntegrityCheckerConfig {
    pub file_store_config: IndexerGrpcFileStoreConfig,
    pub starting_version: Option<u64>,
}

impl IndexerGrpcFileStoreDataIntegrityCheckerConfig {
    pub fn new(
        file_store_config: IndexerGrpcFileStoreConfig,
        starting_version: Option<u64>,
    ) -> Self {
        Self {
            file_store_config,
            starting_version,
        }
    }
}

#[async_trait::async_trait]
impl RunnableConfig for IndexerGrpcFileStoreDataIntegrityCheckerConfig {
    async fn run(&self) -> Result<()> {
        let mut worker = Worker::new(self.file_store_config.clone(), self.starting_version)
            .await
            .context("Failed to create data integrity checker")?;
        worker
            .run()
            .await
            .context("Failed to run data integrity checker")
            .expect("Data integrity checker failed");
        Ok(())
    }

    fn get_server_name(&self) -> String {
        "idxdataitg".to_string()
    }
}

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod processor;

use anyhow::Result;
use aptos_indexer_grpc_server_framework::RunnableConfig;
use aptos_indexer_grpc_utils::config::IndexerGrpcFileStoreConfig;
use serde::{Deserialize, Serialize};

/// Configuration for the indexer gRPC GCS migration job.
/// This job migrates the files in the legacy GCS bucket to the new GCS bucket, e.g.,
/// from uncompressed to compressed files.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IndexerGrpcGcsMigrationConfig {
    /// The configuration for the legacy file store.
    /// Bucket read access and object read access required.
    pub legacy_file_store_config: IndexerGrpcFileStoreConfig,
    /// The configuration for the new file store. Write access required.
    /// Bucket write access and object write access required.
    pub new_file_store_config: IndexerGrpcFileStoreConfig,
    /// The chain ID of the network; verification purpose.
    pub chain_id: u64,
}

impl IndexerGrpcGcsMigrationConfig {
    pub fn new(
        legacy_file_store_config: IndexerGrpcFileStoreConfig,
        new_file_store_config: IndexerGrpcFileStoreConfig,
        chain_id: u64,
    ) -> Self {
        Self {
            legacy_file_store_config,
            new_file_store_config,
            chain_id,
        }
    }
}

#[async_trait::async_trait]
impl RunnableConfig for IndexerGrpcGcsMigrationConfig {
    async fn run(&self) -> Result<()> {
        let mut processor = processor::Processor::new(
            self.legacy_file_store_config.clone(),
            self.new_file_store_config.clone(),
            self.chain_id,
        );

        processor
            .run()
            .await
            .expect("File store processor exited unexpectedly");
        Ok(())
    }

    fn get_server_name(&self) -> String {
        "idxgcsmgrt".to_string()
    }
}

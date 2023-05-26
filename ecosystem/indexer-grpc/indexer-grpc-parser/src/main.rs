// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Ok, Result};
use aptos_indexer_grpc_parser::worker::Worker;
use aptos_indexer_grpc_server_framework::{RunnableConfig, ServerArgs};
use clap::Parser;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IndexerGrpcProcessorConfig {
    pub processor_name: String,
    pub postgres_connection_string: String,
    pub indexer_grpc_data_service_addresss: String,
    pub auth_token: String,
    pub starting_version: Option<u64>,
    pub number_concurrent_processing_tasks: Option<usize>,
    pub ans_address: Option<String>,
}

#[async_trait::async_trait]
impl RunnableConfig for IndexerGrpcProcessorConfig {
    async fn run(&self) -> Result<()> {
        let worker = Worker::new(
            self.processor_name.clone(),
            self.postgres_connection_string.clone(),
            self.indexer_grpc_data_service_addresss.clone(),
            self.auth_token.clone(),
            self.starting_version,
            self.number_concurrent_processing_tasks,
            self.ans_address.clone(),
        )
        .await;
        worker.run().await;
        Ok(())
    }

    fn get_server_name(&self) -> String {
        "idxproc".to_string()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = ServerArgs::parse();
    args.run::<IndexerGrpcProcessorConfig>().await
}

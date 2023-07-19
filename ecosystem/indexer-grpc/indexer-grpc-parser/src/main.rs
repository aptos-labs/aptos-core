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
    // TODO: add tls support.
    pub indexer_grpc_data_service_address: String,
    // Indexer GRPC http2 ping interval in seconds; default to 30.
    // tonic ref: https://docs.rs/tonic/latest/tonic/transport/channel/struct.Endpoint.html#method.http2_keep_alive_interval
    pub indexer_grpc_http2_ping_interval_in_secs: Option<u64>,
    // Indexer GRPC http2 ping timeout in seconds; default to 10.
    pub indexer_grpc_http2_ping_timeout_in_secs: Option<u64>,
    pub auth_token: String,
    pub starting_version: Option<u64>,
    pub ending_version: Option<u64>,
    pub number_concurrent_processing_tasks: Option<usize>,
    pub ans_address: Option<String>,
    pub nft_points_contract: Option<String>,
}

#[async_trait::async_trait]
impl RunnableConfig for IndexerGrpcProcessorConfig {
    async fn run(&self) -> Result<()> {
        let mut worker = Worker::new(
            self.processor_name.clone(),
            self.postgres_connection_string.clone(),
            self.indexer_grpc_data_service_address.clone(),
            std::time::Duration::from_secs(
                self.indexer_grpc_http2_ping_interval_in_secs.unwrap_or(30),
            ),
            std::time::Duration::from_secs(
                self.indexer_grpc_http2_ping_timeout_in_secs.unwrap_or(10),
            ),
            self.auth_token.clone(),
            self.starting_version,
            self.ending_version,
            self.number_concurrent_processing_tasks,
            self.ans_address.clone(),
            self.nft_points_contract.clone(),
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

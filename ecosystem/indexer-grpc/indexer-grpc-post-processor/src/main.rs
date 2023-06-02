// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_indexer_grpc_post_processor::pfn_ledger_checker::PfnLedgerChecker;
use aptos_indexer_grpc_server_framework::{RunnableConfig, ServerArgs};
use clap::Parser;
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IndexerGrpcPostProcessorConfig {
    pub file_store_bucket_name: String,
    pub verfied_backup_bucket_name: String,
    pub redis_main_instance_address: String,
    pub fullnode_grpc_address: String,
    pub public_fullnode_address: String,
}

#[async_trait::async_trait]
impl RunnableConfig for IndexerGrpcPostProcessorConfig {
    async fn run(&self) -> Result<()> {
        let checker = PfnLedgerChecker::new(self.public_fullnode_address.clone());
        info!("Starting PfnLedgerChecker");
        checker.run().await
    }

    fn get_server_name(&self) -> String {
        "idxbg".to_string()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = ServerArgs::parse();
    args.run::<IndexerGrpcPostProcessorConfig>().await
}

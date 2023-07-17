// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_indexer_grpc_post_processor::{
    file_storage_verifier::FileStorageVerifier, metrics::TASK_FAILURE_COUNT,
    pfn_ledger_checker::PfnLedgerChecker,
};
use aptos_indexer_grpc_server_framework::{RunnableConfig, ServerArgs};
use aptos_indexer_grpc_utils::config::IndexerGrpcFileStoreConfig;
use clap::Parser;
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IndexerGrpcPFNCheckerConfig {
    // List of public fullnode addresses.
    pub public_fullnode_addresses: Vec<String>,
    pub indexer_grpc_address: String,
    pub indexer_grpc_auth_token: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IndexerGrpcFileStorageVerifierConfig {
    pub file_store_config: IndexerGrpcFileStoreConfig,
    pub chain_id: u64,
}

// TODO: change this to match pattern.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IndexerGrpcPostProcessorConfig {
    pub pfn_checker_config: Option<IndexerGrpcPFNCheckerConfig>,
    pub file_storage_verifier: Option<IndexerGrpcFileStorageVerifierConfig>,
}

#[async_trait::async_trait]
impl RunnableConfig for IndexerGrpcPostProcessorConfig {
    async fn run(&self) -> Result<()> {
        let mut tasks = vec![];
        if let Some(config) = &self.pfn_checker_config {
            tasks.push(tokio::spawn({
                let config = config.clone();
                async move {
                    let public_fullnode_addresses = config.public_fullnode_addresses.clone();
                    loop {
                        if let Ok(checker) = PfnLedgerChecker::new(
                            public_fullnode_addresses.clone(),
                            config.indexer_grpc_address.clone(),
                            config.indexer_grpc_auth_token.clone(),
                        )
                        .await
                        {
                            info!("Starting PfnLedgerChecker");
                            if let Err(err) = checker.run().await {
                                tracing::error!("PfnLedgerChecker failed: {:?}", err);
                                TASK_FAILURE_COUNT
                                    .with_label_values(&["pfn_ledger_checker"])
                                    .inc();
                            }
                        } else {
                            tracing::error!("PfnLedgerChecker failed to initialize");
                            TASK_FAILURE_COUNT
                                .with_label_values(&["pfn_ledger_checker"])
                                .inc();
                        }
                    }
                }
            }));
        }

        if let Some(config) = &self.file_storage_verifier {
            tasks.push(tokio::spawn({
                let config = config.clone();
                async move {
                    let checker =
                        FileStorageVerifier::new(config.file_store_config.clone(), config.chain_id);
                    info!("Starting FileStorageVerifier");
                    if let Err(err) = checker.run().await {
                        tracing::error!("FileStorageVerifier failed: {:?}", err);
                        TASK_FAILURE_COUNT
                            .with_label_values(&["file_storage_verifier"])
                            .inc();
                    }
                }
            }));
        }

        let _ = futures::future::join_all(tasks).await;
        unreachable!("All tasks should run forever");
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

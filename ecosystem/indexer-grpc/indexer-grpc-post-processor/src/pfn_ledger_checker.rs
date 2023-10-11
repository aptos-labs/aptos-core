// Copyright © Aptos Foundation

use crate::metrics::INDEXER_GRPC_LATENCY_AGAINST_PFN_LATENCY_IN_SECS;
use anyhow::{ensure, Result};
use aptos_indexer_grpc_utils::constants::{
    GRPC_API_GATEWAY_API_KEY_HEADER, GRPC_REQUEST_NAME_HEADER,
};
use aptos_protos::indexer::v1::{raw_data_client::RawDataClient, GetTransactionsRequest};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, SystemTime},
};
use tracing::info;

const PFN_CHECKER_WAIT_TIME_IN_SECS: u64 = 20;
// The frequency of reconnecting to indexer grpc.
const GRPC_RECONNECT_FREQUENCY_IN_SECS: u64 = 4 * 60;

// Thread-safe latest indexer grpc response bblock time.
type IndexerGrpcResponseBlockTimeInMs = Arc<Mutex<u64>>;

pub struct PfnLedgerChecker {
    pub public_fullnode_addresses: Vec<String>,
    pub indexer_grpc_address: String,
    pub indexer_grpc_auth_token: String,
    pub indexer_grpc_response_block_time_in_ms: IndexerGrpcResponseBlockTimeInMs,
    pub ledger_version: u64,
    pub chain_id: u64,
}

#[derive(Debug, Deserialize, Serialize)]
struct IndexResponse {
    chain_id: u8,
    epoch: String,
    ledger_version: String,
    oldest_ledger_version: String,
    ledger_timestamp: String,
    node_role: String,
    oldest_block_height: String,
    block_height: String,
    git_hash: String,
}

impl PfnLedgerChecker {
    pub async fn new(
        public_fullnode_addresses: Vec<String>,
        indexer_grpc_address: String,
        indexer_grpc_auth_token: String,
    ) -> Result<Self> {
        ensure!(
            !public_fullnode_addresses.is_empty(),
            "public_fullnode_addresses cannot be empty"
        );
        ensure!(
            !indexer_grpc_address.is_empty(),
            "indexer_grpc_address cannot be empty"
        );

        let client = reqwest::Client::new();
        let pfn_address = public_fullnode_addresses[0].clone();
        let response = client
            .get(pfn_address.as_str())
            .send()
            .await?
            .json::<IndexResponse>()
            .await?;
        let ledger_version = response.ledger_version.parse::<u64>()?;
        let chain_id = response.chain_id as u64;

        Ok(Self {
            public_fullnode_addresses,
            indexer_grpc_address,
            indexer_grpc_auth_token,
            indexer_grpc_response_block_time_in_ms: Arc::new(Mutex::new(0)),
            ledger_version,
            chain_id,
        })
    }

    pub async fn run(&self) -> Result<()> {
        let mut handles = Vec::new();
        let indexer_grpc_address = self.indexer_grpc_address.clone();
        let indexer_grpc_auth_token = self.indexer_grpc_auth_token.clone();
        let starting_version = self.ledger_version;
        let indexer_grpc_response_block_time_in_ms =
            self.indexer_grpc_response_block_time_in_ms.clone();
        let handle = tokio::spawn(async move {
            let res = handle_indexer_grpc(
                indexer_grpc_address.clone(),
                indexer_grpc_auth_token.clone(),
                starting_version,
                indexer_grpc_response_block_time_in_ms,
            )
            .await;
            if let Err(e) = res {
                anyhow::bail!("Failed to handle indexer grpc: {:?}", e);
            }
            Ok(())
        });
        handles.push(handle);
        for pfn_address in self.public_fullnode_addresses.as_slice() {
            let chain_id = self.chain_id;
            let public_fullnode_addresses = pfn_address.clone();
            let indexer_grpc_response_block_time_in_ms =
                self.indexer_grpc_response_block_time_in_ms.clone();
            let handle = tokio::spawn(async move {
                handle_pfn_response(
                    public_fullnode_addresses,
                    chain_id,
                    indexer_grpc_response_block_time_in_ms,
                )
                .await
            });
            handles.push(handle);
        }
        let (task_res, _, _) = futures::future::select_all(handles.into_iter()).await;
        match task_res {
            Ok(Ok(_)) => {
                unreachable!("The task should never finish successfully");
            },
            _ => {
                anyhow::bail!("Failed to run pfn ledger checker: {:?}", task_res);
            },
        }
    }
}

async fn handle_pfn_response(
    pfn_address: String,
    chain_id: u64,
    indexer_grpc_response_block_time_in_ms: IndexerGrpcResponseBlockTimeInMs,
) -> Result<()> {
    info!("Start processing pfn_address: {}", pfn_address);
    // Let the map be filled with some data first.
    tokio::time::sleep(Duration::from_secs(PFN_CHECKER_WAIT_TIME_IN_SECS)).await;
    let client = reqwest::Client::new();
    loop {
        let response = client
            .get(pfn_address.as_str())
            .send()
            .await?
            .json::<IndexResponse>()
            .await?;
        let pfn_chain_id = response.chain_id as u64;
        ensure!(chain_id == pfn_chain_id, "chain_id mismatch");
        let latency_in_sec = {
            let indexer_grpc_response_block_time_in_ms =
                *indexer_grpc_response_block_time_in_ms.lock().unwrap();
            let pfn_block_time_in_ms = response.ledger_timestamp.parse::<u64>()? / 1000;
            let latency_in_ms =
                pfn_block_time_in_ms as i64 - indexer_grpc_response_block_time_in_ms as i64;
            latency_in_ms as f64 / 1000.0
        };
        INDEXER_GRPC_LATENCY_AGAINST_PFN_LATENCY_IN_SECS
            .with_label_values(&[&pfn_address.to_string()])
            .set(latency_in_sec);
    }
}

async fn handle_indexer_grpc(
    indexer_grpc_address: String,
    indexer_grpc_auth_token: String,
    starting_version: u64,
    indexer_grpc_response_block_time_in_ms: IndexerGrpcResponseBlockTimeInMs,
) -> Result<()> {
    let mut current_version = starting_version;
    loop {
        let time_now = SystemTime::now();
        let channel =
            tonic::transport::Channel::from_shared(format!("https://{}", indexer_grpc_address))?
                .http2_keep_alive_interval(Duration::from_secs(60))
                .keep_alive_timeout(Duration::from_secs(10));
        let mut indexer_grpc_client = RawDataClient::connect(channel).await?;
        let mut request = tonic::Request::new(GetTransactionsRequest {
            starting_version: Some(current_version),
            transactions_count: None,
            batch_size: None,
        });

        request.metadata_mut().insert(
            GRPC_API_GATEWAY_API_KEY_HEADER,
            format!("Bearer {}", indexer_grpc_auth_token).parse()?,
        );
        request
            .metadata_mut()
            .insert(GRPC_REQUEST_NAME_HEADER, "indexer-grpc-monitor".parse()?);
        let mut response = indexer_grpc_client
            .get_transactions(request)
            .await?
            .into_inner();
        info!("Start processing indexer grpc response items.");
        while let Some(resp) = response.next().await {
            if time_now.elapsed().unwrap() > Duration::from_secs(GRPC_RECONNECT_FREQUENCY_IN_SECS) {
                break;
            }
            match resp {
                Ok(item) => {
                    ensure!(
                        !item.transactions.is_empty(),
                        "item.transactions.len() must be 1"
                    );
                    // Required.
                    current_version = item.transactions.last().unwrap().version;

                    let block_time_opt = item.transactions.last().unwrap().timestamp.clone();

                    if let Some(block_time) = block_time_opt {
                        let block_time_in_ms =
                            block_time.seconds as u64 * 1000 + block_time.nanos as u64 / 1_000_000;
                        // Update the latest indexer grpc response block time.
                        let mut indexer_grpc_response_block_time_in_ms =
                            indexer_grpc_response_block_time_in_ms.lock().unwrap();
                        *indexer_grpc_response_block_time_in_ms = block_time_in_ms;
                    }
                },
                Err(e) => {
                    anyhow::bail!("Error while receiving item: {:?}", e);
                },
            }
        }
    }
}

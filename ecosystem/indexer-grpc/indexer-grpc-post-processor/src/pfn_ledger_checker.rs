// Copyright © Aptos Foundation

use crate::metrics::INDEXER_GRPC_LATENCY_AGAINST_PFN_LATENCY_IN_SECS;
use anyhow::{ensure, Result};
use aptos_indexer_grpc_utils::constants::GRPC_AUTH_TOKEN_HEADER;
use aptos_protos::indexer::v1::{raw_data_client::RawDataClient, GetTransactionsRequest};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::RwLock;
use tracing::info;

const LOOKUP_TABLE_SIZE: usize = 10000;
const PFN_CHECKER_WAIT_TIME_IN_SECS: u64 = 10;

// This is to create a look up table for recent indexer grpc response time across multiple threads.
type IndexerGrpcResponseTimeMap = Arc<RwLock<BTreeMap<u64, SystemTime>>>;

pub struct PfnLedgerChecker {
    pub public_fullnode_addresses: Vec<String>,
    pub indexer_grpc_address: String,
    pub indexer_grpc_auth_token: String,
    // Records of the recent indexer grpc response items for their response time.
    pub indexer_grpc_response_time_map: IndexerGrpcResponseTimeMap,
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
            indexer_grpc_response_time_map: Arc::new(RwLock::new(BTreeMap::new())),
            ledger_version,
            chain_id,
        })
    }

    pub async fn run(&self) -> Result<()> {
        let mut handles = Vec::new();
        let indexer_grpc_address = self.indexer_grpc_address.clone();
        let indexer_grpc_auth_token = self.indexer_grpc_auth_token.clone();
        let starting_version = self.ledger_version;
        let indexer_grpc_response_time_map = self.indexer_grpc_response_time_map.clone();
        let handle = tokio::spawn(async move {
            let res = handle_indexer_grpc(
                indexer_grpc_address.clone(),
                indexer_grpc_auth_token.clone(),
                starting_version,
                indexer_grpc_response_time_map,
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
            let indexer_grpc_response_time_map = self.indexer_grpc_response_time_map.clone();
            let handle = tokio::spawn(async move {
                handle_pfn_response(
                    public_fullnode_addresses,
                    chain_id,
                    indexer_grpc_response_time_map,
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
    indexer_grpc_response_time_map: IndexerGrpcResponseTimeMap,
) -> Result<()> {
    info!("Start processing pfn_address: {}", pfn_address);
    let client = reqwest::Client::new();
    loop {
        let pfn_ledger_time = SystemTime::now();
        let response = client
            .get(pfn_address.as_str())
            .send()
            .await?
            .json::<IndexResponse>()
            .await?;
        let ledger_version = response.ledger_version.parse::<u64>()?;
        let pfn_chain_id = response.chain_id as u64;
        ensure!(chain_id == pfn_chain_id, "chain_id mismatch");
        // Wait a few seconds to make sure the indexer grpc response is ready.
        tokio::time::sleep(Duration::from_secs(PFN_CHECKER_WAIT_TIME_IN_SECS)).await;
        let map_read_lock = indexer_grpc_response_time_map.read().await;
        if let Some(indexer_node_ledger_time) = map_read_lock.get(&ledger_version) {
            let latency = indexer_node_ledger_time
                .duration_since(UNIX_EPOCH)?
                .as_secs_f64()
                - pfn_ledger_time.duration_since(UNIX_EPOCH)?.as_secs_f64();
            INDEXER_GRPC_LATENCY_AGAINST_PFN_LATENCY_IN_SECS
                .with_label_values(&[&pfn_address.to_string()])
                .set(latency);
        }
    }
}

async fn handle_indexer_grpc(
    indexer_grpc_address: String,
    indexer_grpc_auth_token: String,
    starting_version: u64,
    indexer_grpc_response_time_map: IndexerGrpcResponseTimeMap,
) -> Result<()> {
    let channel =
        tonic::transport::Channel::from_shared(format!("http://{}", indexer_grpc_address))?
            .http2_keep_alive_interval(Duration::from_secs(60))
            .keep_alive_timeout(Duration::from_secs(10));
    let mut indexer_grpc_client = RawDataClient::connect(channel).await?;
    let mut request = tonic::Request::new(GetTransactionsRequest {
        starting_version: Some(starting_version),
        transactions_count: None,
        batch_size: None,
    });

    request
        .metadata_mut()
        .insert(GRPC_AUTH_TOKEN_HEADER, indexer_grpc_auth_token.parse()?);
    let mut response = indexer_grpc_client
        .get_transactions(request)
        .await?
        .into_inner();
    info!("Start processing indexer grpc response items.");
    loop {
        match response.next().await {
            Some(Ok(item)) => {
                let time_now = SystemTime::now();
                let mut map_update_lock = indexer_grpc_response_time_map.write().await;
                for transaction in item.transactions {
                    let version = transaction.version;
                    map_update_lock.insert(version, time_now);
                    if map_update_lock.len() > LOOKUP_TABLE_SIZE {
                        map_update_lock.pop_first();
                    }
                }
            },
            Some(Err(e)) => {
                anyhow::bail!("Error while receiving item: {:?}", e);
            },
            None => {
                unreachable!("Indexer grpc response stream should not be closed.");
            },
        }
    }
}

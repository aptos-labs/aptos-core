// Copyright © Aptos Foundation

use crate::metrics::OBSERVED_LATEST_TRANSACTION_LATENCY;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{error, info};

pub struct PfnLedgerChecker {
    pub public_fullnode_address: String,
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
    pub fn new(public_fullnode_address: String) -> Self {
        Self {
            public_fullnode_address,
        }
    }

    pub async fn run(&self) -> Result<()> {
        // Create a http client.
        let client = reqwest::Client::new();
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
            let response = client
                .get(self.public_fullnode_address.as_str())
                .send()
                .await;

            if response.is_err() {
                error!("failure to get response: {:?}", response);
                continue;
            }
            let response = response.unwrap();
            let json_response = response.json::<IndexResponse>().await;
            if json_response.is_err() {
                error!("failure to parse response {:?}", json_response);
            }
            let json_response = json_response.unwrap();
            let transaction_timestamp_in_secs =
                json_response.ledger_timestamp.parse::<u128>().unwrap() as f64 / 1_000_000.0;
            let current_timestamp_in_secs = chrono::Utc::now().timestamp() as f64;
            let latency_in_secs = current_timestamp_in_secs - transaction_timestamp_in_secs;
            info!(
                latency_in_secs = latency_in_secs,
                "observed latest transaction latency"
            );
            OBSERVED_LATEST_TRANSACTION_LATENCY
                .with_label_values(&["pfn"])
                .set(latency_in_secs);
        }
    }
}

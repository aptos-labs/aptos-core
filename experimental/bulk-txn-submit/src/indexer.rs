// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_logger::warn;
use aptos_sdk::types::chain_id::ChainId;
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use std::sync::{atomic::AtomicI64, Arc};

#[derive(Serialize, Deserialize, Debug)]
struct IndexerStatusResponse {
    processor_status: Vec<ProcessorStatus>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ProcessorStatus {
    processor: String,
    last_updated: String,
    last_success_version: u64,
    last_transaction_timestamp: String,
}

fn indexer_status_url(chain_id: ChainId) -> &'static str {
    if chain_id.is_mainnet() {
        "https://indexer.mainnet.aptoslabs.com/api/rest/get_latest_processor_status"
    } else if chain_id.is_testnet() {
        "https://indexer-testnet.staging.gcp.aptosdev.com/api/rest/get_latest_processor_status"
    } else {
        "https://indexer-devnet.staging.gcp.aptosdev.com/api/rest/get_latest_processor_status"
    }
}

async fn fetch_indexer_delay(chain_id: ChainId) -> Result<i64> {
    let resp = reqwest::get(indexer_status_url(chain_id))
        .await?
        .text()
        .await?;

    let parsed: IndexerStatusResponse = serde_json::from_str(&resp)?;

    let timestamps = parsed
        .processor_status
        .iter()
        .map(|status| {
            let now_parsed: DateTime<Utc> = Utc.from_utc_datetime(&NaiveDateTime::parse_from_str(
                &status.last_transaction_timestamp,
                "%Y-%m-%dT%H:%M:%S%.f",
            )?);
            Ok(Utc::now().signed_duration_since(now_parsed).num_seconds())
            // }).collect::<Vec<Result<_, chrono::ParseError>>>();
        })
        .collect::<Result<Vec<_>, chrono::ParseError>>()?;
    // info!("Indexer delay by processor: {:?}", timestamps);
    Ok(timestamps.into_iter().max().unwrap())
}

pub(crate) async fn continuously_update_indexer_delay(
    chain_id: ChainId,
    delay_state: Arc<AtomicI64>,
) {
    loop {
        match fetch_indexer_delay(chain_id).await {
            Ok(delay) => {
                delay_state.store(delay, std::sync::atomic::Ordering::Relaxed);
            },
            Err(e) => {
                //warn!("fetch_indexer_delay error: {:?}", e);
            },
        }

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}

#[ignore]
#[tokio::test]
async fn test_fetch_indexer_delay() {
    for _ in 0..1000 {
        println!("{}", fetch_indexer_delay(ChainId::testnet()).await.unwrap());
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}

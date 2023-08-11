// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::PROCESSOR_LAST_UPDATED_TIME_LATENCY_IN_SECS;
use anyhow::Result;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::info;

const PROCESSOR_STATUS_CHECKER_WAIT_TIME_IN_SECS: u64 = 10;

pub struct ProcessorStatusChecker {
    pub hasura_rest_api_endpoint: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct ProcessorStatusResponse {
    processor_status: Vec<ProcessorStatus>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ProcessorStatus {
    pub processor: String,
    pub last_updated: String,
    pub last_success_version: i64,
}

impl ProcessorStatusChecker {
    pub fn new(hasura_rest_api_endpoint: String) -> Self {
        Self {
            hasura_rest_api_endpoint,
        }
    }

    pub async fn run(&self) -> Result<()> {
        loop {
            let endpoint = self.hasura_rest_api_endpoint.clone();
            match handle_hasura(endpoint).await {
                Ok(_) => {},
                Err(e) => {
                    tracing::error!(e = ?e, "Failed to get processor status response from hasura");
                    panic!();
                },
            }
            tokio::time::sleep(Duration::from_secs(
                PROCESSOR_STATUS_CHECKER_WAIT_TIME_IN_SECS,
            ))
            .await;
        }
    }
}

async fn handle_hasura(hasura_endpoint: String) -> Result<()> {
    let endpoint = hasura_endpoint.clone();
    info!("Connecting to hasura endpoint: {}", endpoint);
    let client = reqwest::Client::new();
    let result = client.get(endpoint).send().await?;
    let processor_status_response_result = result.json::<ProcessorStatusResponse>().await;
    let processor_status_response = match processor_status_response_result {
        Ok(processor_status_response) => processor_status_response,
        Err(e) => {
            tracing::error!(e = ?e, "Failed to get processor status response");
            panic!();
        },
    };

    for processor_status in processor_status_response.processor_status {
        let last_updated_time = NaiveDateTime::parse_from_str(
            processor_status.last_updated.as_str(),
            "%Y-%m-%dT%H:%M:%S%.f",
        )
        .unwrap();
        let current_time = SystemTime::now();
        let latency = current_time.duration_since(UNIX_EPOCH)?.as_secs_f64()
            - last_updated_time
                .signed_duration_since(NaiveDateTime::from_timestamp(0, 0))
                .to_std()?
                .as_secs_f64();
        PROCESSOR_LAST_UPDATED_TIME_LATENCY_IN_SECS
            .with_label_values(&[processor_status.processor.as_str()])
            .set(latency);
    }
    Ok(())
}

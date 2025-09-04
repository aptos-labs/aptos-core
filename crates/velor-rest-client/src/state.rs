// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_api_types::{
    X_VELOR_BLOCK_HEIGHT, X_VELOR_CHAIN_ID, X_VELOR_CURSOR, X_VELOR_EPOCH,
    X_VELOR_LEDGER_OLDEST_VERSION, X_VELOR_LEDGER_TIMESTAMP, X_VELOR_LEDGER_VERSION,
    X_VELOR_OLDEST_BLOCK_HEIGHT,
};

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct State {
    pub chain_id: u8,
    pub epoch: u64,
    pub version: u64,
    pub timestamp_usecs: u64,
    pub oldest_ledger_version: u64,
    pub oldest_block_height: u64,
    pub block_height: u64,
    pub cursor: Option<String>,
}

impl State {
    pub fn from_headers(headers: &reqwest::header::HeaderMap) -> anyhow::Result<Self> {
        let maybe_chain_id = headers
            .get(X_VELOR_CHAIN_ID)
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.parse().ok());
        let maybe_version = headers
            .get(X_VELOR_LEDGER_VERSION)
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.parse().ok());
        let maybe_timestamp = headers
            .get(X_VELOR_LEDGER_TIMESTAMP)
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.parse().ok());
        let maybe_epoch = headers
            .get(X_VELOR_EPOCH)
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.parse().ok());
        let maybe_oldest_ledger_version = headers
            .get(X_VELOR_LEDGER_OLDEST_VERSION)
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.parse().ok());
        let maybe_block_height = headers
            .get(X_VELOR_BLOCK_HEIGHT)
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.parse().ok());
        let maybe_oldest_block_height = headers
            .get(X_VELOR_OLDEST_BLOCK_HEIGHT)
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.parse().ok());
        let cursor = headers
            .get(X_VELOR_CURSOR)
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        let state = if let (
            Some(chain_id),
            Some(version),
            Some(timestamp_usecs),
            Some(epoch),
            Some(oldest_ledger_version),
            Some(block_height),
            Some(oldest_block_height),
            cursor,
        ) = (
            maybe_chain_id,
            maybe_version,
            maybe_timestamp,
            maybe_epoch,
            maybe_oldest_ledger_version,
            maybe_block_height,
            maybe_oldest_block_height,
            cursor,
        ) {
            Self {
                chain_id,
                epoch,
                version,
                timestamp_usecs,
                oldest_ledger_version,
                block_height,
                oldest_block_height,
                cursor,
            }
        } else {
            anyhow::bail!(
                "Failed to build State from headers due to missing values in response. \
                Chain ID: {:?}, Version: {:?}, Timestamp: {:?}, Epoch: {:?}, \
                Oldest Ledger Version: {:?}, Block Height: {:?} Oldest Block Height: {:?}",
                maybe_chain_id,
                maybe_version,
                maybe_timestamp,
                maybe_epoch,
                maybe_oldest_ledger_version,
                maybe_block_height,
                maybe_oldest_block_height,
            )
        };

        Ok(state)
    }
}

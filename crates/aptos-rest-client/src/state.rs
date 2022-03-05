// Copyright (c) The Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_api_types::{X_APTOS_CHAIN_ID, X_APTOS_LEDGER_TIMESTAMP, X_APTOS_LEDGER_VERSION};

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct State {
    pub chain_id: u8,
    pub version: u64,
    pub timestamp_usecs: u64,
}

impl State {
    pub fn from_headers(headers: &reqwest::header::HeaderMap) -> anyhow::Result<Self> {
        let maybe_chain_id = headers
            .get(X_APTOS_CHAIN_ID)
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.parse().ok());
        let maybe_version = headers
            .get(X_APTOS_LEDGER_VERSION)
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.parse().ok());
        let maybe_timestamp = headers
            .get(X_APTOS_LEDGER_TIMESTAMP)
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.parse().ok());

        let state = if let (Some(chain_id), Some(version), Some(timestamp_usecs)) =
            (maybe_chain_id, maybe_version, maybe_timestamp)
        {
            Self {
                chain_id,
                version,
                timestamp_usecs,
            }
        } else {
            todo!()
        };

        Ok(state)
    }
}

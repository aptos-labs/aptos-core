// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::RestError;
use anyhow::anyhow;
use aptos_api_types::{
    mime_types::{BCS, JSON},
    LedgerInfo, X_APTOS_CHAIN_ID, X_APTOS_EPOCH, X_APTOS_LEDGER_OLDEST_VERSION,
    X_APTOS_LEDGER_TIMESTAMP, X_APTOS_LEDGER_VERSION,
};
use reqwest::{header::CONTENT_TYPE, Response};
use serde::de::DeserializeOwned;

/// Ledger response, containing the ledger state and the inner type
#[derive(Clone, Debug)]
pub struct LedgerResponse<T> {
    pub(crate) inner: T,
    pub(crate) ledger_info: LedgerInfo,
}

impl<T: DeserializeOwned> LedgerResponse<T> {
    pub async fn from_response(response: Response) -> anyhow::Result<LedgerResponse<T>> {
        if !response.status().is_success() {
            let error_response = response.json::<RestError>().await?;
            return Err(anyhow::anyhow!("Request failed: {:?}", error_response));
        }
        let ledger_info = ledger_info_from_headers(response.headers())?;

        let encoding = response
            .headers()
            .get(CONTENT_TYPE)
            .map(|inner| inner.to_str());

        let inner: T = match encoding {
            Some(Ok(BCS)) => bcs::from_bytes(&response.bytes().await?)?,
            Some(Ok(JSON)) => serde_json::from_str(&response.text().await?)?,
            _ => return Err(anyhow!("Invalid encoding type {:?}", encoding)),
        };

        Ok(LedgerResponse { inner, ledger_info })
    }

    pub fn ledger_info(&self) -> &LedgerInfo {
        &self.ledger_info
    }

    pub fn inner(&self) -> &T {
        &self.inner
    }

    pub fn into_inner(self) -> T {
        self.inner
    }
}

fn ledger_info_from_headers(headers: &reqwest::header::HeaderMap) -> anyhow::Result<LedgerInfo> {
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
    let maybe_epoch = headers
        .get(X_APTOS_EPOCH)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.parse().ok());
    let maybe_oldest_ledger_version = headers
        .get(X_APTOS_LEDGER_OLDEST_VERSION)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.parse().ok());

    if let (
        Some(chain_id),
        Some(ledger_version),
        Some(ledger_timestamp),
        Some(epoch),
        Some(oldest_ledger_version),
    ) = (
        maybe_chain_id,
        maybe_version,
        maybe_timestamp,
        maybe_epoch,
        maybe_oldest_ledger_version,
    ) {
        Ok(LedgerInfo {
            chain_id,
            epoch,
            ledger_version,
            ledger_timestamp,
            oldest_ledger_version,
        })
    } else {
        Err(anyhow!("Failed to parse LedgerInfo from headers"))
    }
}

// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! v2-specific request and response types.
//!
//! All successful v2 responses use the `V2Response<T>` envelope, which
//! embeds ledger metadata in the JSON body (no custom response headers).

use aptos_api_types::LedgerInfo;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Standard envelope for all successful v2 API responses.
///
/// Ledger metadata is included in the body -- v2 does NOT set `X-Aptos-*` headers.
/// For paginated endpoints, `cursor` contains an opaque token for the next page.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct V2Response<T: Serialize> {
    /// The actual response data.
    pub data: T,
    /// Ledger metadata at the time of the request.
    pub ledger: LedgerMetadata,
    /// Opaque pagination cursor (present when more pages exist).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

/// Ledger metadata included in every successful v2 response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LedgerMetadata {
    pub chain_id: u8,
    pub ledger_version: u64,
    pub oldest_ledger_version: u64,
    pub ledger_timestamp_usec: u64,
    pub epoch: u64,
    pub block_height: u64,
    pub oldest_block_height: u64,
}

impl From<&LedgerInfo> for LedgerMetadata {
    fn from(info: &LedgerInfo) -> Self {
        LedgerMetadata {
            chain_id: info.chain_id,
            ledger_version: info.ledger_version.into(),
            oldest_ledger_version: info.oldest_ledger_version.into(),
            ledger_timestamp_usec: info.ledger_timestamp.into(),
            epoch: info.epoch.into(),
            block_height: info.block_height.into(),
            oldest_block_height: info.oldest_block_height.into(),
        }
    }
}

impl<T: Serialize> V2Response<T> {
    pub fn new(data: T, ledger_info: &LedgerInfo) -> Self {
        Self {
            data,
            ledger: LedgerMetadata::from(ledger_info),
            cursor: None,
        }
    }

    pub fn with_cursor(mut self, cursor: Option<String>) -> Self {
        self.cursor = cursor;
        self
    }
}

/// Query parameters for paginated endpoints with optional ledger version.
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct PaginatedLedgerParams {
    /// Optional ledger version to query at.
    pub ledger_version: Option<u64>,
    /// Opaque cursor from a previous page response.
    pub cursor: Option<String>,
}

/// Query parameters for paginated endpoints that only need a cursor.
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct CursorOnlyParams {
    /// Opaque cursor from a previous page response.
    pub cursor: Option<String>,
}

/// Query parameters for single-item endpoints with optional ledger version.
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct LedgerVersionParam {
    /// Optional ledger version to query at.
    pub ledger_version: Option<u64>,
}

/// Query parameters for block endpoints.
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct BlockParams {
    /// Whether to include transactions in the block response.
    pub with_transactions: Option<bool>,
}

/// Health check response (not wrapped in V2Response since it's a special endpoint).
#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub ledger: LedgerMetadata,
}

/// Node info returned by /v2/info.
#[derive(Debug, Serialize, ToSchema)]
pub struct NodeInfo {
    pub chain_id: u8,
    pub role: String,
    pub api_version: String,
}

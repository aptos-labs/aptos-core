// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::server::utils::CONTENT_TYPE_JSON;
use aptos_transaction_tracing::store::TransactionTraceStore;
use hyper::{Body, StatusCode};

/// Handles GET /transaction_tracing — returns the current tracing filter config.
pub fn handle_get_request() -> (StatusCode, Body, String) {
    let filter = TransactionTraceStore::global().get_filter();
    match serde_json::to_string(filter.as_ref()) {
        Ok(json) => (StatusCode::OK, Body::from(json), CONTENT_TYPE_JSON.into()),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Body::from(format!("Failed to serialize filter: {}", e)),
            CONTENT_TYPE_JSON.into(),
        ),
    }
}

// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::server::utils::CONTENT_TYPE_JSON;
use aptos_transaction_tracing::{filter::TransactionFilter, store::TransactionTraceStore};
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

/// Handles POST /transaction_tracing — updates the tracing filter.
/// Expects JSON body: { "enabled": bool, "sender_allowlist": ["0x..."] }
pub async fn handle_post_request(body: Body) -> (StatusCode, Body, String) {
    let body_bytes = match hyper::body::to_bytes(body).await {
        Ok(bytes) => bytes,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Body::from(format!("Failed to read request body: {}", e)),
                CONTENT_TYPE_JSON.into(),
            );
        },
    };

    let filter: TransactionFilter = match serde_json::from_slice(&body_bytes) {
        Ok(f) => f,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Body::from(format!("Invalid JSON: {}", e)),
                CONTENT_TYPE_JSON.into(),
            );
        },
    };

    TransactionTraceStore::global().update_filter(filter);

    let updated = TransactionTraceStore::global().get_filter();
    match serde_json::to_string(updated.as_ref()) {
        Ok(json) => (StatusCode::OK, Body::from(json), CONTENT_TYPE_JSON.into()),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Body::from(format!("Failed to serialize filter: {}", e)),
            CONTENT_TYPE_JSON.into(),
        ),
    }
}

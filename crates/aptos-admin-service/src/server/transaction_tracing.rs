// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_system_utils::utils::reply_with_status;
use aptos_transaction_tracing::{filter::TransactionFilter, store::TransactionTraceStore};
use hyper::{Body, Request, Response, StatusCode};

/// Handles POST /transaction_tracing — updates the tracing filter.
/// Expects JSON body: { "enabled": bool, "sender_allowlist": ["0x..."] }
pub async fn handle_post_request(req: Request<Body>) -> hyper::Result<Response<Body>> {
    let (_, body) = req.into_parts();
    let body_bytes = match hyper::body::to_bytes(body).await {
        Ok(bytes) => bytes,
        Err(e) => {
            return Ok(reply_with_status(
                StatusCode::BAD_REQUEST,
                format!("Failed to read request body: {}", e),
            ));
        },
    };

    let filter: TransactionFilter = match serde_json::from_slice(&body_bytes) {
        Ok(f) => f,
        Err(e) => {
            return Ok(reply_with_status(
                StatusCode::BAD_REQUEST,
                format!("Invalid JSON: {}", e),
            ));
        },
    };

    TransactionTraceStore::global().update_filter(filter);

    let updated = TransactionTraceStore::global().get_filter();
    match serde_json::to_string(updated.as_ref()) {
        Ok(json) => Ok(Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .body(Body::from(json))
            .unwrap()),
        Err(e) => Ok(reply_with_status(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to serialize filter: {}", e),
        )),
    }
}

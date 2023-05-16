// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::server::{
    json_encoder::JsonEncoder,
    utils,
    utils::{CONTENT_TYPE_JSON, CONTENT_TYPE_TEXT},
};
use hyper::{Body, StatusCode};
use prometheus::TextEncoder;

/// Handles a new forge metrics request
pub fn handle_forge_metrics() -> (StatusCode, Body, String) {
    // Get and encode the metrics
    let metrics = utils::get_all_metrics();
    let encoded_metrics = match serde_json::to_string(&metrics) {
        Ok(encoded_metrics) => encoded_metrics,
        Err(error) => format!("Failed to get forge metrics! Error: {}", error),
    };

    (
        StatusCode::OK,
        Body::from(encoded_metrics),
        CONTENT_TYPE_JSON.into(),
    )
}

/// Handles a new metrics request (with JSON encoding)
pub fn handle_json_metrics_request() -> (StatusCode, Body, String) {
    let buffer = utils::get_encoded_metrics(JsonEncoder);
    (StatusCode::OK, Body::from(buffer), CONTENT_TYPE_JSON.into())
}

/// Handles a new metrics request (with text encoding)
pub fn handle_metrics_request() -> (StatusCode, Body, String) {
    let buffer = utils::get_encoded_metrics(TextEncoder::new());
    (StatusCode::OK, Body::from(buffer), CONTENT_TYPE_TEXT.into())
}

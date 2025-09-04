// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::server::{
    json_encoder::JsonEncoder,
    utils,
    utils::{CONTENT_TYPE_JSON, CONTENT_TYPE_TEXT},
};
use velor_config::config::NodeConfig;
use hyper::{Body, StatusCode};
use prometheus::TextEncoder;

// The metric key for the consensus execution gauge
const CONSENSUS_EXECUTION_GAUGE: &str = "velor_state_sync_consensus_executing_gauge{}";

/// Handles a consensus health check request. This method returns
/// 200 if the node is currently participating in consensus.
///
/// Note: we assume that this endpoint will only be used every few seconds.
pub async fn handle_consensus_health_check(node_config: &NodeConfig) -> (StatusCode, Body, String) {
    // Verify the node is a validator. If not, return an error.
    if !node_config.base.role.is_validator() {
        return (
            StatusCode::BAD_REQUEST,
            Body::from("This node is not a validator!"),
            CONTENT_TYPE_TEXT.into(),
        );
    }

    // Check the value of the consensus execution gauge
    let metrics = utils::get_all_metrics();
    if let Some(gauge_value) = metrics.get(CONSENSUS_EXECUTION_GAUGE) {
        if gauge_value == "1" {
            return (
                StatusCode::OK,
                Body::from("Consensus health check passed!"),
                CONTENT_TYPE_TEXT.into(),
            );
        }
    }

    // Otherwise, consensus is not executing
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Body::from("Consensus health check failed! Consensus is not executing!"),
        CONTENT_TYPE_TEXT.into(),
    )
}

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

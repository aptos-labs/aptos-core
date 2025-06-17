use crate::server::utils::{get_all_metrics, CONTENT_TYPE_TEXT};
use hyper::{Body, StatusCode};
use prometheus::TextEncoder;

/// Handles a request for consensus metrics
pub fn handle_consensus_metrics_request() -> (StatusCode, Body, String) {
    let all_metrics = get_all_metrics();
    
    // Filter for consensus metrics
    let consensus_metrics: Vec<String> = all_metrics
        .iter()
        .filter_map(|(key, value)| {
            if key.starts_with("aptos_consensus") {
                Some(format!("{} {}", key, value))
            } else {
                None
            }
        })
        .collect();

    (
        StatusCode::OK,
        Body::from(consensus_metrics.join("\n")),
        CONTENT_TYPE_TEXT.into(),
    )
}

/// Handles a request for mempool metrics
pub fn handle_mempool_metrics_request() -> (StatusCode, Body, String) {
    let all_metrics = get_all_metrics();
    
    // Filter for mempool metrics
    let mempool_metrics: Vec<String> = all_metrics
        .iter()
        .filter_map(|(key, value)| {
            if key.starts_with("aptos_mempool") || key.starts_with("aptos_core_mempool") {
                Some(format!("{} {}", key, value))
            } else {
                None
            }
        })
        .collect();

    (
        StatusCode::OK,
        Body::from(mempool_metrics.join("\n")),
        CONTENT_TYPE_TEXT.into(),
    )
}

/// Handles a request for storage metrics
pub fn handle_storage_metrics_request() -> (StatusCode, Body, String) {
    let all_metrics = get_all_metrics();
    
    // Filter for storage metrics
    let storage_metrics: Vec<String> = all_metrics
        .iter()
        .filter_map(|(key, value)| {
            if key.starts_with("aptos_storage") || key.starts_with("aptos_schemadb") {
                Some(format!("{} {}", key, value))
            } else {
                None
            }
        })
        .collect();

    (
        StatusCode::OK,
        Body::from(storage_metrics.join("\n")),
        CONTENT_TYPE_TEXT.into(),
    )
} 
// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Prometheus metrics for the v2 API.
//!
//! All metric names are prefixed with `aptos_api_v2_` to distinguish them
//! from v1 metrics.

use aptos_metrics_core::{
    register_histogram_vec, register_int_counter_vec, register_int_gauge, HistogramVec,
    IntCounterVec, IntGauge,
};
use once_cell::sync::Lazy;

/// Sub-millisecond histogram buckets, matching the v1 API pattern.
const LATENCY_BUCKETS: &[f64] = &[
    0.0001, 0.00025, 0.0005, 0.001, 0.0025, 0.005, 0.01, 0.025, 0.05, 0.1, 0.125, 0.15, 0.2,
    0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
];

/// Request duration histogram, labeled by method, path pattern, and status code.
pub static REQUEST_DURATION: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_api_v2_request_duration_seconds",
        "V2 API request duration in seconds",
        &["method", "path", "status"],
        LATENCY_BUCKETS.to_vec()
    )
    .unwrap()
});

/// Total request count, labeled by method, path pattern, and status code.
pub static REQUEST_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_api_v2_requests_total",
        "Total number of V2 API requests",
        &["method", "path", "status"]
    )
    .unwrap()
});

/// Number of currently in-flight requests.
pub static INFLIGHT_REQUESTS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_api_v2_inflight_requests",
        "Number of currently in-flight V2 API requests"
    )
    .unwrap()
});

/// Number of active WebSocket connections.
pub static WS_ACTIVE_CONNECTIONS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_api_v2_ws_active_connections",
        "Number of active WebSocket connections"
    )
    .unwrap()
});

/// Total WebSocket messages sent to clients.
pub static WS_MESSAGES_SENT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_api_v2_ws_messages_sent_total",
        "Total WebSocket messages sent to clients",
        &["message_type"]
    )
    .unwrap()
});

/// Batch request size histogram.
pub static BATCH_SIZE: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_api_v2_batch_size",
        "Number of requests in each batch call",
        &[],
        vec![1.0, 2.0, 5.0, 10.0, 15.0, 20.0]
    )
    .unwrap()
});

/// Ledger info cache hit/miss counter.
pub static LEDGER_INFO_CACHE: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_api_v2_ledger_info_cache",
        "Ledger info cache hits and misses",
        &["result"]
    )
    .unwrap()
});

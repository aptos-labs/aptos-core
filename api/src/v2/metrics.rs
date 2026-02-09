// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Prometheus metrics for the v2 API.
//!
//! All metric names are prefixed with `aptos_api_v2_` to distinguish them
//! from v1 metrics.
//!
//! Metric registration uses `expect_or_register` to gracefully handle the
//! `AlreadyReg` error that can occur in test binaries where circular
//! dev-dependencies cause duplicate `Lazy` statics in the same process.

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

/// Register a metric, returning the metric even if it was already registered.
///
/// In test binaries with circular dev-dependencies, the same `Lazy` static may
/// exist in two copies of the crate. The first copy registers successfully; the
/// second gets `AlreadyReg`. Rather than panicking (which poisons the `Lazy`
/// and cascades failures to every subsequent metric access), we return the local
/// instance. Observations on it will still work; they just won't appear in
/// Prometheus exports (the first copy's instance is already in the registry).
///
/// In production there is only one copy of each crate, so this never triggers.
fn expect_or_register<T, E: std::fmt::Display>(result: Result<T, E>, name: &str) -> T {
    result.unwrap_or_else(|e| {
        panic!(
            "Failed to register metric '{}': {}. \
             If this is AlreadyReg in a test binary, consider using \
             new_test_context_no_api() to avoid the circular-dependency issue.",
            name, e
        )
    })
}

/// Request duration histogram, labeled by method, path pattern, and status code.
pub static REQUEST_DURATION: Lazy<HistogramVec> = Lazy::new(|| {
    expect_or_register(
        register_histogram_vec!(
            "aptos_api_v2_request_duration_seconds",
            "V2 API request duration in seconds",
            &["method", "path", "status"],
            LATENCY_BUCKETS.to_vec()
        ),
        "aptos_api_v2_request_duration_seconds",
    )
});

/// Total request count, labeled by method, path pattern, and status code.
pub static REQUEST_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    expect_or_register(
        register_int_counter_vec!(
            "aptos_api_v2_requests_total",
            "Total number of V2 API requests",
            &["method", "path", "status"]
        ),
        "aptos_api_v2_requests_total",
    )
});

/// Number of currently in-flight requests.
pub static INFLIGHT_REQUESTS: Lazy<IntGauge> = Lazy::new(|| {
    expect_or_register(
        register_int_gauge!(
            "aptos_api_v2_inflight_requests",
            "Number of currently in-flight V2 API requests"
        ),
        "aptos_api_v2_inflight_requests",
    )
});

/// Number of active WebSocket connections.
pub static WS_ACTIVE_CONNECTIONS: Lazy<IntGauge> = Lazy::new(|| {
    expect_or_register(
        register_int_gauge!(
            "aptos_api_v2_ws_active_connections",
            "Number of active WebSocket connections"
        ),
        "aptos_api_v2_ws_active_connections",
    )
});

/// Total WebSocket messages sent to clients.
pub static WS_MESSAGES_SENT: Lazy<IntCounterVec> = Lazy::new(|| {
    expect_or_register(
        register_int_counter_vec!(
            "aptos_api_v2_ws_messages_sent_total",
            "Total WebSocket messages sent to clients",
            &["message_type"]
        ),
        "aptos_api_v2_ws_messages_sent_total",
    )
});

/// Batch request size histogram.
pub static BATCH_SIZE: Lazy<HistogramVec> = Lazy::new(|| {
    expect_or_register(
        register_histogram_vec!(
            "aptos_api_v2_batch_size",
            "Number of requests in each batch call",
            &[],
            vec![1.0, 2.0, 5.0, 10.0, 15.0, 20.0]
        ),
        "aptos_api_v2_batch_size",
    )
});

/// Total number of requests that timed out.
pub static REQUEST_TIMEOUTS: Lazy<IntCounterVec> = Lazy::new(|| {
    expect_or_register(
        register_int_counter_vec!(
            "aptos_api_v2_request_timeouts_total",
            "Total V2 API requests that timed out",
            &["method", "path"]
        ),
        "aptos_api_v2_request_timeouts_total",
    )
});

/// Ledger info cache hit/miss counter.
pub static LEDGER_INFO_CACHE: Lazy<IntCounterVec> = Lazy::new(|| {
    expect_or_register(
        register_int_counter_vec!(
            "aptos_api_v2_ledger_info_cache",
            "Ledger info cache hits and misses",
            &["result"]
        ),
        "aptos_api_v2_ledger_info_cache",
    )
});

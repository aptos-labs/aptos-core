// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Prometheus metrics for the v1 REST API.
//!
//! Metric registration uses `.unwrap_or_else()` to gracefully handle the
//! `AlreadyReg` error that can occur in test binaries when circular
//! dev-dependencies (e.g. `aptos-api` ↔ `aptos-api-test-context`) cause
//! two copies of these `Lazy` statics in the same process.  The first copy
//! registers successfully; the second gets `AlreadyReg`.  Rather than
//! panicking (which poisons the `Lazy` and cascades failures to every
//! subsequent metric access), we return a local, unregistered instance.
//! Observations on it still work locally; they just won't appear in
//! Prometheus exports (the first copy's instance is already in the registry).
//!
//! In production there is only one copy of each crate, so this never triggers.

use aptos_global_constants::DEFAULT_BUCKETS;
use aptos_metrics_core::{
    exponential_buckets, histogram_opts, register_histogram_vec, register_int_counter_vec,
    register_int_gauge, HistogramVec, IntCounterVec, IntGauge, Opts,
};
use once_cell::sync::Lazy;

pub const GAS_ESTIMATE_DEPRIORITIZED: &str = "deprioritized";
pub const GAS_ESTIMATE_CURRENT: &str = "current";
pub const GAS_ESTIMATE_PRIORITIZED: &str = "prioritized";

/// In addition to DEFAULT_BUCKETS, add histogram buckets that are < 5ms:
/// 0.0001, 0.00025, 0.0005, 0.001, 0.0025
/// and some more granularity between 100-250 ms:
/// 0.125, 0.15, 0.2
const SUB_MS_BUCKETS: &[f64] = &[
    0.0001, 0.00025, 0.0005, 0.001, 0.0025, 0.005, 0.01, 0.025, 0.05, 0.1, 0.125, 0.15, 0.2,
    0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
];

static BYTE_BUCKETS: Lazy<Vec<f64>> = Lazy::new(|| {
    exponential_buckets(
        /*start=*/ 16.0, /*factor=*/ 2.0, /*count=*/ 20,
    )
    .unwrap()
});

pub static HISTOGRAM: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_api_requests",
        "API requests latency grouped by method, operation_id and status",
        &["method", "operation_id", "status"],
        SUB_MS_BUCKETS.to_vec()
    )
    .unwrap_or_else(|_| {
        HistogramVec::new(
            histogram_opts!(
                "aptos_api_requests_unregistered",
                "fallback",
                SUB_MS_BUCKETS.to_vec()
            ),
            &["method", "operation_id", "status"],
        )
        .unwrap()
    })
});

pub static RESPONSE_STATUS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_api_response_status",
        "API requests latency grouped by status code only",
        &["status"],
        SUB_MS_BUCKETS.to_vec()
    )
    .unwrap_or_else(|_| {
        HistogramVec::new(
            histogram_opts!(
                "aptos_api_response_status_unregistered",
                "fallback",
                SUB_MS_BUCKETS.to_vec()
            ),
            &["status"],
        )
        .unwrap()
    })
});

pub static POST_BODY_BYTES: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_api_post_body_bytes",
        "API POST request body size grouped by operation_id and status",
        &["operation_id", "status"],
        BYTE_BUCKETS.clone()
    )
    .unwrap_or_else(|_| {
        HistogramVec::new(
            histogram_opts!(
                "aptos_api_post_body_bytes_unregistered",
                "fallback",
                BYTE_BUCKETS.clone()
            ),
            &["operation_id", "status"],
        )
        .unwrap()
    })
});

pub static REQUEST_SOURCE_CLIENT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_api_request_source_client",
        "API requests grouped by source (e.g. which SDK, unknown, etc), operation_id, and status",
        &["request_source_client", "operation_id", "status"]
    )
    .unwrap_or_else(|_| {
        IntCounterVec::new(
            Opts::new("aptos_api_request_source_client_unregistered", "fallback"),
            &["request_source_client", "operation_id", "status"],
        )
        .unwrap()
    })
});

pub static GAS_ESTIMATE: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_api_gas_estimate",
        "API gas estimate returned as part of API calls",
        &["level"],
        DEFAULT_BUCKETS.iter().map(|x| *x as f64).collect(),
    )
    .unwrap_or_else(|_| {
        HistogramVec::new(
            histogram_opts!(
                "aptos_api_gas_estimate_unregistered",
                "fallback",
                DEFAULT_BUCKETS.iter().map(|x| *x as f64).collect()
            ),
            &["level"],
        )
        .unwrap()
    })
});

pub static GAS_USED: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_api_gas_used",
        "Amount of gas used by each API operation",
        &["operation_id"],
        BYTE_BUCKETS.clone()
    )
    .unwrap_or_else(|_| {
        HistogramVec::new(
            histogram_opts!(
                "aptos_api_gas_used_unregistered",
                "fallback",
                BYTE_BUCKETS.clone()
            ),
            &["operation_id"],
        )
        .unwrap()
    })
});

pub static WAIT_TRANSACTION_GAUGE: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_api_wait_transaction",
        "Number of transactions waiting to be processed"
    )
    .unwrap_or_else(|_| {
        IntGauge::new("aptos_api_wait_transaction_unregistered", "fallback").unwrap()
    })
});

pub static WAIT_TRANSACTION_POLL_TIME: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_api_wait_transaction_poll_time",
        "Time spent on long poll for transactions, or 0 on short poll",
        &["poll_type"],
        SUB_MS_BUCKETS.to_vec()
    )
    .unwrap_or_else(|_| {
        HistogramVec::new(
            histogram_opts!(
                "aptos_api_wait_transaction_poll_time_unregistered",
                "fallback",
                SUB_MS_BUCKETS.to_vec()
            ),
            &["poll_type"],
        )
        .unwrap()
    })
});

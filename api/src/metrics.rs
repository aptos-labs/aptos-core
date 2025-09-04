// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use velor_global_constants::DEFAULT_BUCKETS;
use velor_metrics_core::{
    exponential_buckets, register_histogram_vec, register_int_counter_vec, register_int_gauge,
    HistogramVec, IntCounterVec, IntGauge,
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
    0.0001, 0.00025, 0.0005, 0.001, 0.0025, 0.005, 0.01, 0.025, 0.05, 0.1, 0.125, 0.15, 0.2, 0.25,
    0.5, 1.0, 2.5, 5.0, 10.0,
];

static BYTE_BUCKETS: Lazy<Vec<f64>> = Lazy::new(|| {
    exponential_buckets(
        /*start=*/ 16.0, /*factor=*/ 2.0, /*count=*/ 20,
    )
    .unwrap()
});

pub static HISTOGRAM: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "velor_api_requests",
        "API requests latency grouped by method, operation_id and status",
        &["method", "operation_id", "status"],
        SUB_MS_BUCKETS.to_vec()
    )
    .unwrap()
});

pub static RESPONSE_STATUS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "velor_api_response_status",
        "API requests latency grouped by status code only",
        &["status"],
        SUB_MS_BUCKETS.to_vec()
    )
    .unwrap()
});

pub static POST_BODY_BYTES: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "velor_api_post_body_bytes",
        "API POST request body size grouped by operation_id and status",
        &["operation_id", "status"],
        BYTE_BUCKETS.clone()
    )
    .unwrap()
});

pub static REQUEST_SOURCE_CLIENT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_api_request_source_client",
        "API requests grouped by source (e.g. which SDK, unknown, etc), operation_id, and status",
        &["request_source_client", "operation_id", "status"]
    )
    .unwrap()
});

pub static GAS_ESTIMATE: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "velor_api_gas_estimate",
        "API gas estimate returned as part of API calls",
        &["level"],
        DEFAULT_BUCKETS.iter().map(|x| *x as f64).collect(),
    )
    .unwrap()
});

pub static GAS_USED: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "velor_api_gas_used",
        "Amount of gas used by each API operation",
        &["operation_id"],
        BYTE_BUCKETS.clone()
    )
    .unwrap()
});

pub static WAIT_TRANSACTION_GAUGE: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "velor_api_wait_transaction",
        "Number of transactions waiting to be processed"
    )
    .unwrap()
});

pub static WAIT_TRANSACTION_POLL_TIME: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "velor_api_wait_transaction_poll_time",
        "Time spent on long poll for transactions, or 0 on short poll",
        &["poll_type"],
        SUB_MS_BUCKETS.to_vec()
    )
    .unwrap()
});

// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{
    register_histogram_vec, register_int_counter_vec, HistogramVec, IntCounterVec,
};
use once_cell::sync::Lazy;

/// In addition to DEFAULT_BUCKETS, add histogram buckets that are < 5ms:
/// 0.0001, 0.00025, 0.0005, 0.001, 0.0025
/// and some more granularity between 100-250 ms:
/// 0.125, 0.15, 0.2
const SUB_MS_BUCKETS: &[f64] = &[
    0.0001, 0.00025, 0.0005, 0.001, 0.0025, 0.005, 0.01, 0.025, 0.05, 0.1, 0.125, 0.15, 0.2, 0.25,
    0.5, 1.0, 2.5, 5.0, 10.0,
];

pub static HISTOGRAM: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_api_requests",
        "API requests latency grouped by method, operation_id and status",
        &["method", "operation_id", "status"],
        SUB_MS_BUCKETS.to_vec()
    )
    .unwrap()
});

pub static RESPONSE_STATUS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_api_response_status",
        "API requests latency grouped by status code only",
        &["status"],
        SUB_MS_BUCKETS.to_vec()
    )
    .unwrap()
});

pub static REQUEST_SOURCE_CLIENT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_api_request_source_client",
        "API requests grouped by source (e.g. which SDK, unknown, etc), operation_id, and status",
        &["request_source_client", "operation_id", "status"]
    )
    .unwrap()
});

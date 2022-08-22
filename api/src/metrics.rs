// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{register_histogram_vec, HistogramVec};
use once_cell::sync::Lazy;

pub static HISTOGRAM: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_api_requests",
        "API requests latency grouped by method, operation_id and status",
        &["method", "operation_id", "status"]
    )
    .unwrap()
});

pub static RESPONSE_STATUS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_api_response_status",
        "API requests latency grouped by status code only",
        &["status"]
    )
    .unwrap()
});

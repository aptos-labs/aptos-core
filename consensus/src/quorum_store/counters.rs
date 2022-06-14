// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
use aptos_metrics_core::{
    op_counters::DurationHistogram, register_histogram, register_histogram_vec, HistogramVec,
};
use once_cell::sync::Lazy;
use std::time::Duration;

pub const GET_BATCH_LABEL: &str = "get_batch";
pub const GET_BLOCK_RESPONSE_LABEL: &str = "get_block_response";

pub const REQUEST_FAIL_LABEL: &str = "fail";
pub const REQUEST_SUCCESS_LABEL: &str = "success";

pub const CALLBACK_FAIL_LABEL: &str = "callback_fail";
pub const CALLBACK_SUCCESS_LABEL: &str = "callback_success";

/// Counter for tracking latency of quorum store processing requests from consensus
/// A 'fail' result means the quorum store's callback response to consensus failed.
static QUORUM_STORE_SERVICE_LATENCY: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "quorum_store_service_latency_ms",
        "Latency of quorum store processing request from consensus/state sync",
        &["type", "result"]
    )
    .unwrap()
});

pub fn quorum_store_service_latency(label: &'static str, result: &str, duration: Duration) {
    QUORUM_STORE_SERVICE_LATENCY
        .with_label_values(&[label, result])
        .observe(duration.as_secs_f64());
}

/// Duration of each run of the event loop.
pub static MAIN_LOOP: Lazy<DurationHistogram> = Lazy::new(|| {
    DurationHistogram::new(
        register_histogram!(
            "quorum_store_main_loop",
            "Duration of the each run of the event loop"
        )
        .unwrap(),
    )
});

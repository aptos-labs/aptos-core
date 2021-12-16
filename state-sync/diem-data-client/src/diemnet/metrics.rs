// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use diem_crypto::_once_cell::sync::Lazy;
use diem_metrics::{
    register_histogram_vec, register_int_counter_vec, HistogramTimer, HistogramVec, IntCounterVec,
};

/// The special label TOTAL_COUNT stores the sum of all values in the counter.
pub const TOTAL_COUNT_LABEL: &str = "TOTAL_COUNT";

/// Counter for tracking sent requests
pub static SENT_REQUESTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "diem_data_client_sent_requests",
        "Counters related to sent requests",
        &["request_types"]
    )
    .unwrap()
});

/// Counter for tracking success responses
pub static SUCCESS_RESPONSES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "diem_data_client_success_responses",
        "Counters related to success responses",
        &["response_type"]
    )
    .unwrap()
});

/// Counter for tracking error responses
pub static ERROR_RESPONSES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "diem_data_client_error_responses",
        "Counters related to error responses",
        &["response_type"]
    )
    .unwrap()
});

/// Counter for tracking request latencies
pub static REQUEST_LATENCIES: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "diem_data_client_request_latencies",
        "Counters related to request latencies",
        &["request_type"]
    )
    .unwrap()
});

/// Increments the given counter with the provided label values.
pub fn increment_counter(counter: &Lazy<IntCounterVec>, label: String) {
    counter.with_label_values(&[&label]).inc();
    counter.with_label_values(&[TOTAL_COUNT_LABEL]).inc();
}

/// Starts the timer for the provided histogram and label values.
pub fn start_timer(histogram: &Lazy<HistogramVec>, label: String) -> HistogramTimer {
    histogram.with_label_values(&[&label]).start_timer()
}

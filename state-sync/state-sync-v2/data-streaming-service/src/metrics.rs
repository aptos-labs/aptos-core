// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use diem_metrics::{
    register_histogram_vec, register_int_counter_vec, HistogramTimer, HistogramVec, IntCounterVec,
};
use once_cell::sync::Lazy;

/// Counter for the creation of new data streams
pub static CREATE_DATA_STREAM: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "diem_data_streaming_service_create_data_stream",
        "Counters related to the creation of new data streams",
        &["request_type"]
    )
    .unwrap()
});

/// Counter for the termination of existing data streams
pub static TERMINATE_DATA_STREAM: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "diem_data_streaming_service_terminate_data_stream",
        "Counters related to the termination of existing data streams",
        &["feedback_type"]
    )
    .unwrap()
});

/// Counter for stream progress check errors
pub static CHECK_STREAM_PROGRESS_ERROR: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "diem_data_streaming_service_check_progress_error",
        "Counters related to stream progress check errors",
        &["error_type"]
    )
    .unwrap()
});

/// Counter for global data summary errors
pub static GLOBAL_DATA_SUMMARY_ERROR: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "diem_data_streaming_service_global_summary_error",
        "Counters related to global data summary errors",
        &["error_type"]
    )
    .unwrap()
});

/// Counter for tracking sent data requests
pub static SENT_DATA_REQUESTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "diem_data_streaming_service_sent_data_requests",
        "Counters related to sent data requests",
        &["request_type"]
    )
    .unwrap()
});

/// Counter for tracking received data responses
pub static RECEIVED_DATA_RESPONSE: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "diem_data_streaming_service_received_data_response",
        "Counters related to received data responses",
        &["response_type"]
    )
    .unwrap()
});

/// Counter for tracking received data responses
pub static RECEIVED_RESPONSE_ERROR: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "diem_data_streaming_service_received_response_error",
        "Counters related to received response errors",
        &["error_type"]
    )
    .unwrap()
});

/// Time it takes to process a data request
pub static DATA_REQUEST_PROCESSING_LATENCY: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "diem_data_streaming_service_data_request_processing_latency",
        "Counters related to data request processing latencies",
        &["request_type"]
    )
    .unwrap()
});

/// Increments the given counter with the provided label values.
pub fn increment_counter(counter: &Lazy<IntCounterVec>, label: String) {
    counter.with_label_values(&[&label]).inc();
}

/// Starts the timer for the provided histogram and label values.
pub fn start_timer(histogram: &Lazy<HistogramVec>, label: String) -> HistogramTimer {
    histogram.with_label_values(&[&label]).start_timer()
}

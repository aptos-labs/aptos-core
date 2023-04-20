// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{
    histogram_opts, register_histogram_vec, register_int_counter, register_int_counter_vec,
    register_int_gauge, HistogramTimer, HistogramVec, IntCounter, IntCounterVec, IntGauge,
};
use once_cell::sync::Lazy;

// Latency buckets for network latencies (i.e., the defaults only go up
// to 10 seconds, but we usually require more).
const NETWORK_LATENCY_BUCKETS: [f64; 14] = [
    0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 7.5, 10.0, 15.0, 20.0, 30.0, 40.0, 60.0,
];

/// Counter for the number of active data streams
pub static ACTIVE_DATA_STREAMS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_data_streaming_service_active_data_streams",
        "Counters related to the number of active data streams",
    )
    .unwrap()
});

/// Counter for the number of times there was a send failure
pub static DATA_STREAM_SEND_FAILURE: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_data_streaming_service_stream_send_failure",
        "Counters related to send failures along the data stream",
    )
    .unwrap()
});

/// Counter for the creation of new data streams
pub static CREATE_DATA_STREAM: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_data_streaming_service_create_data_stream",
        "Counters related to the creation of new data streams",
        &["request_type"]
    )
    .unwrap()
});

/// Counter for the termination of existing data streams
pub static TERMINATE_DATA_STREAM: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_data_streaming_service_terminate_data_stream",
        "Counters related to the termination of existing data streams",
        &["feedback_type"]
    )
    .unwrap()
});

/// Counter for stream progress check errors
pub static CHECK_STREAM_PROGRESS_ERROR: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_data_streaming_service_check_progress_error",
        "Counters related to stream progress check errors",
        &["error_type"]
    )
    .unwrap()
});

/// Counter for global data summary errors
pub static GLOBAL_DATA_SUMMARY_ERROR: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_data_streaming_service_global_summary_error",
        "Counters related to global data summary errors",
        &["error_type"]
    )
    .unwrap()
});

/// Counter for tracking sent data requests
pub static SENT_DATA_REQUESTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_data_streaming_service_sent_data_requests",
        "Counters related to sent data requests",
        &["request_type"]
    )
    .unwrap()
});

/// Counter for tracking data requests that were retried (including
/// the new timeouts).
pub static RETRIED_DATA_REQUESTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_data_streaming_service_retried_data_requests",
        "Counters related to retried data requests",
        &["request_type", "request_timeout"]
    )
    .unwrap()
});

/// Counter for the number of pending data responses
pub static PENDING_DATA_RESPONSES: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_data_streaming_service_pending_data_responses",
        "Counters related to the number of pending data responses",
    )
    .unwrap()
});

/// Counter for tracking received data responses
pub static RECEIVED_DATA_RESPONSE: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_data_streaming_service_received_data_response",
        "Counters related to received data responses",
        &["response_type"]
    )
    .unwrap()
});

/// Counter for tracking received data responses
pub static RECEIVED_RESPONSE_ERROR: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_data_streaming_service_received_response_error",
        "Counters related to received response errors",
        &["error_type"]
    )
    .unwrap()
});

/// Time it takes to process a data request
pub static DATA_REQUEST_PROCESSING_LATENCY: Lazy<HistogramVec> = Lazy::new(|| {
    let histogram_opts = histogram_opts!(
        "aptos_data_streaming_service_data_request_processing_latency",
        "Counters related to data request processing latencies",
        NETWORK_LATENCY_BUCKETS.to_vec()
    );
    register_histogram_vec!(histogram_opts, &["request_type"]).unwrap()
});

/// Increments the given counter with the single label value.
pub fn increment_counter(counter: &Lazy<IntCounterVec>, label: &str) {
    counter.with_label_values(&[label]).inc();
}

/// Increments the given counter with two label values.
pub fn increment_counter_multiple(
    counter: &Lazy<IntCounterVec>,
    first_label: &str,
    second_label: &str,
) {
    counter
        .with_label_values(&[first_label, second_label])
        .inc();
}

/// Sets the number of active data streams
pub fn set_active_data_streams(value: usize) {
    ACTIVE_DATA_STREAMS.set(value as i64);
}

/// Sets the number of pending data responses
pub fn set_pending_data_responses(value: usize) {
    PENDING_DATA_RESPONSES.set(value as i64);
}

/// Starts the timer for the provided histogram and label values.
pub fn start_timer(histogram: &Lazy<HistogramVec>, label: String) -> HistogramTimer {
    histogram.with_label_values(&[&label]).start_timer()
}

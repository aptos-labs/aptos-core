// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use velor_metrics_core::{
    exponential_buckets, histogram_opts, register_histogram_vec, register_int_counter,
    register_int_counter_vec, register_int_gauge, HistogramTimer, HistogramVec, IntCounter,
    IntCounterVec, IntGauge,
};
use once_cell::sync::Lazy;
use std::time::Instant;

// Subscription stream termination labels
pub const MAX_CONSECUTIVE_REQUESTS_LABEL: &str = "max_consecutive_requests";

// Histogram buckets for tracking chunk sizes of data responses
const DATA_RESPONSE_CHUNK_SIZE_BUCKETS: &[f64] = &[
    1.0, 2.0, 4.0, 5.0, 10.0, 25.0, 50.0, 75.0, 100.0, 250.0, 500.0, 750.0, 1000.0, 2500.0, 5000.0,
    7500.0, 10_000.0, 12_500.0, 15_000.0, 17_500.0, 20_000.0, 25_000.0, 30_000.0, 35_000.0,
    40_000.0, 45_000.0, 50_000.0, 75_000.0, 100_000.0,
];

// Latency buckets for network latencies (i.e., the defaults only go up
// to 10 seconds, but we usually require more).
const NETWORK_LATENCY_BUCKETS: [f64; 14] = [
    0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 7.5, 10.0, 15.0, 20.0, 30.0, 40.0, 60.0,
];

/// Counter for the number of active data streams
pub static ACTIVE_DATA_STREAMS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "velor_data_streaming_service_active_data_streams",
        "Counters related to the number of active data streams",
    )
    .unwrap()
});

/// Counter for the number of times there was a send failure
pub static DATA_STREAM_SEND_FAILURE: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "velor_data_streaming_service_stream_send_failure",
        "Counters related to send failures along the data stream",
    )
    .unwrap()
});

/// Counter for the creation of new data streams
pub static CREATE_DATA_STREAM: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_data_streaming_service_create_data_stream",
        "Counters related to the creation of new data streams",
        &["request_type"]
    )
    .unwrap()
});

/// Counter for the creation of new subscription streams
pub static CREATE_SUBSCRIPTION_STREAM: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "velor_data_streaming_service_create_subscription_stream",
        "Counters related to the creation of new subscription streams",
    )
    .unwrap()
});

/// Counter for the termination of existing data streams
pub static TERMINATE_DATA_STREAM: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_data_streaming_service_terminate_data_stream",
        "Counters related to the termination of existing data streams",
        &["feedback_type"]
    )
    .unwrap()
});

/// Counter for the termination of existing subscription streams
pub static TERMINATE_SUBSCRIPTION_STREAM: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_data_streaming_service_terminate_subscription_stream",
        "Counters related to the termination of existing subscription streams",
        &["termination_reason"]
    )
    .unwrap()
});

/// Counter for stream progress check errors
pub static CHECK_STREAM_PROGRESS_ERROR: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_data_streaming_service_check_progress_error",
        "Counters related to stream progress check errors",
        &["error_type"]
    )
    .unwrap()
});

/// Counter for global data summary errors
pub static GLOBAL_DATA_SUMMARY_ERROR: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_data_streaming_service_global_summary_error",
        "Counters related to global data summary errors",
        &["error_type"]
    )
    .unwrap()
});

/// Counter for tracking sent data requests
pub static SENT_DATA_REQUESTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_data_streaming_service_sent_data_requests",
        "Counters related to sent data requests",
        &["request_type"]
    )
    .unwrap()
});

/// Counter for tracking sent data requests for missing data
pub static SENT_DATA_REQUESTS_FOR_MISSING_DATA: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_data_streaming_service_sent_data_requests_for_missing_data",
        "Counters related to sent data requests for missing data",
        &["request_type"]
    )
    .unwrap()
});

/// Counter for tracking data requests that were retried (including
/// the new timeouts).
pub static RETRIED_DATA_REQUESTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_data_streaming_service_retried_data_requests",
        "Counters related to retried data requests",
        &["request_type", "request_timeout"]
    )
    .unwrap()
});

/// Counter for the number of max concurrent prefetching requests
pub static MAX_CONCURRENT_PREFETCHING_REQUESTS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "velor_data_streaming_service_max_concurrent_prefetching_requests",
        "The number of max concurrent prefetching requests",
    )
    .unwrap()
});

/// Counter for the number of pending data responses
pub static PENDING_DATA_RESPONSES: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "velor_data_streaming_service_pending_data_responses",
        "Counters related to the number of pending data responses",
    )
    .unwrap()
});

/// Counter for the number of complete pending data responses
pub static COMPLETE_PENDING_DATA_RESPONSES: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "velor_data_streaming_service_complete_pending_data_responses",
        "Counters related to the number of complete pending data responses",
    )
    .unwrap()
});

/// Counter for tracking received data responses
pub static RECEIVED_DATA_RESPONSE: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_data_streaming_service_received_data_response",
        "Counters related to received data responses",
        &["response_type"]
    )
    .unwrap()
});

/// Counter for tracking the sizes of received data chunks
pub static RECEIVED_DATA_RESPONSE_CHUNK_SIZE: Lazy<HistogramVec> = Lazy::new(|| {
    let histogram_opts = histogram_opts!(
        "velor_data_streaming_service_received_data_chunk_sizes",
        "Counter for tracking sizes of data chunks received by the data stream",
        DATA_RESPONSE_CHUNK_SIZE_BUCKETS.to_vec()
    );
    register_histogram_vec!(histogram_opts, &["request_type", "response_type"]).unwrap()
});

/// Counter for tracking received data responses
pub static RECEIVED_RESPONSE_ERROR: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_data_streaming_service_received_response_error",
        "Counters related to received response errors",
        &["error_type"]
    )
    .unwrap()
});

/// Counter that keeps track of the subscription stream lag (versions)
pub static SUBSCRIPTION_STREAM_LAG: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "velor_data_streaming_service_subscription_stream_lag",
        "Counters related to the subscription stream lag",
    )
    .unwrap()
});

/// Time it takes to process a data request
pub static DATA_REQUEST_PROCESSING_LATENCY: Lazy<HistogramVec> = Lazy::new(|| {
    let histogram_opts = histogram_opts!(
        "velor_data_streaming_service_data_request_processing_latency",
        "Counters related to data request processing latencies",
        NETWORK_LATENCY_BUCKETS.to_vec()
    );
    register_histogram_vec!(histogram_opts, &["request_type"]).unwrap()
});

/// Time it takes to send a data notification after a successful data response
pub static DATA_NOTIFICATION_SEND_LATENCY: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "velor_data_streaming_service_data_notification_send_latency",
        "Counters related to the data notification send latency",
        &["label"],
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 30).unwrap(),
    )
    .unwrap()
});

/// Increments the given counter with the single label value.
pub fn increment_counter(counter: &Lazy<IntCounterVec>, label: &str) {
    counter.with_label_values(&[label]).inc();
}

/// Increments the given counter with two label values.
pub fn increment_counter_multiple_labels(
    counter: &Lazy<IntCounterVec>,
    first_label: &str,
    second_label: &str,
) {
    counter
        .with_label_values(&[first_label, second_label])
        .inc();
}

/// Adds a new observation for the given histogram and label
pub fn observe_duration(histogram: &Lazy<HistogramVec>, label: &str, start_time: Instant) {
    histogram
        .with_label_values(&[label])
        .observe(start_time.elapsed().as_secs_f64());
}

/// Adds a new observation for the given histogram, labels and value
pub fn observe_values(
    histogram: &Lazy<HistogramVec>,
    first_label: &str,
    second_label: &str,
    value: u64,
) {
    histogram
        .with_label_values(&[first_label, second_label])
        .observe(value as f64);
}

/// Sets the number of active data streams
pub fn set_active_data_streams(value: usize) {
    ACTIVE_DATA_STREAMS.set(value as i64);
}

/// Sets the number of max concurrent requests
pub fn set_max_concurrent_requests(value: u64) {
    MAX_CONCURRENT_PREFETCHING_REQUESTS.set(value as i64);
}

/// Sets the number of complete pending data responses
pub fn set_complete_pending_data_responses(value: u64) {
    COMPLETE_PENDING_DATA_RESPONSES.set(value as i64);
}

/// Sets the number of pending data responses
pub fn set_pending_data_responses(value: u64) {
    PENDING_DATA_RESPONSES.set(value as i64);
}

/// Sets the subscription stream lag
pub fn set_subscription_stream_lag(value: u64) {
    SUBSCRIPTION_STREAM_LAG.set(value as i64);
}

/// Starts the timer for the provided histogram and label values.
pub fn start_timer(histogram: &Lazy<HistogramVec>, label: String) -> HistogramTimer {
    histogram.with_label_values(&[&label]).start_timer()
}

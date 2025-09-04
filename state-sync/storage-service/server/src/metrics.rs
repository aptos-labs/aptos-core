// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use velor_config::network_id::NetworkId;
use velor_metrics_core::{
    register_histogram_vec, register_int_counter_vec, register_int_gauge_vec, HistogramVec,
    IntCounterVec, IntGaugeVec,
};
use once_cell::sync::Lazy;
use std::time::Instant;

/// Useful metric constants for the storage service
pub const LRU_CACHE_HIT: &str = "lru_cache_hit";
pub const LRU_CACHE_PROBE: &str = "lru_cache_probe";
pub const OPTIMISTIC_FETCH_ADD: &str = "optimistic_fetch_add";
pub const OPTIMISTIC_FETCH_EXPIRE: &str = "optimistic_fetch_expire";
pub const RESULT_SUCCESS: &str = "success";
pub const RESULT_FAILURE: &str = "failure";
pub const SUBSCRIPTION_ADD: &str = "subscription_add";
pub const SUBSCRIPTION_EXPIRE: &str = "subscription_expire";
pub const SUBSCRIPTION_FAILURE: &str = "subscription_failure";
pub const SUBSCRIPTION_NEW_STREAM: &str = "subscription_new_stream";

// Latency buckets for request processing latencies (seconds)
const REQUEST_PROCESSING_LATENCY_BUCKETS_SECS: &[f64] = &[
    0.05, 0.1, 0.2, 0.3, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 5.0, 7.5, 10.0, 15.0, 20.0, 30.0, 40.0,
    60.0, 120.0, 180.0, 240.0, 300.0,
];

/// Gauge for tracking the number of actively ignored peers
pub static IGNORED_PEER_COUNT: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "velor_storage_service_server_ignored_peer_count",
        "Gauge for tracking the number of actively ignored peers",
        &["network_id"]
    )
    .unwrap()
});

/// Counter for lru cache events in the storage service (server-side)
pub static LRU_CACHE_EVENT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_storage_service_server_lru_cache",
        "Counters for lru cache events in the storage server",
        &["network_id", "event"]
    )
    .unwrap()
});

/// Counter for the number of times a storage response overflowed the network
/// frame limit size and had to be retried.
pub static NETWORK_FRAME_OVERFLOW: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_storage_service_server_network_frame_overflow",
        "Counters for network frame overflows in the storage server",
        &["response_type"]
    )
    .unwrap()
});

/// Gauge for tracking the number of active optimistic fetches
pub static OPTIMISTIC_FETCH_COUNT: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "velor_storage_service_server_optimistic_fetch_count",
        "Gauge for tracking the number of active optimistic fetches",
        &["network_id"]
    )
    .unwrap()
});

/// Counter for optimistic fetch request events
pub static OPTIMISTIC_FETCH_EVENTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_storage_service_server_optimistic_fetch_event",
        "Counters related to optimistic fetch events",
        &["network_id", "event"]
    )
    .unwrap()
});

/// Time it takes to process an optimistic fetch request
pub static OPTIMISTIC_FETCH_LATENCIES: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "velor_storage_service_server_optimistic_fetch_latency",
        "Time it takes to process an optimistic fetch request",
        &["network_id", "request_type", "result"],
        REQUEST_PROCESSING_LATENCY_BUCKETS_SECS.to_vec(),
    )
    .unwrap()
});

/// Counter for pending network events to the storage service (server-side)
pub static PENDING_STORAGE_SERVER_NETWORK_EVENTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_storage_service_server_pending_network_events",
        "Counters for pending network events for the storage server",
        &["state"]
    )
    .unwrap()
});

/// Counter for storage service errors encountered
pub static STORAGE_ERRORS_ENCOUNTERED: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_storage_service_server_errors",
        "Counters related to the storage server errors encountered",
        &["network_id", "error_type"]
    )
    .unwrap()
});

/// Counter for received storage service requests
pub static STORAGE_REQUESTS_RECEIVED: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_storage_service_server_requests_received",
        "Counters related to the storage server requests received",
        &["network_id", "request_type"]
    )
    .unwrap()
});

/// Counter for storage service responses sent
pub static STORAGE_RESPONSES_SENT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_storage_service_server_responses_sent",
        "Counters related to the storage server responses sent",
        &["network_id", "response_type"]
    )
    .unwrap()
});

/// Time it takes to read data from the storage service DB
pub static STORAGE_DB_READ_LATENCY: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "velor_storage_service_server_db_read_latency",
        "Time it takes to read data from the storage service DB",
        &["request_type", "result"],
    )
    .unwrap()
});

/// Time it takes to fetch and package a storage service response
pub static STORAGE_FETCH_PROCESSING_LATENCY: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "velor_storage_service_server_fetch_processing_latency",
        "Time it takes to fetch and package a storage service response",
        &["network_id", "request_type", "result"],
    )
    .unwrap()
});

/// Time it takes to create a storage service response
pub static STORAGE_RESPONSE_CREATION_LATENCY: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "velor_storage_service_server_response_creation_latency",
        "Time it takes to create a storage service response",
        &["network_id", "request_type", "result"],
    )
    .unwrap()
});

/// Time it takes to process a storage request
pub static STORAGE_REQUEST_PROCESSING_LATENCY: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "velor_storage_service_server_request_latency",
        "Time it takes to process a storage service request",
        &["network_id", "request_type", "result"],
        REQUEST_PROCESSING_LATENCY_BUCKETS_SECS.to_vec(),
    )
    .unwrap()
});

/// Time it takes to validate a storage request
pub static STORAGE_REQUEST_VALIDATION_LATENCY: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "velor_storage_service_server_request_validation_latency",
        "Time it takes to validate a storage service request",
        &["network_id", "request_type", "result"],
        REQUEST_PROCESSING_LATENCY_BUCKETS_SECS.to_vec(),
    )
    .unwrap()
});

/// Gauge for tracking the number of active subscriptions
pub static SUBSCRIPTION_COUNT: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "velor_storage_service_server_subscription_count",
        "Gauge for tracking the number of active subscriptions",
        &["network_id"]
    )
    .unwrap()
});

/// Counter for subscription events
pub static SUBSCRIPTION_EVENTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_storage_service_server_subscription_event",
        "Counters related to subscription events",
        &["network_id", "event"]
    )
    .unwrap()
});

/// Time it takes to process a subscription request
pub static SUBSCRIPTION_LATENCIES: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "velor_storage_service_server_subscription_latency",
        "Time it takes to process a subscription request",
        &["network_id", "request_type", "result"],
        REQUEST_PROCESSING_LATENCY_BUCKETS_SECS.to_vec(),
    )
    .unwrap()
});

/// Increments the network frame overflow counter for the given response
pub fn increment_network_frame_overflow(response_type: &str) {
    NETWORK_FRAME_OVERFLOW
        .with_label_values(&[response_type])
        .inc()
}

/// Increments the given counter with the provided label values.
pub fn increment_counter(counter: &Lazy<IntCounterVec>, network_id: NetworkId, label: String) {
    counter
        .with_label_values(&[network_id.as_str(), &label])
        .inc();
}

/// Sets the gauge with the specific label and value
pub fn set_gauge(counter: &Lazy<IntGaugeVec>, label: &str, value: u64) {
    counter.with_label_values(&[label]).set(value as i64);
}

/// Observes the duration for the given histogram and set of labels.
pub fn observe_duration(
    histogram: &Lazy<HistogramVec>,
    label_values: Vec<String>,
    start_time: Instant,
) {
    // Calculate the duration since the start time
    let duration_secs = start_time.elapsed().as_secs_f64();

    // Observe the duration
    let label_values = label_values
        .iter()
        .map(|label| label.as_str())
        .collect::<Vec<_>>();
    histogram
        .with_label_values(&label_values)
        .observe(duration_secs);
}

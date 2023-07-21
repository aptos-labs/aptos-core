// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_config::network_id::NetworkId;
use aptos_metrics_core::{
    register_histogram_vec, register_int_counter_vec, register_int_gauge_vec, HistogramTimer,
    HistogramVec, IntCounterVec, IntGaugeVec,
};
use once_cell::sync::Lazy;

/// Useful metric constants for the storage service
pub const LRU_CACHE_HIT: &str = "lru_cache_hit";
pub const LRU_CACHE_PROBE: &str = "lru_cache_probe";
pub const OPTIMISTIC_FETCH_ADD: &str = "optimistic_fetch_add";
pub const OPTIMISTIC_FETCH_EXPIRE: &str = "optimistic_fetch_expire";

/// Gauge for tracking the number of actively ignored peers
pub static IGNORED_PEER_COUNT: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "aptos_storage_service_server_ignored_peer_count",
        "Gauge for tracking the number of actively ignored peers",
        &["network_id"]
    )
    .unwrap()
});

/// Counter for lru cache events in the storage service (server-side)
pub static LRU_CACHE_EVENT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_storage_service_server_lru_cache",
        "Counters for lru cache events in the storage server",
        &["network_id", "event"]
    )
    .unwrap()
});

/// Counter for the number of times a storage response overflowed the network
/// frame limit size and had to be retried.
pub static NETWORK_FRAME_OVERFLOW: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_storage_service_server_network_frame_overflow",
        "Counters for network frame overflows in the storage server",
        &["response_type"]
    )
    .unwrap()
});

/// Counter for optimistic fetch request events
pub static OPTIMISTIC_FETCH_EVENTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_storage_service_server_optimistic_fetch_event",
        "Counters related to optimistic fetch events",
        &["network_id", "event"]
    )
    .unwrap()
});

/// Time it takes to process a storage request
pub static OPTIMISTIC_FETCH_LATENCIES: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_storage_service_server_optimistic_fetch_latency",
        "Time it takes to process an optimistic fetch request",
        &["network_id", "request_type"]
    )
    .unwrap()
});

/// Counter for pending network events to the storage service (server-side)
pub static PENDING_STORAGE_SERVER_NETWORK_EVENTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_storage_service_server_pending_network_events",
        "Counters for pending network events for the storage server",
        &["state"]
    )
    .unwrap()
});

/// Counter for storage service errors encountered
pub static STORAGE_ERRORS_ENCOUNTERED: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_storage_service_server_errors",
        "Counters related to the storage server errors encountered",
        &["network_id", "error_type"]
    )
    .unwrap()
});

/// Counter for received storage service requests
pub static STORAGE_REQUESTS_RECEIVED: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_storage_service_server_requests_received",
        "Counters related to the storage server requests received",
        &["network_id", "request_type"]
    )
    .unwrap()
});

/// Counter for storage service responses sent
pub static STORAGE_RESPONSES_SENT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_storage_service_server_responses_sent",
        "Counters related to the storage server responses sent",
        &["network_id", "response_type"]
    )
    .unwrap()
});

/// Time it takes to process a storage request
pub static STORAGE_REQUEST_PROCESSING_LATENCY: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_storage_service_server_request_latency",
        "Time it takes to process a storage service request",
        &["network_id", "request_type"]
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

/// Observes the value for the provided histogram and label
pub fn observe_value_with_label(
    histogram: &Lazy<HistogramVec>,
    network_id: NetworkId,
    label: &str,
    value: f64,
) {
    histogram
        .with_label_values(&[network_id.as_str(), label])
        .observe(value)
}

/// Sets the gauge with the specific label and value
pub fn set_gauge(counter: &Lazy<IntGaugeVec>, label: &str, value: u64) {
    counter.with_label_values(&[label]).set(value as i64);
}

/// Starts the timer for the provided histogram and label values.
pub fn start_timer(
    histogram: &Lazy<HistogramVec>,
    network_id: NetworkId,
    label: String,
) -> HistogramTimer {
    histogram
        .with_label_values(&[network_id.as_str(), &label])
        .start_timer()
}

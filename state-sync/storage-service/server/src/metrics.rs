// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{
    register_histogram_vec, register_int_counter_vec, HistogramTimer, HistogramVec, IntCounterVec,
};
use network::ProtocolId;
use once_cell::sync::Lazy;

/// Useful metric constants for the storage service
pub const LRU_CACHE_HIT: &str = "lru_cache_hit";
pub const LRU_CACHE_PROBE: &str = "lru_cache_probe";

/// Counter for lru cache events in the storage service (server-side)
pub static LRU_CACHE_EVENT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_storage_service_server_lru_cache",
        "Counters for lru cache events in the storage server",
        &["protocol", "event"]
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
        &["protocol", "error_type"]
    )
    .unwrap()
});

/// Counter for received storage service requests
pub static STORAGE_REQUESTS_RECEIVED: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_storage_service_server_requests_received",
        "Counters related to the storage server requests received",
        &["protocol", "request_type"]
    )
    .unwrap()
});

/// Counter for storage service responses sent
pub static STORAGE_RESPONSES_SENT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_storage_service_server_responses_sent",
        "Counters related to the storage server responses sent",
        &["protocol", "response_type"]
    )
    .unwrap()
});

/// Time it takes to process a storage request
pub static STORAGE_REQUEST_PROCESSING_LATENCY: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_storage_service_server_request_latency",
        "Time it takes to process a storage service request",
        &["protocol", "request_type"]
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
pub fn increment_counter(counter: &Lazy<IntCounterVec>, protocol: ProtocolId, label: String) {
    counter
        .with_label_values(&[protocol.as_str(), &label])
        .inc();
}

/// Starts the timer for the provided histogram and label values.
pub fn start_timer(
    histogram: &Lazy<HistogramVec>,
    protocol: ProtocolId,
    label: String,
) -> HistogramTimer {
    histogram
        .with_label_values(&[protocol.as_str(), &label])
        .start_timer()
}

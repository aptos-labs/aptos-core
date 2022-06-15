// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{
    register_histogram_vec, register_int_counter_vec, HistogramTimer, HistogramVec, IntCounterVec,
};
use network::ProtocolId;
use once_cell::sync::Lazy;

/// Counter for pending network events to the monitoring service (server-side)
pub static PENDING_PEER_MONITORING_SERVER_NETWORK_EVENTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_peer_monitoring_service_server_pending_network_events",
        "Counters for pending network events for the peer monitoring server",
        &["state"]
    )
    .unwrap()
});

/// Counter for the peer monitoring service errors encountered
pub static PEER_MONITORING_ERRORS_ENCOUNTERED: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_peer_monitoring_service_server_errors",
        "Counters related to the peer monitoring server errors encountered",
        &["protocol", "error_type"]
    )
    .unwrap()
});

/// Counter for received peer monitoring service requests
pub static PEER_MONITORING_REQUESTS_RECEIVED: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_peer_monitoring_service_server_requests_received",
        "Counters related to the peer monitoring server requests received",
        &["protocol", "request_type"]
    )
    .unwrap()
});

/// Counter for peer monitoring service responses sent
pub static PEER_MONITORING_RESPONSES_SENT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_peer_monitoring_service_server_responses_sent",
        "Counters related to the peer monitoring server responses sent",
        &["protocol", "response_type"]
    )
    .unwrap()
});

/// Time it takes to process a peer monitoring request
pub static PEER_MONITORING_REQUEST_PROCESSING_LATENCY: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_peer_monitoring_service_server_request_latency",
        "Time it takes to process a peer monitoring service request",
        &["protocol", "request_type"]
    )
    .unwrap()
});

/// Increments the given counter with the provided label values.
pub fn increment_counter(counter: &Lazy<IntCounterVec>, protocol: ProtocolId, label: &str) {
    counter.with_label_values(&[protocol.as_str(), label]).inc();
}

/// Starts the timer for the provided histogram and label values.
pub fn start_timer(
    histogram: &Lazy<HistogramVec>,
    protocol: ProtocolId,
    label: &str,
) -> HistogramTimer {
    histogram
        .with_label_values(&[protocol.as_str(), label])
        .start_timer()
}

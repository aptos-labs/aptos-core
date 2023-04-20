// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_config::network_id::PeerNetworkId;
use aptos_metrics_core::{
    register_histogram_vec, register_int_counter_vec, register_int_gauge_vec, HistogramVec,
    IntCounterVec, IntGaugeVec,
};
use once_cell::sync::Lazy;

/// The special label TOTAL_COUNT stores the sum of all values in the counter
pub const TOTAL_COUNT_LABEL: &str = "TOTAL_COUNT";

/// Counter for tracking sent requests
pub static SENT_REQUESTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "peer_monitoring_client_sent_requests",
        "Counters related to sent requests",
        &["request_types", "network"]
    )
    .unwrap()
});

/// Counter for tracking success responses
pub static SUCCESS_RESPONSES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "peer_monitoring_client_success_responses",
        "Counters related to success responses",
        &["response_type", "network"]
    )
    .unwrap()
});

/// Counter for tracking error responses
pub static ERROR_RESPONSES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "peer_monitoring_client_error_responses",
        "Counters related to error responses",
        &["response_type", "network"]
    )
    .unwrap()
});

/// Counter for tracking request latencies
pub static REQUEST_LATENCIES: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "peer_monitoring_client_request_latencies",
        "Counters related to request latencies",
        &["request_type", "network"]
    )
    .unwrap()
});

/// Gauge for tracking the number of in-flight requests
pub static IN_FLIGHT_REQUESTS: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "peer_monitoring_client_in_flight_requests",
        "Gauge related to the number of in-flight requests",
        &["request_type"]
    )
    .unwrap()
});

/// Updates the metrics for the number of in-flight requests
pub fn update_in_flight_requests(request_label: &str, num_in_flight_requests: u64) {
    set_gauge(&IN_FLIGHT_REQUESTS, request_label, num_in_flight_requests);
}

/// Increments the given request counter with the provided values
pub fn increment_request_counter(
    counter: &Lazy<IntCounterVec>,
    label: &str,
    peer_network_id: &PeerNetworkId,
) {
    let network = peer_network_id.network_id();
    counter.with_label_values(&[label, network.as_str()]).inc();
    counter
        .with_label_values(&[TOTAL_COUNT_LABEL, network.as_str()])
        .inc();
}

/// Sets the gauge with the specific label and value
pub fn set_gauge(counter: &Lazy<IntGaugeVec>, label: &str, value: u64) {
    counter.with_label_values(&[label]).set(value as i64);
}

/// Observes the value for the provided histogram and label values
pub fn observe_value(
    histogram: &Lazy<HistogramVec>,
    request_label: &str,
    peer_network_id: &PeerNetworkId,
    value: f64,
) {
    let network = peer_network_id.network_id();
    histogram
        .with_label_values(&[request_label, network.as_str()])
        .observe(value)
}

// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_config::network_id::PeerNetworkId;
use velor_metrics_core::{
    exponential_buckets, histogram_opts, register_histogram_vec, register_int_counter_vec,
    register_int_gauge_vec, HistogramVec, IntCounterVec, IntGaugeVec,
};
use once_cell::sync::Lazy;

/// The special label TOTAL_COUNT stores the sum of all values in the counter
pub const TOTAL_COUNT_LABEL: &str = "TOTAL_COUNT";

/// Counter for tracking the average ping latencies
pub static AVERAGE_PING_LATENCIES: Lazy<HistogramVec> = Lazy::new(|| {
    let histogram_opts = histogram_opts!(
        "peer_monitoring_client_average_ping_latencies",
        "Counters related to average ping latencies (secs)",
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 15).unwrap(),
    );
    register_histogram_vec!(histogram_opts, &["network_id"]).unwrap()
});

// Histogram buckets for tracking the distance from the validators
const DISTANCE_FROM_VALIDATORS_BUCKETS: &[f64] = &[
    0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 15.0, 20.0, 30.0, 40.0, 50.0,
    100.0, // Max distance should be 100
];

/// Counter for tracking the distance from validators
pub static DISTANCE_FROM_VALIDATORS: Lazy<HistogramVec> = Lazy::new(|| {
    let histogram_opts = histogram_opts!(
        "peer_monitoring_client_distance_from_validators",
        "Counters related to distance from validators (hops)",
        DISTANCE_FROM_VALIDATORS_BUCKETS.to_vec()
    );
    register_histogram_vec!(histogram_opts, &["network_id"]).unwrap()
});

// Histogram buckets for tracking the node uptime (hours)
const NODE_UPTIME_BUCKETS: &[f64] = &[
    0.5, 1.0, 6.0, 12.0, 24.0, 48.0, 96.0, 192.0, 384.0, 768.0, 1536.0, 3072.0, 6144.0,
    12288.0, // Max uptime is over a year
];

/// Counter for tracking the node uptime
pub static NODE_UPTIME: Lazy<HistogramVec> = Lazy::new(|| {
    let histogram_opts = histogram_opts!(
        "peer_monitoring_client_node_uptime",
        "Counters related to the node uptime (hours)",
        NODE_UPTIME_BUCKETS.to_vec()
    );
    register_histogram_vec!(histogram_opts, &["network_id"]).unwrap()
});

// Histogram buckets for tracking the number of connected peers
const NUM_CONNECTED_PEERS_BUCKETS: &[f64] = &[
    0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 20.0, 30.0, 50.0, 100.0, 200.0, 400.0,
    1000.0, // Max number of connected peers should never be more than 1000
];

/// Counter for tracking the number of connected peers
pub static NUM_CONNECTED_PEERS: Lazy<HistogramVec> = Lazy::new(|| {
    let histogram_opts = histogram_opts!(
        "peer_monitoring_client_num_connected_peers",
        "Counters related to the number of connected peers",
        NUM_CONNECTED_PEERS_BUCKETS.to_vec()
    );
    register_histogram_vec!(histogram_opts, &["network_id"]).unwrap()
});

/// Counter for tracking sent requests
pub static SENT_REQUESTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "peer_monitoring_client_sent_requests",
        "Counters related to sent requests",
        &["request_types", "network_id"]
    )
    .unwrap()
});

/// Counter for tracking success responses
pub static SUCCESS_RESPONSES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "peer_monitoring_client_success_responses",
        "Counters related to success responses",
        &["response_type", "network_id"]
    )
    .unwrap()
});

/// Counter for tracking error responses
pub static ERROR_RESPONSES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "peer_monitoring_client_error_responses",
        "Counters related to error responses",
        &["response_type", "network_id"]
    )
    .unwrap()
});

/// Counter for tracking request latencies
pub static REQUEST_LATENCIES: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "peer_monitoring_client_request_latencies",
        "Counters related to request latencies",
        &["request_type", "network_id"]
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
    let network_id = peer_network_id.network_id();
    counter
        .with_label_values(&[label, network_id.as_str()])
        .inc();
    counter
        .with_label_values(&[TOTAL_COUNT_LABEL, network_id.as_str()])
        .inc();
}

/// Sets the gauge with the specific label and value
pub fn set_gauge(counter: &Lazy<IntGaugeVec>, label: &str, value: u64) {
    counter.with_label_values(&[label]).set(value as i64);
}

/// Observes the value for the provided histogram
pub fn observe_value(histogram: &Lazy<HistogramVec>, peer_network_id: &PeerNetworkId, value: f64) {
    let network_id = peer_network_id.network_id();
    histogram
        .with_label_values(&[network_id.as_str()])
        .observe(value)
}

/// Observes the value for the provided histogram and label
pub fn observe_value_with_label(
    histogram: &Lazy<HistogramVec>,
    request_label: &str,
    peer_network_id: &PeerNetworkId,
    value: f64,
) {
    let network_id = peer_network_id.network_id();
    histogram
        .with_label_values(&[request_label, network_id.as_str()])
        .observe(value)
}

// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_config::network_id::PeerNetworkId;
use velor_crypto::_once_cell::sync::Lazy;
use velor_metrics_core::{
    histogram_opts, register_histogram_vec, register_int_counter_vec, register_int_gauge_vec,
    HistogramTimer, HistogramVec, IntCounterVec, IntGaugeVec,
};

// Useful metric constants and labels
pub const BLOCK_TIMESTAMP_LAG_LABEL: &str = "block_timestamp_lag";
pub const PRIORITIZED_PEER: &str = "prioritized_peer";
pub const PROPOSE_TO_SEEN_LATENCY_LABEL: &str = "propose_to_seen_latency";
pub const PROPOSE_TO_SYNC_LATENCY_LABEL: &str = "propose_to_sync_latency";
pub const REGULAR_PEER: &str = "regular_peer";
pub const SEEN_TO_SYNC_LATENCY_LABEL: &str = "seen_to_sync_latency";
pub const TOTAL_COUNT_LABEL: &str = "TOTAL_COUNT";

// TOOD(joshlind): add peer priorities back to the requests

/// Counter for tracking sent requests
pub static SENT_REQUESTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_data_client_sent_requests",
        "Counters related to sent requests",
        &["request_types", "network"]
    )
    .unwrap()
});

/// Counter for tracking success responses
pub static SUCCESS_RESPONSES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_data_client_success_responses",
        "Counters related to success responses",
        &["response_type", "network"]
    )
    .unwrap()
});

/// Counter for tracking error responses
pub static ERROR_RESPONSES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_data_client_error_responses",
        "Counters related to error responses",
        &["response_type", "network"]
    )
    .unwrap()
});

// Buckets for tracking the number of multi-fetches sent per request
const MULTI_FETCH_BUCKETS: &[f64] = &[
    1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 15.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0,
    80.0, 90.0, 100.0, 150.0, 200.0, 300.0, 400.0, 500.0,
];

/// Counter for tracking the number of multi-fetches sent per request
pub static MULTI_FETCHES_PER_REQUEST: Lazy<HistogramVec> = Lazy::new(|| {
    let histogram_opts = histogram_opts!(
        "velor_data_client_multi_fetches_per_request",
        "Counters related to the number of multi-fetches sent per request",
        MULTI_FETCH_BUCKETS.to_vec()
    );
    register_histogram_vec!(histogram_opts, &["label"]).unwrap()
});

// Latency buckets for network latencies (seconds)
const REQUEST_LATENCY_BUCKETS_SECS: &[f64] = &[
    0.05, 0.1, 0.2, 0.3, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 5.0, 7.5, 10.0, 15.0, 20.0, 30.0, 40.0,
    60.0, 120.0, 180.0, 240.0, 300.0,
];

/// Counter for tracking request latencies
pub static REQUEST_LATENCIES: Lazy<HistogramVec> = Lazy::new(|| {
    let histogram_opts = histogram_opts!(
        "velor_data_client_request_latencies",
        "Counters related to request latencies",
        REQUEST_LATENCY_BUCKETS_SECS.to_vec()
    );
    register_histogram_vec!(histogram_opts, &["request_type", "network"]).unwrap()
});

/// Gauge for tracking the number of in-flight polls
pub static IN_FLIGHT_POLLS: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "velor_data_client_in_flight_polls",
        "Gauge related to the number of in-flight polls",
        &["peer_type"]
    )
    .unwrap()
});

/// Gauge for tracking the number of connected peers (priority and regular)
pub static CONNECTED_PEERS: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "velor_data_client_connected_peers",
        "Gauge related to the number of connected peers",
        &["peer_type"]
    )
    .unwrap()
});

/// Gauge for tracking the number of connected peers by priority
pub static CONNECTED_PEERS_AND_PRIORITIES: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "velor_data_client_connected_peers_and_priorities",
        "Gauge related to the number of connected peers by priority",
        &["peer_type"]
    )
    .unwrap()
});

/// Gauge for the highest advertised data
pub static HIGHEST_ADVERTISED_DATA: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "velor_data_client_highest_advertised_data",
        "Gauge related to the highest advertised data",
        &["data_type"]
    )
    .unwrap()
});

/// Gauge for tracking the ignored peers (by network)
pub static IGNORED_PEERS: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "velor_data_client_ignored_peers",
        "Gauge related to the number of ignored peers",
        &["network"]
    )
    .unwrap()
});

/// Gauge for the lowest advertised data
pub static LOWEST_ADVERTISED_DATA: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "velor_data_client_lowest_advertised_data",
        "Gauge related to the lowest advertised data",
        &["data_type"]
    )
    .unwrap()
});

/// Gauge for the optimal chunk sizes
pub static OPTIMAL_CHUNK_SIZES: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "velor_data_client_optimal_chunk_sizes",
        "Gauge related to the optimal chunk sizes",
        &["data_type"]
    )
    .unwrap()
});

// Latency buckets for the sync latencies (seconds). Note: there are a
// lot of buckets here because we really care about sync latencies.
const SYNC_LATENCY_BUCKETS_SECS: &[f64] = &[
    0.05, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0, 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8,
    1.9, 2.0, 2.1, 2.2, 2.3, 2.4, 2.5, 3.0, 5.0, 10.0, 15.0, 20.0, 30.0, 40.0, 60.0, 120.0, 180.0,
    240.0, 300.0, 360.0, 420.0, 480.0, 540.0, 600.0, 1200.0, 1800.0, 3600.0, 7200.0, 14400.0,
];

/// Counter for tracking various sync latencies
pub static SYNC_LATENCIES: Lazy<HistogramVec> = Lazy::new(|| {
    let histogram_opts = histogram_opts!(
        "velor_data_client_sync_latencies",
        "Counters related to sync latencies",
        SYNC_LATENCY_BUCKETS_SECS.to_vec()
    );
    register_histogram_vec!(histogram_opts, &["label"]).unwrap()
});

/// Gauge for tracking the number of sent requests by peer buckets
pub static SENT_REQUESTS_BY_PEER_BUCKET: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "velor_data_client_sent_requests_by_peer_bucket",
        "Gauge related to the sent requests by peer buckets",
        &["peer_bucket_id", "request_label"]
    )
    .unwrap()
});

/// Gauge for tracking the number of received responses by peer buckets
pub static RECEIVED_RESPONSES_BY_PEER_BUCKET: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "velor_data_client_received_responses_by_peer_bucket",
        "Gauge related to the received responses by peer buckets",
        &["peer_bucket_id", "request_label"]
    )
    .unwrap()
});

/// An enum representing the various types of data that can be
/// fetched via the data client.
pub enum DataType {
    LedgerInfos,
    States,
    TransactionOutputs,
    Transactions,
}

impl DataType {
    pub fn as_str(&self) -> &'static str {
        match self {
            DataType::LedgerInfos => "ledger_infos",
            DataType::States => "states",
            DataType::TransactionOutputs => "transaction_outputs",
            DataType::Transactions => "transactions",
        }
    }

    pub fn get_all_types() -> Vec<DataType> {
        vec![
            DataType::LedgerInfos,
            DataType::States,
            DataType::TransactionOutputs,
            DataType::Transactions,
        ]
    }
}

/// Increments the given request counter with the provided values.
pub fn increment_request_counter(
    counter: &Lazy<IntCounterVec>,
    label: &str,
    peer_network_id: PeerNetworkId,
) {
    let network = peer_network_id.network_id();
    counter.with_label_values(&[label, network.as_str()]).inc();
    counter
        .with_label_values(&[TOTAL_COUNT_LABEL, network.as_str()])
        .inc();
}

/// Observes the value for the provided histogram and label
pub fn observe_value_with_label(histogram: &Lazy<HistogramVec>, label: &str, value: f64) {
    histogram.with_label_values(&[label]).observe(value)
}

/// Sets the gauge with the specific label and value
pub fn set_gauge(counter: &Lazy<IntGaugeVec>, label: &str, value: u64) {
    counter.with_label_values(&[label]).set(value as i64);
}

/// Sets the gauge with the specific label and value for the specified bucket
pub fn set_gauge_for_bucket(counter: &Lazy<IntGaugeVec>, bucket: &str, label: &str, value: u64) {
    counter
        .with_label_values(&[bucket, label])
        .set(value as i64);
}

/// Starts the timer for the provided histogram and label values.
pub fn start_request_timer(
    histogram: &Lazy<HistogramVec>,
    request_label: &str,
    peer_network_id: PeerNetworkId,
) -> HistogramTimer {
    let network = peer_network_id.network_id();
    histogram
        .with_label_values(&[request_label, network.as_str()])
        .start_timer()
}

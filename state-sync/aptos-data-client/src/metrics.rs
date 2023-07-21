// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_config::network_id::PeerNetworkId;
use aptos_crypto::_once_cell::sync::Lazy;
use aptos_metrics_core::{
    histogram_opts, register_histogram_vec, register_int_counter_vec, register_int_gauge_vec,
    HistogramTimer, HistogramVec, IntCounterVec, IntGaugeVec,
};

// Useful metric constants and labels
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
        "aptos_data_client_sent_requests",
        "Counters related to sent requests",
        &["request_types", "network"]
    )
    .unwrap()
});

/// Counter for tracking success responses
pub static SUCCESS_RESPONSES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_data_client_success_responses",
        "Counters related to success responses",
        &["response_type", "network"]
    )
    .unwrap()
});

/// Counter for tracking error responses
pub static ERROR_RESPONSES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_data_client_error_responses",
        "Counters related to error responses",
        &["response_type", "network"]
    )
    .unwrap()
});

// Latency buckets for network latencies (seconds)
const REQUEST_LATENCY_BUCKETS_SECS: [f64; 18] = [
    0.05, 0.1, 0.2, 0.3, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 5.0, 7.5, 10.0, 15.0, 20.0, 30.0, 40.0,
    60.0,
];

/// Counter for tracking request latencies
pub static REQUEST_LATENCIES: Lazy<HistogramVec> = Lazy::new(|| {
    let histogram_opts = histogram_opts!(
        "aptos_data_client_request_latencies",
        "Counters related to request latencies",
        REQUEST_LATENCY_BUCKETS_SECS.to_vec()
    );
    register_histogram_vec!(histogram_opts, &["request_type", "network"]).unwrap()
});

/// Gauge for tracking the number of in-flight polls
pub static IN_FLIGHT_POLLS: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "aptos_data_client_in_flight_polls",
        "Gauge related to the number of in-flight polls",
        &["peer_type"]
    )
    .unwrap()
});

/// Gauge for tracking the number of connected peers (priority and regular)
pub static CONNECTED_PEERS: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "aptos_data_client_connected_peers",
        "Gauge related to the number of connected peers",
        &["peer_type"]
    )
    .unwrap()
});

/// Gauge for the highest advertised data
pub static HIGHEST_ADVERTISED_DATA: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "aptos_data_client_highest_advertised_data",
        "Gauge related to the highest advertised data",
        &["data_type"]
    )
    .unwrap()
});

/// Gauge for the lowest advertised data
pub static LOWEST_ADVERTISED_DATA: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "aptos_data_client_lowest_advertised_data",
        "Gauge related to the lowest advertised data",
        &["data_type"]
    )
    .unwrap()
});

/// Gauge for the optimal chunk sizes
pub static OPTIMAL_CHUNK_SIZES: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "aptos_data_client_optimal_chunk_sizes",
        "Gauge related to the optimal chunk sizes",
        &["data_type"]
    )
    .unwrap()
});

// Latency buckets for the sync latencies (seconds). Note: there are a
// lot of buckets here because we really care about sync latencies.
const SYNC_LATENCY_BUCKETS_SECS: [f64; 36] = [
    0.05, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0, 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8,
    1.9, 2.0, 2.1, 2.2, 2.3, 2.4, 2.5, 3.0, 5.0, 10.0, 15.0, 20.0, 30.0, 40.0, 60.0, 120.0, 180.0,
];

/// Counter for tracking various sync latencies
pub static SYNC_LATENCIES: Lazy<HistogramVec> = Lazy::new(|| {
    let histogram_opts = histogram_opts!(
        "aptos_data_client_sync_latencies",
        "Counters related to sync latencies",
        SYNC_LATENCY_BUCKETS_SECS.to_vec()
    );
    register_histogram_vec!(histogram_opts, &["label"]).unwrap()
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

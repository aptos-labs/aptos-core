// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::_once_cell::sync::Lazy;
use aptos_metrics::{
    register_histogram_vec, register_int_counter_vec, register_int_gauge_vec, HistogramTimer,
    HistogramVec, IntCounterVec, IntGaugeVec,
};

/// The special label TOTAL_COUNT stores the sum of all values in the counter.
pub const TOTAL_COUNT_LABEL: &str = "TOTAL_COUNT";
pub const PRIORITIZED_PEER: &str = "prioritized_peer";
pub const REGULAR_PEER: &str = "regular_peer";

/// Counter for tracking sent requests
pub static SENT_REQUESTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_data_client_sent_requests",
        "Counters related to sent requests",
        &["request_types"]
    )
    .unwrap()
});

/// Counter for tracking success responses
pub static SUCCESS_RESPONSES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_data_client_success_responses",
        "Counters related to success responses",
        &["response_type"]
    )
    .unwrap()
});

/// Counter for tracking error responses
pub static ERROR_RESPONSES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_data_client_error_responses",
        "Counters related to error responses",
        &["response_type"]
    )
    .unwrap()
});

/// Counter for tracking request latencies
pub static REQUEST_LATENCIES: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_data_client_request_latencies",
        "Counters related to request latencies",
        &["request_type"]
    )
    .unwrap()
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

/// An enum representing the various types of data that can be
/// fetched via the data client.
pub enum DataType {
    AccountStates,
    LedgerInfos,
    TransactionOutputs,
    Transactions,
}

impl DataType {
    pub fn as_str(&self) -> &'static str {
        match self {
            DataType::AccountStates => "account_states",
            DataType::LedgerInfos => "ledger_infos",
            DataType::TransactionOutputs => "transaction_outputs",
            DataType::Transactions => "transactions",
        }
    }

    pub fn get_all_types() -> Vec<DataType> {
        vec![
            DataType::AccountStates,
            DataType::LedgerInfos,
            DataType::TransactionOutputs,
            DataType::Transactions,
        ]
    }
}

/// Increments the given counter with the provided label values.
pub fn increment_counter(counter: &Lazy<IntCounterVec>, label: String) {
    counter.with_label_values(&[&label]).inc();
    counter.with_label_values(&[TOTAL_COUNT_LABEL]).inc();
}

/// Sets the gauge with the specific label and value
pub fn set_gauge(counter: &Lazy<IntGaugeVec>, label: String, value: u64) {
    counter.with_label_values(&[&label]).set(value as i64);
}

/// Starts the timer for the provided histogram and label values.
pub fn start_timer(histogram: &Lazy<HistogramVec>, label: String) -> HistogramTimer {
    histogram.with_label_values(&[&label]).start_timer()
}

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_config::network_id::PeerNetworkId;
use aptos_metrics_core::{
    register_histogram_vec, register_int_counter_vec, HistogramVec, IntCounterVec,
};
use once_cell::sync::Lazy;

/// The special label TOTAL_COUNT stores the sum of all values in the counter
pub const TOTAL_COUNT_LABEL: &str = "TOTAL_COUNT";

/// Counter for tracking sent direct send messages by the network client
pub static DIRECT_SEND_SENT_MESSAGES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "consensus_observer_network_client_direct_send_sent_messages",
        "Counters related to sent direct send messages for the network client",
        &["message_type", "network_id"]
    )
    .unwrap()
});

/// Counter for tracking direct send message errors by the network client
pub static DIRECT_SEND_ERRORS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "consensus_observer_network_client_direct_send_errors",
        "Counters related to direct send message errors for the network client",
        &["error_type", "network_id"]
    )
    .unwrap()
});

/// Counter for pending network events to the consensus observer
pub static PENDING_CONSENSUS_OBSERVER_NETWORK_EVENTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "consensus_observer_pending_network_events",
        "Counters for pending network events for consensus observer",
        &["state"]
    )
    .unwrap()
});

/// Counter for tracking RPC error responses received by the network client
pub static RPC_ERROR_RESPONSES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "consensus_observer_network_client_rpc_error_responses",
        "Counters related to RPC error responses from the network client",
        &["response_type", "network_id"]
    )
    .unwrap()
});

/// Counter for tracking sent RPC requests by the network client
pub static RPC_SENT_REQUESTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "consensus_observer_network_client_rpc_sent_requests",
        "Counters related to sent RPC requests for the network client",
        &["request_type", "network_id"]
    )
    .unwrap()
});

/// Counter for tracking successful RPC responses received by the network client
pub static RPC_SUCCESS_RESPONSES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "consensus_observer_network_client_rpc_success_responses",
        "Counters related to RPC success responses received by the network client",
        &["response_type", "network_id"]
    )
    .unwrap()
});

/// Counter for tracking RPC request latencies sent by the network client
pub static RPC_REQUEST_LATENCIES: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "consensus_observer_network_client_rpc_request_latencies",
        "Counters related to RPC request latencies sent by the network client",
        &["request_type", "network_id"]
    )
    .unwrap()
});

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

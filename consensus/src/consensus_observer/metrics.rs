// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_config::network_id::{NetworkId, PeerNetworkId};
use aptos_metrics_core::{
    register_histogram_vec, register_int_counter_vec, register_int_gauge_vec, HistogramVec,
    IntCounterVec, IntGaugeVec,
};
use once_cell::sync::Lazy;

// Useful metric labels
pub const CREATED_SUBSCRIPTION_LABEL: &str = "created_subscription";

/// Counter for tracking created subscriptions for the consensus observer
pub static OBSERVER_CREATED_SUBSCRIPTIONS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "consensus_observer_created_subscriptions",
        "Counters for created subscriptions for consensus observer",
        &["creation_label", "network_id"]
    )
    .unwrap()
});

/// Counter for tracking the number of active subscriptions for the consensus observer
pub static OBSERVER_NUM_ACTIVE_SUBSCRIPTIONS: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "consensus_observer_num_active_subscriptions",
        "Gauge related to active subscriptions for the consensus observer",
        &["network_id"]
    )
    .unwrap()
});

/// Counter for tracking successful RPC responses received by the consensus observer
pub static OBSERVER_RECEIVED_MESSAGE_RESPONSES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "consensus_observer_received_message_responses",
        "Counters related to successful RPC responses received by the consensus observer",
        &["response_type", "network_id"]
    )
    .unwrap()
});

/// Counter for tracking received (direct send) messages by the consensus observer
pub static OBSERVER_RECEIVED_MESSAGES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "consensus_observer_received_messages",
        "Counters related to received (direct send) messages by the consensus observer",
        &["message_type", "network_id"]
    )
    .unwrap()
});

/// Counter for tracking RPC request latencies sent by the consensus observer
pub static OBSERVER_REQUEST_LATENCIES: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "consensus_observer_request_latencies",
        "Counters related to RPC request latencies sent by the consensus observer",
        &["request_type", "network_id"]
    )
    .unwrap()
});

/// Counter for tracking RPC error responses received by the consensus observer
pub static OBSERVER_SENT_MESSAGE_ERRORS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "consensus_observer_sent_message_errors",
        "Counters related to RPC error responses received by the consensus observer",
        &["response_type", "network_id"]
    )
    .unwrap()
});

/// Counter for tracking sent RPC requests by the consensus observer
pub static OBSERVER_SENT_REQUESTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "consensus_observer_sent_requests",
        "Counters related to sent RPC requests by the consensus observer",
        &["request_type", "network_id"]
    )
    .unwrap()
});

/// Counter for tracking terminated subscriptions for the consensus observer
pub static OBSERVER_TERMINATED_SUBSCRIPTIONS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "consensus_observer_terminated_subscriptions",
        "Counters for terminated subscriptions for consensus observer",
        &["termination_label", "network_id"]
    )
    .unwrap()
});

/// Counter for pending network events for consensus observer and publisher
pub static PENDING_CONSENSUS_OBSERVER_NETWORK_EVENTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "consensus_observer_pending_network_events",
        "Counters for pending network events for consensus observer and publisher",
        &["state"]
    )
    .unwrap()
});

/// Counter for tracking the number of active subscribers for the consensus publisher
pub static PUBLISHER_NUM_ACTIVE_SUBSCRIBERS: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "consensus_publisher_num_active_subscribers",
        "Gauge related to active subscribers for the consensus publisher",
        &["network_id"]
    )
    .unwrap()
});

/// Counter for tracking received RPC requests by the consensus publisher
pub static PUBLISHER_RECEIVED_REQUESTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "consensus_publisher_received_requests",
        "Counters related to received RPC requests by the consensus publisher",
        &["request_type", "network_id"]
    )
    .unwrap()
});

/// Counter for tracking sent (direct send) message errors for the consensus publisher
pub static PUBLISHER_SENT_MESSAGE_ERRORS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "consensus_publisher_sent_message_errors",
        "Counters related to sent (direct send) message errors for the consensus publisher",
        &["error_type", "network_id"]
    )
    .unwrap()
});

/// Counter for tracking sent (direct send) messages by the consensus publisher
pub static PUBLISHER_SENT_MESSAGES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "consensus_publisher_sent_messages",
        "Counters related to sent (direct send) messages by the consensus publisher",
        &["message_type", "network_id"]
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

/// Sets the gauge with the specific label and value
pub fn set_gauge(counter: &Lazy<IntGaugeVec>, network_id: &NetworkId, value: i64) {
    counter.with_label_values(&[network_id.as_str()]).set(value);
}

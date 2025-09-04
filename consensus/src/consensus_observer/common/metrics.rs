// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::unwrap_used)]

use velor_config::network_id::{NetworkId, PeerNetworkId};
use velor_metrics_core::{
    exponential_buckets, register_histogram_vec, register_int_counter, register_int_counter_vec,
    register_int_gauge_vec, HistogramVec, IntCounter, IntCounterVec, IntGaugeVec,
};
use once_cell::sync::Lazy;

// Useful observer metric labels
pub const BLOCK_PAYLOAD_LABEL: &str = "block_payload";
pub const COMMIT_DECISION_LABEL: &str = "commit_decision";
pub const COMMITTED_BLOCKS_LABEL: &str = "committed_blocks";
pub const CREATED_SUBSCRIPTION_LABEL: &str = "created_subscription";
pub const ORDERED_BLOCK_ENTRIES_LABEL: &str = "ordered_block_entries";
pub const ORDERED_BLOCK_LABEL: &str = "ordered_block";
pub const ORDERED_BLOCK_WITH_WINDOW_LABEL: &str = "ordered_block_with_window";
pub const PENDING_BLOCK_ENTRIES_BY_HASH_LABEL: &str = "pending_block_by_hash_entries";
pub const PENDING_BLOCK_ENTRIES_LABEL: &str = "pending_block_entries";
pub const PENDING_BLOCKS_BY_HASH_LABEL: &str = "pending_blocks_by_hash";
pub const PENDING_BLOCKS_LABEL: &str = "pending_blocks";
pub const STORED_PAYLOADS_LABEL: &str = "stored_payloads";

// Useful state sync metric labels
pub const STATE_SYNCING_FOR_FALLBACK: &str = "sync_for_fallback";
pub const STATE_SYNCING_TO_COMMIT: &str = "sync_to_commit";

/// Counter for tracking created subscriptions for the consensus observer
pub static OBSERVER_CREATED_SUBSCRIPTIONS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "consensus_observer_created_subscriptions",
        "Counters for created subscriptions for consensus observer",
        &["creation_label", "network_id"]
    )
    .unwrap()
});

/// Counter for tracking the number of times the block state was cleared by the consensus observer
pub static OBSERVER_CLEARED_BLOCK_STATE: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "consensus_observer_cleared_block_state",
        "Counter for tracking the number of times the block state was cleared by the consensus observer",
    ).unwrap()
});

/// Counter for tracking dropped (direct send) messages by the consensus observer
pub static OBSERVER_DROPPED_MESSAGES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "consensus_observer_dropped_messages",
        "Counters related to dropped (direct send) messages by the consensus observer",
        &["message_type", "network_id"]
    )
    .unwrap()
});

/// Counter for tracking invalid (direct send) messages by the consensus observer
pub static OBSERVER_INVALID_MESSAGES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "consensus_observer_invalid_messages",
        "Counters related to invalid (direct send) messages by the consensus observer",
        &["message_type", "network_id"]
    )
    .unwrap()
});

/// Counter for tracking rejected (direct send) messages by the consensus observer
pub static OBSERVER_REJECTED_MESSAGES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "consensus_observer_rejected_messages",
        "Counters related to rejected (direct send) messages by the consensus observer",
        &["message_type", "network_id"]
    )
    .unwrap()
});

/// Gauge for tracking the number of active subscriptions for the consensus observer
pub static OBSERVER_NUM_ACTIVE_SUBSCRIPTIONS: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "consensus_observer_num_active_subscriptions",
        "Gauge related to active subscriptions for the consensus observer",
        &["network_id"]
    )
    .unwrap()
});

/// Gauge for tracking the number of processed blocks by the consensus observer
pub static OBSERVER_NUM_PROCESSED_BLOCKS: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "consensus_observer_num_processed_blocks",
        "Gauge for tracking the number of processed blocks by the consensus observer",
        &["processed_type"]
    )
    .unwrap()
});

/// Gauge for tracking the processed block rounds by the consensus observer
pub static OBSERVER_PROCESSED_BLOCK_ROUNDS: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "consensus_observer_processed_block_rounds",
        "Gauge for tracking the processed block rounds by the consensus observer",
        &["processed_type"]
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

/// Gauge for tracking the rounds of received messages by the consensus observer
pub static OBSERVER_RECEIVED_MESSAGE_ROUNDS: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "consensus_observer_received_message_rounds",
        "Gauge for tracking the rounds of received messages by the consensus observer",
        &["message_type"]
    )
    .unwrap()
});

/// Counter for tracking observer message processing latencies
pub static OBSERVER_MESSAGE_PROCESSING_LATENCIES: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "consensus_observer_message_processing_latency",
        "Counters related to observer message processing latencies",
        &["message_type", "network_id"],
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 30).unwrap(),
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

/// Gauge for tracking when consensus observer has invoked state sync
pub static OBSERVER_STATE_SYNC_EXECUTING: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "consensus_observer_state_sync_executing",
        "Gauge for tracking when consensus observer has invoked state sync",
        &["syncing_mode"]
    )
    .unwrap()
});

/// Counter for tracking state sync fallback invocations by the consensus observer
pub static OBSERVER_STATE_SYNC_FALLBACK_COUNTER: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "consensus_observer_state_sync_fallback_counter",
        "Counter for tracking state sync fallback invocations by the consensus observer",
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

/// Gauge for tracking the number of active subscribers for the consensus publisher
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

/// Increments the given counter with the provided values
pub fn increment_counter(
    counter: &Lazy<IntCounterVec>,
    label: &str,
    peer_network_id: &PeerNetworkId,
) {
    let network_id = peer_network_id.network_id();
    counter
        .with_label_values(&[label, network_id.as_str()])
        .inc();
}

/// Increments the given counter without labels
pub fn increment_counter_without_labels(counter: &Lazy<IntCounter>) {
    counter.inc();
}

/// Observes the value for the provided histogram and label
pub fn observe_value_with_label(
    histogram: &Lazy<HistogramVec>,
    label: &str,
    peer_network_id: &PeerNetworkId,
    value: f64,
) {
    let network_id = peer_network_id.network_id();
    histogram
        .with_label_values(&[label, network_id.as_str()])
        .observe(value)
}

/// Sets the gauge with the specific network ID and value
pub fn set_gauge(counter: &Lazy<IntGaugeVec>, network_id: &NetworkId, value: i64) {
    counter.with_label_values(&[network_id.as_str()]).set(value);
}

/// Sets the gauge with the specific label and value
pub fn set_gauge_with_label(counter: &Lazy<IntGaugeVec>, label: &str, value: u64) {
    counter.with_label_values(&[label]).set(value as i64);
}

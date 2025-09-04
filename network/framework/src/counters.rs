// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::protocols::wire::handshake::v1::ProtocolId;
use velor_config::network_id::NetworkContext;
use velor_metrics_core::{
    exponential_buckets, register_histogram_vec, register_int_counter_vec, register_int_gauge,
    register_int_gauge_vec, Histogram, HistogramTimer, HistogramVec, IntCounter, IntCounterVec,
    IntGauge, IntGaugeVec,
};
use velor_netcore::transport::ConnectionOrigin;
use velor_short_hex_str::AsShortHexStr;
use velor_types::PeerId;
use once_cell::sync::Lazy;

// some type labels
pub const REQUEST_LABEL: &str = "request";
pub const RESPONSE_LABEL: &str = "response";

// some state labels
pub const CANCELED_LABEL: &str = "canceled";
pub const DECLINED_LABEL: &str = "declined";
pub const EXPIRED_LABEL: &str = "expired";
pub const RECEIVED_LABEL: &str = "received";
pub const SENT_LABEL: &str = "sent";
pub const SUCCEEDED_LABEL: &str = "succeeded";
pub const FAILED_LABEL: &str = "failed";
pub const UNKNOWN_LABEL: &str = "unknown";

// Connection operation labels
pub const DIAL_LABEL: &str = "dial";
pub const DIAL_PEER_LABEL: &str = "dial_peer";
pub const DISCONNECT_LABEL: &str = "disconnect";

// Direction labels
pub const INBOUND_LABEL: &str = "inbound";
pub const OUTBOUND_LABEL: &str = "outbound";

// Peer ping labels
const CONNECTED_LABEL: &str = "connected";
const PRE_DIAL_LABEL: &str = "pre_dial";

// Serialization labels
pub const SERIALIZATION_LABEL: &str = "serialization";
pub const DESERIALIZATION_LABEL: &str = "deserialization";

pub static VELOR_CONNECTIONS: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "velor_connections",
        "Number of current connections and their direction",
        &["role_type", "network_id", "peer_id", "direction"]
    )
    .unwrap()
});

pub fn connections(network_context: &NetworkContext, origin: ConnectionOrigin) -> IntGauge {
    VELOR_CONNECTIONS.with_label_values(&[
        network_context.role().as_str(),
        network_context.network_id().as_str(),
        network_context.peer_id().short_str().as_str(),
        origin.as_str(),
    ])
}

pub static VELOR_CONNECTIONS_REJECTED: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_connections_rejected",
        "Number of connections rejected per interface",
        &["role_type", "network_id", "peer_id", "direction"]
    )
    .unwrap()
});

pub fn connections_rejected(
    network_context: &NetworkContext,
    origin: ConnectionOrigin,
) -> IntCounter {
    VELOR_CONNECTIONS_REJECTED.with_label_values(&[
        network_context.role().as_str(),
        network_context.network_id().as_str(),
        network_context.peer_id().short_str().as_str(),
        origin.as_str(),
    ])
}

pub static VELOR_NETWORK_PEER_CONNECTED: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "velor_network_peer_connected",
        "Indicates if we are connected to a particular peer",
        &["role_type", "network_id", "peer_id", "remote_peer_id"]
    )
    .unwrap()
});

pub fn peer_connected(network_context: &NetworkContext, remote_peer_id: &PeerId, v: i64) {
    if network_context.network_id().is_validator_network() {
        VELOR_NETWORK_PEER_CONNECTED
            .with_label_values(&[
                network_context.role().as_str(),
                network_context.network_id().as_str(),
                network_context.peer_id().short_str().as_str(),
                remote_peer_id.short_str().as_str(),
            ])
            .set(v)
    }
}

/// Increments the counter based on `NetworkContext`
pub fn inc_by_with_context(
    counter: &IntCounterVec,
    network_context: &NetworkContext,
    label: &str,
    val: u64,
) {
    counter
        .with_label_values(&[
            network_context.role().as_str(),
            network_context.network_id().as_str(),
            network_context.peer_id().short_str().as_str(),
            label,
        ])
        .inc_by(val)
}

pub static VELOR_NETWORK_PENDING_CONNECTION_UPGRADES: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "velor_network_pending_connection_upgrades",
        "Number of concurrent inbound or outbound connections we're currently negotiating",
        &["role_type", "network_id", "peer_id", "direction"]
    )
    .unwrap()
});

pub fn pending_connection_upgrades(
    network_context: &NetworkContext,
    direction: ConnectionOrigin,
) -> IntGauge {
    VELOR_NETWORK_PENDING_CONNECTION_UPGRADES.with_label_values(&[
        network_context.role().as_str(),
        network_context.network_id().as_str(),
        network_context.peer_id().short_str().as_str(),
        direction.as_str(),
    ])
}

/// A simple counter for tracking network connection operations
pub static VELOR_NETWORK_CONNECTION_OPERATIONS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_network_connection_operations",
        "Counter for tracking connection operations",
        &["network_id", "operation", "label"]
    )
    .unwrap()
});

/// Updates the network connection operation metrics with the given operation and label
pub fn update_network_connection_operation_metrics(
    network_context: &NetworkContext,
    operation: String,
    label: String,
) {
    VELOR_NETWORK_CONNECTION_OPERATIONS
        .with_label_values(&[network_context.network_id().as_str(), &operation, &label])
        .inc();
}

pub static VELOR_NETWORK_CONNECTION_UPGRADE_TIME: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "velor_network_connection_upgrade_time_seconds",
        "Time to complete a new inbound or outbound connection upgrade",
        &["role_type", "network_id", "peer_id", "direction", "state"]
    )
    .unwrap()
});

pub fn connection_upgrade_time(
    network_context: &NetworkContext,
    direction: ConnectionOrigin,
    state: &'static str,
) -> Histogram {
    VELOR_NETWORK_CONNECTION_UPGRADE_TIME.with_label_values(&[
        network_context.role().as_str(),
        network_context.network_id().as_str(),
        network_context.peer_id().short_str().as_str(),
        direction.as_str(),
        state,
    ])
}

pub static VELOR_NETWORK_DISCOVERY_NOTES: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "velor_network_discovery_notes",
        "Velor network discovery notes",
        &["role_type"]
    )
    .unwrap()
});

pub static VELOR_NETWORK_RPC_MESSAGES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!("velor_network_rpc_messages", "Number of RPC messages", &[
        "role_type",
        "network_id",
        "peer_id",
        "message_type",
        "message_direction",
        "state"
    ])
    .unwrap()
});

pub fn rpc_messages(
    network_context: &NetworkContext,
    message_type_label: &'static str,
    message_direction_label: &'static str,
    state_label: &'static str,
) -> IntCounter {
    VELOR_NETWORK_RPC_MESSAGES.with_label_values(&[
        network_context.role().as_str(),
        network_context.network_id().as_str(),
        network_context.peer_id().short_str().as_str(),
        message_type_label,
        message_direction_label,
        state_label,
    ])
}

pub static VELOR_NETWORK_RPC_BYTES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_network_rpc_bytes",
        "Number of RPC bytes transferred",
        &[
            "role_type",
            "network_id",
            "peer_id",
            "message_type",
            "message_direction",
            "state"
        ]
    )
    .unwrap()
});

pub fn rpc_bytes(
    network_context: &NetworkContext,
    message_type_label: &'static str,
    message_direction_label: &'static str,
    state_label: &'static str,
) -> IntCounter {
    VELOR_NETWORK_RPC_BYTES.with_label_values(&[
        network_context.role().as_str(),
        network_context.network_id().as_str(),
        network_context.peer_id().short_str().as_str(),
        message_type_label,
        message_direction_label,
        state_label,
    ])
}

pub static INVALID_NETWORK_MESSAGES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_network_invalid_messages",
        "Number of invalid messages (RPC/direct_send)",
        &["role_type", "network_id", "peer_id", "type"]
    )
    .unwrap()
});

pub static PEER_SEND_FAILURES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_network_peer_send_failures",
        "Number of messages failed to send to peer",
        &["role_type", "network_id", "peer_id", "protocol_id"]
    )
    .unwrap()
});

pub static VELOR_NETWORK_OUTBOUND_RPC_REQUEST_LATENCY: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "velor_network_outbound_rpc_request_latency_seconds",
        "Outbound RPC request latency in seconds",
        &["role_type", "network_id", "peer_id", "protocol_id"]
    )
    .unwrap()
});

pub fn outbound_rpc_request_latency(
    network_context: &NetworkContext,
    protocol_id: ProtocolId,
) -> Histogram {
    VELOR_NETWORK_OUTBOUND_RPC_REQUEST_LATENCY.with_label_values(&[
        network_context.role().as_str(),
        network_context.network_id().as_str(),
        network_context.peer_id().short_str().as_str(),
        protocol_id.as_str(),
    ])
}

pub static VELOR_NETWORK_INBOUND_RPC_HANDLER_LATENCY: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "velor_network_inbound_rpc_handler_latency_seconds",
        "Inbound RPC request application handler latency in seconds",
        &["role_type", "network_id", "peer_id", "protocol_id"]
    )
    .unwrap()
});

pub fn inbound_rpc_handler_latency(
    network_context: &NetworkContext,
    protocol_id: ProtocolId,
) -> Histogram {
    VELOR_NETWORK_INBOUND_RPC_HANDLER_LATENCY.with_label_values(&[
        network_context.role().as_str(),
        network_context.network_id().as_str(),
        network_context.peer_id().short_str().as_str(),
        protocol_id.as_str(),
    ])
}

pub static VELOR_NETWORK_DIRECT_SEND_MESSAGES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_network_direct_send_messages",
        "Number of direct send messages",
        &["role_type", "network_id", "peer_id", "state"]
    )
    .unwrap()
});

pub fn direct_send_messages(
    network_context: &NetworkContext,
    state_label: &'static str,
) -> IntCounter {
    VELOR_NETWORK_DIRECT_SEND_MESSAGES.with_label_values(&[
        network_context.role().as_str(),
        network_context.network_id().as_str(),
        network_context.peer_id().short_str().as_str(),
        state_label,
    ])
}

pub static VELOR_NETWORK_DIRECT_SEND_BYTES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_network_direct_send_bytes",
        "Number of direct send bytes transferred",
        &["role_type", "network_id", "peer_id", "state"]
    )
    .unwrap()
});

pub fn direct_send_bytes(
    network_context: &NetworkContext,
    state_label: &'static str,
) -> IntCounter {
    VELOR_NETWORK_DIRECT_SEND_BYTES.with_label_values(&[
        network_context.role().as_str(),
        network_context.network_id().as_str(),
        network_context.peer_id().short_str().as_str(),
        state_label,
    ])
}

/// Counters(queued,dequeued,dropped) related to inbound network notifications for RPCs and
/// DirectSends.
pub static PENDING_NETWORK_NOTIFICATIONS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_network_pending_network_notifications",
        "Number of pending inbound network notifications by state",
        &["state"]
    )
    .unwrap()
});

/// Counter of pending requests in Network Provider
pub static PENDING_NETWORK_REQUESTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_network_pending_requests",
        "Number of pending outbound network requests by state",
        &["state"]
    )
    .unwrap()
});

/// Counter of pending network events to Health Checker.
pub static PENDING_HEALTH_CHECKER_NETWORK_EVENTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_network_pending_health_check_events",
        "Number of pending health check events by state",
        &["state"]
    )
    .unwrap()
});

/// Counter of pending network events to Discovery.
pub static PENDING_DISCOVERY_NETWORK_EVENTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_network_pending_discovery_events",
        "Number of pending discovery events by state",
        &["state"]
    )
    .unwrap()
});

/// Counter of pending requests in Peer Manager
pub static PENDING_PEER_MANAGER_REQUESTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_network_pending_peer_manager_requests",
        "Number of pending peer manager requests by state",
        &["state"]
    )
    .unwrap()
});

//
// Channel Counters
//

/// Counter of pending requests in Connectivity Manager
pub static PENDING_CONNECTIVITY_MANAGER_REQUESTS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "velor_network_pending_connectivity_manager_requests",
        "Number of pending connectivity manager requests"
    )
    .unwrap()
});

/// Counter of pending Connection Handler notifications to PeerManager.
pub static PENDING_CONNECTION_HANDLER_NOTIFICATIONS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "velor_network_pending_connection_handler_notifications",
        "Number of pending connection handler notifications"
    )
    .unwrap()
});

/// Counter of pending dial requests in Peer Manager
pub static PENDING_PEER_MANAGER_DIAL_REQUESTS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "velor_network_pending_peer_manager_dial_requests",
        "Number of pending peer manager dial requests"
    )
    .unwrap()
});

/// Counter of messages pending in queue to be sent out on the wire.
pub static PENDING_WIRE_MESSAGES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_network_pending_wire_messages",
        "Number of pending wire messages",
        &["state"],
    )
    .unwrap()
});

/// Counter of messages pending in queue to be sent out on the multiplex channel
pub static PENDING_MULTIPLEX_MESSAGE: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "velor_network_pending_multiplex_messages",
        "Number of pending multiplex messages"
    )
    .unwrap()
});

/// Counter of stream messages pending in queue to be sent out on the multiplex channel
pub static PENDING_MULTIPLEX_STREAM: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "velor_network_pending_multiplex_stream",
        "Number of pending multiplex stream messages"
    )
    .unwrap()
});

/// Counter of pending requests in Direct Send
pub static PENDING_DIRECT_SEND_REQUESTS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "velor_network_pending_direct_send_requests",
        "Number of pending direct send requests"
    )
    .unwrap()
});

/// Counter of pending Direct Send notifications to Network Provider
pub static PENDING_DIRECT_SEND_NOTIFICATIONS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "velor_network_pending_direct_send_notifications",
        "Number of pending direct send notifications"
    )
    .unwrap()
});

/// Counter of pending requests in RPC
pub static PENDING_RPC_REQUESTS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "velor_network_pending_rpc_requests",
        "Number of pending rpc requests"
    )
    .unwrap()
});

/// Counter of pending RPC notifications to Network Provider
pub static PENDING_RPC_NOTIFICATIONS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "velor_network_pending_rpc_notifications",
        "Number of pending rpc notifications"
    )
    .unwrap()
});

/// Counter of pending requests for each remote peer
pub static PENDING_PEER_REQUESTS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "velor_network_pending_peer_requests",
        "Number of pending peer requests"
    )
    .unwrap()
});

/// Counter of pending RPC events from Peer to Rpc actor.
pub static PENDING_PEER_RPC_NOTIFICATIONS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "velor_network_pending_peer_rpc_notifications",
        "Number of pending peer rpc notifications"
    )
    .unwrap()
});

/// Counter of pending DirectSend events from Peer to DirectSend actor..
pub static PENDING_PEER_DIRECT_SEND_NOTIFICATIONS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "velor_network_pending_peer_direct_send_notifications",
        "Number of pending peer direct send notifications"
    )
    .unwrap()
});

/// Counter of pending connection notifications from Peer to NetworkProvider.
pub static PENDING_PEER_NETWORK_NOTIFICATIONS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "velor_network_pending_peer_network_notifications",
        "Number of pending peer network notifications"
    )
    .unwrap()
});

pub static NETWORK_RATE_LIMIT_METRICS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "velor_network_rate_limit",
        "Network Rate Limiting Metrics",
        &["direction", "metric"]
    )
    .unwrap()
});

pub static NETWORK_APPLICATION_INBOUND_METRIC: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "velor_network_app_inbound_traffic",
        "Network Inbound Traffic by application",
        &[
            "role_type",
            "network_id",
            "peer_id",
            "protocol_id",
            "metric"
        ]
    )
    .unwrap()
});

pub fn network_application_inbound_traffic(
    network_context: NetworkContext,
    protocol_id: ProtocolId,
    size: u64,
) {
    NETWORK_APPLICATION_INBOUND_METRIC
        .with_label_values(&[
            network_context.role().as_str(),
            network_context.network_id().as_str(),
            network_context.peer_id().short_str().as_str(),
            protocol_id.as_str(),
            "size",
        ])
        .observe(size as f64);
}

pub static NETWORK_APPLICATION_OUTBOUND_METRIC: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "velor_network_app_outbound_traffic",
        "Network Outbound Traffic by application",
        &[
            "role_type",
            "network_id",
            "peer_id",
            "protocol_id",
            "metric"
        ]
    )
    .unwrap()
});

pub fn network_application_outbound_traffic(
    network_context: NetworkContext,
    protocol_id: ProtocolId,
    size: u64,
) {
    NETWORK_APPLICATION_OUTBOUND_METRIC
        .with_label_values(&[
            network_context.role().as_str(),
            network_context.network_id().as_str(),
            network_context.peer_id().short_str().as_str(),
            protocol_id.as_str(),
            "size",
        ])
        .observe(size as f64);
}

/// Time it takes to perform message serialization and deserialization
pub static NETWORK_APPLICATION_SERIALIZATION_METRIC: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "velor_network_serialization_metric",
        "Time it takes to perform message serialization and deserialization",
        &["protocol_id", "operation"],
        exponential_buckets(/*start=*/ 1e-6, /*factor=*/ 2.0, /*count=*/ 30).unwrap(),
    )
    .unwrap()
});

/// Starts and returns the timer for serialization/deserialization
pub fn start_serialization_timer(protocol_id: ProtocolId, operation: &str) -> HistogramTimer {
    NETWORK_APPLICATION_SERIALIZATION_METRIC
        .with_label_values(&[protocol_id.as_str(), operation])
        .start_timer()
}

/// Counters related to peer ping times (before and after dialing)
pub static NETWORK_PEER_PING_TIMES: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "velor_network_peer_ping_times",
        "Counters related to peer ping times (before and after dialing)",
        &["network_id", "label"],
    )
    .unwrap()
});

/// Observes the ping time for a connected peer
pub fn observe_connected_ping_time(network_context: &NetworkContext, ping_latency_secs: f64) {
    observe_ping_time(network_context, ping_latency_secs, CONNECTED_LABEL);
}

/// Observes the ping time for a peer before dialing
pub fn observe_pre_dial_ping_time(network_context: &NetworkContext, ping_latency_secs: f64) {
    observe_ping_time(network_context, ping_latency_secs, PRE_DIAL_LABEL);
}

/// Observes the ping time for the given label
fn observe_ping_time(network_context: &NetworkContext, ping_latency_secs: f64, label: &str) {
    NETWORK_PEER_PING_TIMES
        .with_label_values(&[network_context.network_id().as_str(), label])
        .observe(ping_latency_secs);
}

pub static OP_MEASURE: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "velor_network_measure",
        "Measures the time and count of an operation",
        &["op"]
    )
    .unwrap()
});

pub static INBOUND_QUEUE_DELAY: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "velor_network_inbound_queue_time",
        "Time a message sits in queue between peer socket and app code",
        &["protocol_id"],
        exponential_buckets(/*start=*/ 1e-6, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
    )
    .unwrap()
});

pub fn inbound_queue_delay_observe(protocol_id: ProtocolId, seconds: f64) {
    INBOUND_QUEUE_DELAY
        .with_label_values(&[protocol_id.as_str()])
        .observe(seconds)
}

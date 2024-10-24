// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::ProtocolId;
use aptos_metrics_core::{register_histogram_vec, HistogramVec};
use once_cell::sync::Lazy;

// Useful message tracking labels
pub const APPLICATION_SEND_TO_NETWORK_SEND: &str = "application_send_to_network_send";
pub const NETWORK_SEND_TO_NETWORK_RECEIVE: &str = "network_send_to_network_receive";
pub const NETWORK_RECEIVE_TO_APPLICATION_RECEIVE: &str = "network_receive_to_application_receive";

/// Counter for tracking message processing latencies in the network stack
pub static MESSAGE_LATENCY_TRACKER: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "network_stack_message_processing_latency",
        "Counters related to message processing latencies in the network stack",
        &["protocol_id", "message_streamed", "message_phase"]
    )
    .unwrap()
});

/// Observes a message processing latency in the network stack
pub fn observe_message_latency(
    histogram: &Lazy<HistogramVec>,
    protocol_id: &ProtocolId,
    message_streamed: bool,
    message_phase: &str,
    value: f64,
) {
    histogram
        .with_label_values(&[
            &protocol_id.to_string(),
            &message_streamed.to_string(),
            message_phase,
        ])
        .observe(value)
}

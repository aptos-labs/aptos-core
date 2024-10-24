// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::protocols::wire::{
    handshake::v1::ProtocolId,
    messaging::v1::{metrics, IncomingRequest, Priority, RequestId, RpcResponse},
};
use proptest_derive::Arbitrary;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct DirectSendWithMetadata {
    #[serde(with = "serde_bytes")]
    pub serialized_message: Vec<u8>, // The serialized (raw) direct send message
    pub message_metadata: MessageMetadata, // Message metadata for the direct send
    pub latency_metadata: LatencyMetadata, // Latency metadata for the direct send
}

impl DirectSendWithMetadata {
    pub fn new(
        serialized_message: Vec<u8>,
        message_metadata: MessageMetadata,
        latency_metadata: LatencyMetadata,
    ) -> Self {
        Self {
            serialized_message,
            message_metadata,
            latency_metadata,
        }
    }
}

impl IncomingRequest for DirectSendWithMetadata {
    fn protocol_id(&self) -> crate::ProtocolId {
        self.message_metadata.protocol_id
    }

    fn data(&self) -> &Vec<u8> {
        &self.serialized_message
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct RpcRequestWithMetadata {
    pub request_id: RequestId, // The unique ID for the RPC request
    #[serde(with = "serde_bytes")]
    pub serialized_request: Vec<u8>, // The serialized (raw) RPC request
    pub message_metadata: MessageMetadata, // Message metadata for the RPC request
    pub latency_metadata: LatencyMetadata, // Latency metadata for the RPC request
}

impl RpcRequestWithMetadata {
    pub fn new(
        request_id: RequestId,
        serialized_request: Vec<u8>,
        message_metadata: MessageMetadata,
        latency_metadata: LatencyMetadata,
    ) -> Self {
        Self {
            request_id,
            serialized_request,
            message_metadata,
            latency_metadata,
        }
    }
}

impl IncomingRequest for RpcRequestWithMetadata {
    fn protocol_id(&self) -> crate::ProtocolId {
        self.message_metadata.protocol_id
    }

    fn data(&self) -> &Vec<u8> {
        &self.serialized_request
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct RpcResponseWithMetadata {
    pub request_id: RequestId, // The unique ID of the RPC request that this is a response to
    #[serde(with = "serde_bytes")]
    pub serialized_response: Vec<u8>, // The serialized (raw) RPC response
    pub message_metadata: MessageMetadata, // Message metadata for the RPC response
    pub latency_metadata: LatencyMetadata, // Latency metadata for the RPC response
}

impl RpcResponseWithMetadata {
    pub fn new(
        request_id: RequestId,
        serialized_response: Vec<u8>,
        message_metadata: MessageMetadata,
        latency_metadata: LatencyMetadata,
    ) -> Self {
        Self {
            request_id,
            serialized_response,
            message_metadata,
            latency_metadata,
        }
    }

    /// Creates and returns a new `RpcResponseWithMetadata` from a legacy `RpcResponse`.
    /// Note: this will be removed when all nodes are updated to the new message formats.
    pub fn new_from_legacy_response(response: RpcResponse) -> Self {
        Self {
            request_id: response.request_id,
            serialized_response: response.raw_response,
            message_metadata: MessageMetadata::new(ProtocolId::Unknown, response.priority),
            latency_metadata: LatencyMetadata::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct MessageMetadata {
    protocol_id: ProtocolId, // The application-level protocol ID for the message
    priority: Priority,      // The priority of the message
}

impl MessageMetadata {
    pub fn new(protocol_id: ProtocolId, priority: Priority) -> Self {
        Self {
            protocol_id,
            priority,
        }
    }

    /// Returns the protocol ID of the message
    pub fn protocol_id(&self) -> ProtocolId {
        self.protocol_id
    }

    /// Returns the priority of the message
    pub fn priority(&self) -> Priority {
        self.priority
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct LatencyMetadata {
    message_streamed: bool, // Whether the message was streamed in chunks or sent as a whole

    // Timestamps of relevant message events
    application_send_time: Option<SystemTime>, // The time the application originally sent the message
    network_send_time: Option<SystemTime>,     // The time the message was sent over the wire
    network_receive_time: Option<SystemTime>,  // The time the message was received over the wire
    application_receive_time: Option<SystemTime>, // The time the application received the message
}

impl LatencyMetadata {
    pub fn new() -> Self {
        Self {
            message_streamed: false,
            application_send_time: None,
            network_send_time: None,
            network_receive_time: None,
            application_receive_time: None,
        }
    }

    /// Sets the message streamed flag to true
    pub fn set_message_streamed(&mut self) {
        self.message_streamed = true;
    }

    /// Sets the application send time to now
    pub fn set_application_send_time(&mut self) {
        self.application_send_time = Some(SystemTime::now());
    }

    /// Sets the network send time to now
    pub fn set_network_send_time(&mut self) {
        self.network_send_time = Some(SystemTime::now());
    }

    /// Sets the network receive time to now
    pub fn set_network_receive_time(&mut self) {
        self.network_receive_time = Some(SystemTime::now());
    }

    /// Sets the application receive time to now
    pub fn set_application_receive_time(&mut self) {
        self.application_receive_time = Some(SystemTime::now());
    }

    /// Emits all relevant metrics for message latency tracking
    pub fn emit_latency_metrics(&self, protocol_id: &ProtocolId) {
        // Observe the application send to network send time
        if let (Some(application_send_time), Some(network_send_time)) =
            (self.application_send_time, self.network_send_time)
        {
            if let Ok(duration) = network_send_time.duration_since(application_send_time) {
                metrics::observe_message_latency(
                    &metrics::MESSAGE_LATENCY_TRACKER,
                    protocol_id,
                    self.message_streamed,
                    metrics::APPLICATION_SEND_TO_NETWORK_SEND,
                    duration.as_secs_f64(),
                );
            }
        }

        // Observe the network send to network receive time
        if let (Some(network_send_time), Some(network_receive_time)) =
            (self.network_send_time, self.network_receive_time)
        {
            if let Ok(duration) = network_receive_time.duration_since(network_send_time) {
                metrics::observe_message_latency(
                    &metrics::MESSAGE_LATENCY_TRACKER,
                    protocol_id,
                    self.message_streamed,
                    metrics::NETWORK_SEND_TO_NETWORK_RECEIVE,
                    duration.as_secs_f64(),
                );
            }
        }

        // Observe the network receive to application receive time
        if let (Some(network_receive_time), Some(application_receive_time)) =
            (self.network_receive_time, self.application_receive_time)
        {
            if let Ok(duration) = application_receive_time.duration_since(network_receive_time) {
                metrics::observe_message_latency(
                    &metrics::MESSAGE_LATENCY_TRACKER,
                    protocol_id,
                    self.message_streamed,
                    metrics::NETWORK_RECEIVE_TO_APPLICATION_RECEIVE,
                    duration.as_secs_f64(),
                );
            }
        }
    }
}

impl Default for LatencyMetadata {
    fn default() -> Self {
        Self::new()
    }
}

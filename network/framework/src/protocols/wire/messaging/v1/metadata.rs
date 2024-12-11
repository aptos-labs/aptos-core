// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    counters,
    protocols::wire::{
        handshake::v1::ProtocolId,
        messaging::v1::{MultiplexMessage, NetworkMessage, RpcResponse},
    },
};
use aptos_config::network_id::NetworkId;
use std::time::SystemTime;

/// A simple struct that wraps a network message with metadata.
/// Note: this is not sent along the wire, it is only used internally.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetworkMessageWithMetadata {
    /// The metadata about the message
    message_metadata: MessageMetadata,

    /// The network message to send along the wire
    network_message: NetworkMessage,
}

impl NetworkMessageWithMetadata {
    pub fn new(message_metadata: MessageMetadata, network_message: NetworkMessage) -> Self {
        Self {
            message_metadata,
            network_message,
        }
    }

    /// Converts the message into a multiplex message with metadata
    pub fn into_multiplex_message(self) -> MultiplexMessageWithMetadata {
        MultiplexMessageWithMetadata::new(
            self.message_metadata,
            MultiplexMessage::Message(self.network_message),
        )
    }

    /// Consumes the message and returns the individual parts
    pub fn into_parts(self) -> (MessageMetadata, NetworkMessage) {
        (self.message_metadata, self.network_message)
    }

    /// Returns a reference to the message metadata
    pub fn network_message(&self) -> &NetworkMessage {
        &self.network_message
    }
}

/// A simple struct that wraps a multiplex message with metadata.
/// Note: this is not sent along the wire, it is only used internally.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MultiplexMessageWithMetadata {
    /// The metadata about the message
    message_metadata: MessageMetadata,

    /// The multiplex message to send along the wire
    multiplex_message: MultiplexMessage,
}

impl MultiplexMessageWithMetadata {
    pub fn new(message_metadata: MessageMetadata, multiplex_message: MultiplexMessage) -> Self {
        Self {
            message_metadata,
            multiplex_message,
        }
    }

    /// Consumes the message and returns the individual parts
    pub fn into_parts(self) -> (MessageMetadata, MultiplexMessage) {
        (self.message_metadata, self.multiplex_message)
    }
}

/// A simple struct that wraps an RPC response with metadata.
/// Note: this is not sent along the wire, it is only used internally.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RpcResponseWithMetadata {
    /// The metadata about the message
    message_metadata: MessageMetadata,

    /// The response to send along the wire
    response: RpcResponse,
}

impl RpcResponseWithMetadata {
    pub fn new(message_metadata: MessageMetadata, response: RpcResponse) -> Self {
        Self {
            message_metadata,
            response,
        }
    }

    /// Transforms the message into an RPC response network message with metadata
    pub fn into_network_message(self) -> NetworkMessageWithMetadata {
        // Create the RPC response network message
        let network_message = NetworkMessage::RpcResponse(self.response);

        // Create and return the network message with metadata
        NetworkMessageWithMetadata::new(self.message_metadata, network_message)
    }

    /// Returns a reference to the RPC response
    pub fn rpc_response(&self) -> &RpcResponse {
        &self.response
    }
}

/// A simple enum to track the different types of message latencies
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MessageLatencyType {
    ApplicationToWire,
    WireSend,
    ApplicationSendToReceive,
    WireSendToReceive,
}

impl MessageLatencyType {
    pub fn get_label(&self) -> &'static str {
        match self {
            MessageLatencyType::ApplicationToWire => "ApplicationToWire",
            MessageLatencyType::WireSend => "WireSend",
            MessageLatencyType::WireSendToReceive => "WireSendToReceive",
            MessageLatencyType::ApplicationSendToReceive => "ApplicationSendToReceive",
        }
    }
}

/// A simple enum to track the message send type
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MessageSendType {
    DirectSend,  // A direct send message to another peer
    RpcRequest,  // An RPC request to another peer
    RpcResponse, // An RPC response for a request sent by another peer
}

impl MessageSendType {
    pub fn get_label(&self) -> &'static str {
        match self {
            MessageSendType::DirectSend => "DirectSend",
            MessageSendType::RpcRequest => "RpcRequest",
            MessageSendType::RpcResponse => "RpcResponse",
        }
    }
}

/// A simple enum to track the message receive type
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MessageReceiveType {
    DirectSend,  // A direct send message from another peer
    RpcRequest,  // An RPC request from another peer
    RpcResponse, // An RPC response to a request sent by this peer
}

impl MessageReceiveType {
    pub fn get_label(&self) -> &'static str {
        match self {
            MessageReceiveType::DirectSend => "DirectSend",
            MessageReceiveType::RpcRequest => "RpcRequest",
            MessageReceiveType::RpcResponse => "RpcResponse",
        }
    }
}

/// A simple enum to track the message stream type
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MessageStreamType {
    NonStreamedMessage,      // A non-streamed message that fits into a single chunk
    StreamedMessageHead,     // The head (first fragment) of a streamed message
    StreamedMessageFragment, // A fragment of a streamed message (not the head or tail)
    StreamedMessageTail,     // The tail (last fragment) of a streamed message
}

impl MessageStreamType {
    pub fn get_label(&self) -> &'static str {
        match self {
            MessageStreamType::NonStreamedMessage => "NonStreamedMessage",
            MessageStreamType::StreamedMessageHead => "StreamedMessageHead",
            MessageStreamType::StreamedMessageFragment => "StreamedMessageFragment",
            MessageStreamType::StreamedMessageTail => "StreamedMessageTail",
        }
    }
}

/// An enum representing metadata about each message type.
/// Note: this is not sent along the wire, it is only used internally.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MessageMetadata {
    SentMessageMetadata(SentMessageMetadata),
    ReceivedMessageMetadata(ReceivedMessageMetadata),
}

impl MessageMetadata {
    /// Creates and returns new metadata for a sent message
    pub fn new_sent_metadata(metadata: SentMessageMetadata) -> Self {
        MessageMetadata::SentMessageMetadata(metadata)
    }

    /// Creates and returns new metadata for a received message
    pub fn new_received_metadata(metadata: ReceivedMessageMetadata) -> Self {
        MessageMetadata::ReceivedMessageMetadata(metadata)
    }

    /// Transforms the metadata into a sent message metadata.
    /// Note: if the metadata is for another message type, this will return None.
    pub fn into_sent_metadata(self) -> Option<SentMessageMetadata> {
        match self {
            MessageMetadata::SentMessageMetadata(metadata) => Some(metadata),
            _ => None,
        }
    }

    /// Transforms the metadata into a received message metadata.
    /// Note: if the metadata is for another message type, this will return None.
    pub fn into_received_metadata(self) -> Option<ReceivedMessageMetadata> {
        match self {
            MessageMetadata::ReceivedMessageMetadata(metadata) => Some(metadata),
            _ => None,
        }
    }
}

/// A struct holding metadata about a sent message.
/// Note: this is not sent along the wire, it is only used internally.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SentMessageMetadata {
    /// The network ID for the message
    network_id: NetworkId,

    /// The protocol ID for the message. This may not always be
    /// known (e.g., when failing to deserialize a message).
    protocol_id: Option<ProtocolId>,

    /// The type of message being sent
    message_send_type: MessageSendType,

    /// The stream type of the message being sent
    message_stream_type: MessageStreamType,

    /// The time at which the message was sent by the application
    application_send_time: Option<SystemTime>,

    /// The time at which the message started being sent over the wire
    wire_send_start_time: Option<SystemTime>,
}

impl SentMessageMetadata {
    pub fn new(
        network_id: NetworkId,
        protocol_id: Option<ProtocolId>,
        message_send_type: MessageSendType,
        application_send_time: Option<SystemTime>,
    ) -> Self {
        Self {
            network_id,
            protocol_id,
            message_send_type,
            message_stream_type: MessageStreamType::NonStreamedMessage, // Default to non-streamed messages
            application_send_time,
            wire_send_start_time: None,
        }
    }

    /// Returns the time at which the message was first sent by the application
    pub fn application_send_time(&self) -> Option<SystemTime> {
        self.application_send_time
    }

    /// Marks the message as having been fully sent over the network wire,
    /// and emits the relevant latency metrics.
    pub fn mark_message_as_sent(&mut self) {
        // If this message is a streamed message fragment, there's no need to emit
        // any metrics (we only emit metrics for the head and tail of streamed messages).
        if self.message_stream_type == MessageStreamType::StreamedMessageFragment {
            return;
        }

        // Otherwise, emit the latency metrics
        if let Some(application_send_time) = self.application_send_time {
            if let Some(wire_send_start_time) = self.wire_send_start_time {
                // Calculate the application to wire send latency
                let application_to_wire_latency = wire_send_start_time
                    .duration_since(application_send_time)
                    .unwrap_or_default()
                    .as_secs_f64();

                // Calculate the wire send latency
                let wire_send_latency = wire_send_start_time
                    .elapsed()
                    .unwrap_or_default()
                    .as_secs_f64();

                // Update the application to wire latency metrics
                counters::observe_message_send_latency(
                    &self.network_id,
                    &self.protocol_id,
                    &self.message_send_type,
                    &self.message_stream_type,
                    &MessageLatencyType::ApplicationToWire,
                    application_to_wire_latency,
                );

                // Update the wire send latency metrics
                counters::observe_message_send_latency(
                    &self.network_id,
                    &self.protocol_id,
                    &self.message_send_type,
                    &self.message_stream_type,
                    &MessageLatencyType::WireSend,
                    wire_send_latency,
                );
            }
        }
    }

    /// Returns a reference to the network ID
    pub fn network_id(&self) -> &NetworkId {
        &self.network_id
    }

    /// Returns a reference to the protocol ID
    pub fn protocol_id(&self) -> &Option<ProtocolId> {
        &self.protocol_id
    }

    /// Updates the message type
    pub fn update_message_stream_type(&mut self, message_stream_type: MessageStreamType) {
        self.message_stream_type = message_stream_type;
    }

    /// Updates the time at which the message started being sent over the wire
    pub fn update_wire_send_start_time(&mut self) {
        self.wire_send_start_time = Some(SystemTime::now());
    }
}

/// A struct holding metadata about a received message.
/// Note: this is not sent along the wire, it is only used internally.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReceivedMessageMetadata {
    /// The network ID for the message
    network_id: NetworkId,

    /// The protocol ID for the message. This may not always be known (e.g.,
    /// when we first receive a message, or if we fail to deserialize a message).
    protocol_id: Option<ProtocolId>,

    /// The type of message being received. This may not always be known (e.g.,
    /// when we first receive a message, or if we fail to deserialize a message).
    message_receive_type: Option<MessageReceiveType>,

    /// The stream type of the message being received
    message_stream_type: MessageStreamType,

    /// The time at which the message was received over the wire
    wire_receive_time: SystemTime,
}

impl ReceivedMessageMetadata {
    pub fn new(network_id: NetworkId, wire_receive_time: SystemTime) -> Self {
        Self {
            network_id,
            protocol_id: None, // The protocol ID is not known at this point
            message_receive_type: None, // The message receive type is not known at this point
            message_stream_type: MessageStreamType::NonStreamedMessage, // Default to non-streamed messages
            wire_receive_time,
        }
    }

    /// Marks the message as having been received by the application,
    /// and emits the relevant latency metrics.
    pub fn mark_message_as_application_received(&mut self) {
        if let Some(message_receive_type) = &self.message_receive_type {
            // Calculate the application receive latency
            let application_receive_latency = self
                .wire_receive_time
                .elapsed()
                .unwrap_or_default()
                .as_secs_f64();

            // Update the wire to application receive latency metrics
            counters::observe_message_receive_latency(
                &self.network_id,
                &self.protocol_id,
                message_receive_type,
                &self.message_stream_type,
                application_receive_latency,
            );
        }
    }

    /// Returns a reference to the network ID
    pub fn network_id(&self) -> &NetworkId {
        &self.network_id
    }

    /// Returns a reference to the protocol ID
    pub fn protocol_id(&self) -> &Option<ProtocolId> {
        &self.protocol_id
    }

    /// Updates the message stream type
    pub fn update_message_stream_type(&mut self, message_stream_type: MessageStreamType) {
        self.message_stream_type = message_stream_type;
    }

    /// Updates the protocol ID and message receive type
    pub fn update_protocol_id_and_message_type(
        &mut self,
        protocol_id: ProtocolId,
        message_receive_type: MessageReceiveType,
    ) {
        self.protocol_id = Some(protocol_id);
        self.message_receive_type = Some(message_receive_type);
    }
}

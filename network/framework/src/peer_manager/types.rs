// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0
use crate::{
    counters,
    peer::DisconnectReason,
    peer_manager::PeerManagerError,
    protocols::{
        direct_send::Message,
        rpc::{InboundRpcRequest, OutboundRpcRequest},
        wire::messaging::v1::{DirectSendMsg, NetworkMessage, NetworkMessageAndMetadata, Priority},
    },
    transport::{Connection, ConnectionMetadata},
    ProtocolId,
};
use aptos_config::network_id::NetworkContext;
use aptos_types::{network_address::NetworkAddress, PeerId};
use futures::channel::oneshot;
use serde::Serialize;
use std::fmt;
use tokio::time::Instant;

/// A simple enum representing the types of message that can be sent.
/// This is used for tracking latency metrics across different message types.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageSendType {
    DirectSend,
    RpcRequest,
    RpcResponse,
    Error,
}

impl MessageSendType {
    pub fn get_label(&self) -> &'static str {
        match self {
            MessageSendType::DirectSend => "direct_send",
            MessageSendType::RpcRequest => "rpc_request",
            MessageSendType::RpcResponse => "rpc_response",
            MessageSendType::Error => "error",
        }
    }
}

/// A container holding messages with simple metadata
#[derive(Clone, Debug)]
pub struct MessageAndMetadata {
    message: Message,
    latency_metadata: MessageLatencyMetadata,
}

impl MessageAndMetadata {
    /// Creates a new message with the given metadata
    pub fn new(message: Message, latency_metadata: MessageLatencyMetadata) -> Self {
        Self {
            message,
            latency_metadata,
        }
    }

    /// Creates a new message with empty metadata. This is only used for testing.
    #[cfg(test)]
    pub fn new_empty_metadata(message: Message) -> Self {
        Self {
            message,
            latency_metadata: MessageLatencyMetadata::new_for_testing(),
        }
    }

    /// Returns a reference to the message
    pub fn get_message(&self) -> &Message {
        &self.message
    }

    /// Returns a mutable reference to the latency metadata
    pub fn get_latency_metadata(&mut self) -> &mut MessageLatencyMetadata {
        &mut self.latency_metadata
    }

    /// Transforms the message into a network message and metadata
    pub fn into_network_message_and_metadata(self) -> NetworkMessageAndMetadata {
        // Create the direct send message
        let network_message = NetworkMessage::DirectSendMsg(DirectSendMsg {
            protocol_id: self.message.protocol_id,
            priority: Priority::default(),
            raw_msg: Vec::from(self.message.mdata.as_ref()),
        });

        // Create and return the network message and metadata
        NetworkMessageAndMetadata::new_with_metadata(network_message, self.latency_metadata)
    }
}

/// A struct holding simple latency metadata for each network message
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MessageLatencyMetadata {
    message_send_type: MessageSendType, // Type of message being sent
    protocol_id: Option<ProtocolId>,    // Protocol ID of the message being monitored

    serialization_start_time: Option<Instant>, // Time when the message first started to be serialized
    peer_manager_dispatch_time: Option<Instant>, // Time when the message was first dispatched to the peer manager (after serialization)
    peer_dispatch_time: Option<Instant>, // Time when the message was first dispatched to the peer (after peer manager dispatch)
    message_write_time: Option<Instant>, // Time when the message was written to the peer socket (after peer dispatch)
}

impl MessageLatencyMetadata {
    /// Creates an empty message latency metadata
    pub fn new_empty(message_send_type: MessageSendType) -> Self {
        Self {
            message_send_type,
            protocol_id: None,
            serialization_start_time: None,
            peer_manager_dispatch_time: None,
            peer_dispatch_time: None,
            message_write_time: None,
        }
    }

    /// Creates an empty message latency metadata for testing.
    #[cfg(test)]
    pub fn new_for_testing() -> Self {
        Self::new_empty(MessageSendType::DirectSend)
    }

    /// Sets the serialization start time for the given protocol ID
    pub fn set_serialization_start_time(&mut self, protocol_id: ProtocolId) {
        self.protocol_id = Some(protocol_id);
        self.serialization_start_time = Some(Instant::now());
    }

    /// Sets the peer manager dispatch time
    pub fn set_peer_manager_dispatch_time(&mut self) {
        self.peer_manager_dispatch_time = Some(Instant::now());
    }

    /// Sets the peer dispatch time
    pub fn set_peer_dispatch_time(&mut self) {
        self.peer_dispatch_time = Some(Instant::now());
    }

    /// Sets the message write time
    pub fn set_message_write_time(&mut self) {
        self.message_write_time = Some(Instant::now());
    }

    /// Updates and emits the latency metrics for the given protocol ID
    pub fn emit_latency_metrics(&self) {
        // If the protocol ID is not set, do not emit any metrics
        let protocol_id = match self.protocol_id {
            Some(protocol_id) => protocol_id,
            None => return,
        };

        // Observe the serialization latency
        if let (Some(serialization_start_time), Some(peer_manager_dispatch_time)) = (
            self.serialization_start_time,
            self.peer_manager_dispatch_time,
        ) {
            let serialization_latency =
                peer_manager_dispatch_time.duration_since(serialization_start_time);
            counters::observe_outbound_message_queueing_latency(
                &self.message_send_type,
                &protocol_id,
                counters::QUEUEING_FOR_SERIALIZATION_LABEL,
                serialization_latency,
            );
        }

        // Observe the peer manager dispatch latency
        if let (Some(peer_manager_dispatch_time), Some(peer_dispatch_time)) =
            (self.peer_manager_dispatch_time, self.peer_dispatch_time)
        {
            let peer_manager_latency =
                peer_dispatch_time.duration_since(peer_manager_dispatch_time);
            counters::observe_outbound_message_queueing_latency(
                &self.message_send_type,
                &protocol_id,
                counters::QUEUEING_FOR_PEER_MANAGER_LABEL,
                peer_manager_latency,
            );
        }

        // Observe the peer dispatch latency
        if let (Some(peer_dispatch_time), Some(message_write_time)) =
            (self.peer_dispatch_time, self.message_write_time)
        {
            let peer_dispatch_latency = message_write_time.duration_since(peer_dispatch_time);
            counters::observe_outbound_message_queueing_latency(
                &self.message_send_type,
                &protocol_id,
                counters::QUEUEING_FOR_PEER_DISPATCH_LABEL,
                peer_dispatch_latency,
            );
        }

        // Observe the total latency
        if let (Some(message_write_time), Some(serialization_start_time)) =
            (self.message_write_time, self.serialization_start_time)
        {
            let total_latency = message_write_time.duration_since(serialization_start_time);
            counters::observe_outbound_message_queueing_latency(
                &self.message_send_type,
                &protocol_id,
                counters::QUEUEING_FOR_TOTAL_DURATION_LABEL,
                total_latency,
            );
        }
    }
}

/// Request received by PeerManager from upstream actors.
#[derive(Debug, Serialize)]
pub enum PeerManagerRequest {
    /// Send an RPC request to a remote peer.
    SendRpc(PeerId, #[serde(skip)] OutboundRpcRequest),
    /// Fire-and-forget style message send to a remote peer.
    SendDirectSend(PeerId, #[serde(skip)] MessageAndMetadata),
}

impl PeerManagerRequest {
    /// Creates and returns a new direct send message request
    pub fn new_direct_send(peer_id: PeerId, message_and_metadata: MessageAndMetadata) -> Self {
        Self::SendDirectSend(peer_id, message_and_metadata)
    }
}

/// Notifications sent by PeerManager to upstream actors.
#[derive(Debug)]
pub enum PeerManagerNotification {
    /// A new RPC request has been received from a remote peer.
    RecvRpc(PeerId, InboundRpcRequest),
    /// A new message has been received from a remote peer.
    RecvMessage(PeerId, Message),
}

impl PeerManagerNotification {
    /// Returns the peer ID of the notification
    pub fn get_peer_id(&self) -> PeerId {
        match self {
            PeerManagerNotification::RecvRpc(peer_id, _) => *peer_id,
            PeerManagerNotification::RecvMessage(peer_id, _) => *peer_id,
        }
    }
}

#[derive(Debug, Serialize)]
pub enum ConnectionRequest {
    DialPeer(
        PeerId,
        NetworkAddress,
        #[serde(skip)] oneshot::Sender<Result<(), PeerManagerError>>,
    ),
    DisconnectPeer(
        PeerId,
        #[serde(skip)] oneshot::Sender<Result<(), PeerManagerError>>,
    ),
}

#[derive(Clone, PartialEq, Eq, Serialize)]
pub enum ConnectionNotification {
    /// Connection with a new peer has been established.
    NewPeer(ConnectionMetadata, NetworkContext),
    /// Connection to a peer has been terminated. This could have been triggered from either end.
    LostPeer(ConnectionMetadata, NetworkContext, DisconnectReason),
}

impl fmt::Debug for ConnectionNotification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl fmt::Display for ConnectionNotification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConnectionNotification::NewPeer(metadata, context) => {
                write!(f, "[{},{}]", metadata, context)
            },
            ConnectionNotification::LostPeer(metadata, context, reason) => {
                write!(f, "[{},{},{}]", metadata, context, reason)
            },
        }
    }
}

#[derive(Debug, Serialize)]
pub enum TransportNotification<TSocket> {
    NewConnection(#[serde(skip)] Connection<TSocket>),
    Disconnected(ConnectionMetadata, DisconnectReason),
}

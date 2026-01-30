// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
use crate::{
    peer::{DisconnectReason, PeerRequest},
    peer_manager::PeerManagerError,
    protocols::{direct_send::Message, rpc::OutboundRpcRequest},
    transport::{Connection, ConnectionId, ConnectionMetadata},
    ProtocolId,
};
use aptos_channels::aptos_channel;
use aptos_config::network_id::NetworkId;
use aptos_types::{network_address::NetworkAddress, PeerId};
use futures::channel::oneshot;
use serde::Serialize;
use std::fmt;

/// Request received by PeerManager from upstream actors.
#[derive(Debug, Serialize)]
pub enum PeerManagerRequest {
    /// Send an RPC request to a remote peer.
    SendRpc(PeerId, #[serde(skip)] OutboundRpcRequest),
    /// Fire-and-forget style message send to a remote peer.
    SendDirectSend(PeerId, #[serde(skip)] Message),
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
        DisconnectReason,
        #[serde(skip)] oneshot::Sender<Result<(), PeerManagerError>>,
    ),
}

#[derive(Clone, PartialEq, Eq, Serialize)]
pub enum ConnectionNotification {
    /// Connection with a new peer has been established.
    NewPeer(ConnectionMetadata, NetworkId),
    /// Connection to a peer has been terminated. This could have been triggered from either end.
    LostPeer(ConnectionMetadata, NetworkId),
}

impl fmt::Debug for ConnectionNotification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl fmt::Display for ConnectionNotification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConnectionNotification::NewPeer(metadata, network_id) => {
                write!(f, "[{},{}]", metadata, network_id)
            },
            ConnectionNotification::LostPeer(metadata, network_id) => {
                write!(f, "[{},{}]", metadata, network_id)
            },
        }
    }
}

#[derive(Debug, Serialize)]
pub enum TransportNotification<TSocket> {
    NewConnection(#[serde(skip)] Connection<TSocket>),
    Disconnected(ConnectionMetadata, DisconnectReason),
}

/// Represents a single active connection to a peer.
/// Used by PeerManager to track multiple connections per peer.
#[derive(Debug)]
pub struct PeerConnection {
    pub connection_id: ConnectionId,
    pub peer_id: PeerId,
    pub metadata: ConnectionMetadata,
    pub sender: aptos_channel::Sender<ProtocolId, PeerRequest>,
}

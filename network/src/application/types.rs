// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{protocols::wire::handshake::v1::ProtocolId, transport::ConnectionMetadata};
use serde::{Deserialize, Serialize};

/// Errors related to the peer layer in the `NetworkInterface`
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PeerError {
    NotFound,
}

/// Descriptor of a Peer and how it should rank
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PeerInfo {
    pub status: PeerState,
    pub active_connection: ConnectionMetadata,
}

impl PeerInfo {
    pub fn new(connection_metadata: ConnectionMetadata) -> Self {
        PeerInfo {
            status: PeerState::Connected,
            active_connection: connection_metadata,
        }
    }

    pub fn is_connected(&self) -> bool {
        self.status == PeerState::Connected
    }

    pub fn supports_protocol(&self, protocol: ProtocolId) -> bool {
        self.active_connection
            .application_protocols
            .contains(protocol)
    }
}

/// The current state of a `Peer` at any one time
/// TODO: Allow nodes that are unhealthy to stay connected
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum PeerState {
    Connected,
    Disconnecting,
    Disconnected,
}

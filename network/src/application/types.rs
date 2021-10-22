// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::transport::ConnectionMetadata;

/// Errors related to the peer layer in the `NetworkInterface`
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PeerError {
    NotFound,
}

/// Descriptor of a Peer and how it should rank
#[derive(Clone, Debug)]
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
}

/// The current state of a `Peer` at any one time
/// TODO: Allow nodes that are unhealthy to stay connected
#[derive(Clone, Copy, Debug, Ord, PartialOrd, Eq, PartialEq)]
pub enum PeerState {
    Connected,
    Disconnecting,
    Disconnected,
}

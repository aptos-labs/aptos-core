// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    protocols::wire::handshake::v1::{ProtocolId, ProtocolIdSet},
    transport::ConnectionMetadata,
};
use aptos_config::network_id::PeerNetworkId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The current connection state of a peer
/// TODO: Allow nodes that are unhealthy to stay connected
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum ConnectionState {
    Connected,
    Disconnecting,
    Disconnected, // Currently unused (TODO: fix this!)
}

/// The peer monitoring metadata for a peer
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct PeerMonitoringMetadata {
    pub average_ping_latency_secs: Option<f64>, // The average latency ping for the peer
    pub connected_peers_and_metadata: Option<HashMap<PeerNetworkId, PeerMetadata>>, // Connected peers and metadata
    pub distance_from_validators: Option<u64>, // The known distance from the validator set
}

/// We must manually define this because f64 doesn't implement Eq. Instead,
/// we rely on PartialEq (which is sufficient for our use-cases).
impl Eq for PeerMonitoringMetadata {}

impl PeerMonitoringMetadata {
    pub fn new(
        average_ping_latency_secs: Option<f64>,
        connected_peers_and_metadata: Option<HashMap<PeerNetworkId, PeerMetadata>>,
        distance_from_validators: Option<u64>,
    ) -> Self {
        PeerMonitoringMetadata {
            average_ping_latency_secs,
            connected_peers_and_metadata,
            distance_from_validators,
        }
    }
}

/// A container holding all relevant metadata for the peer.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PeerMetadata {
    pub(crate) connection_state: ConnectionState,
    pub(crate) connection_metadata: ConnectionMetadata,
    pub(crate) peer_monitoring_metadata: PeerMonitoringMetadata,
}

impl PeerMetadata {
    pub fn new(connection_metadata: ConnectionMetadata) -> Self {
        PeerMetadata {
            connection_state: ConnectionState::Connected,
            connection_metadata,
            peer_monitoring_metadata: PeerMonitoringMetadata::default(),
        }
    }

    /// Creates and returns a new peer metadata for test environments
    #[cfg(any(test, feature = "fuzzing"))]
    pub fn new_for_test(
        connection_metadata: ConnectionMetadata,
        peer_monitoring_metadata: PeerMonitoringMetadata,
    ) -> Self {
        PeerMetadata {
            connection_state: ConnectionState::Connected,
            connection_metadata,
            peer_monitoring_metadata,
        }
    }

    /// Returns true iff the peer is still connected
    pub fn is_connected(&self) -> bool {
        self.connection_state == ConnectionState::Connected
    }

    /// Returns true iff the peer has advertised support for the given protocol
    pub fn supports_protocol(&self, protocol_id: ProtocolId) -> bool {
        self.connection_metadata
            .application_protocols
            .contains(protocol_id)
    }

    /// Returns true iff the peer has advertised support for at least
    /// one of the given protocols.
    pub fn supports_any_protocol(&self, protocol_ids: &[ProtocolId]) -> bool {
        let protocol_id_set = ProtocolIdSet::from_iter(protocol_ids);
        !self
            .connection_metadata
            .application_protocols
            .intersect(&protocol_id_set)
            .is_empty()
    }

    /// Returns the set of supported protocols for the peer
    pub fn get_supported_protocols(&self) -> ProtocolIdSet {
        self.connection_metadata.application_protocols.clone()
    }

    /// Returns a copy of the connection metadata
    pub fn get_connection_metadata(&self) -> ConnectionMetadata {
        self.connection_metadata.clone()
    }

    /// Returns a copy of the peer monitoring metadata
    pub fn get_peer_monitoring_metadata(&self) -> PeerMonitoringMetadata {
        self.peer_monitoring_metadata.clone()
    }
}

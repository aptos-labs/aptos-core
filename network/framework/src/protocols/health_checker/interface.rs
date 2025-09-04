// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    application::{
        error::Error, interface::NetworkClientInterface, metadata::ConnectionState,
        storage::PeersAndMetadata,
    },
    peer::DisconnectReason,
    protocols::{
        health_checker::{HealthCheckerMsg, HealthCheckerNetworkEvents},
        network::Event,
    },
};
use velor_config::network_id::PeerNetworkId;
use velor_infallible::RwLock;
use velor_types::PeerId;
use futures::{stream::FusedStream, Stream};
use std::{
    collections::HashMap,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

#[derive(Clone, Copy, Default, Debug, Eq, PartialEq)]
pub struct HealthCheckData {
    pub round: u64,
    pub failures: u64,
}

impl HealthCheckData {
    pub fn new(round: u64) -> Self {
        HealthCheckData { round, failures: 0 }
    }
}

/// HealthChecker's view into networking
pub struct HealthCheckNetworkInterface<NetworkClient> {
    health_check_data: RwLock<HashMap<PeerId, HealthCheckData>>,
    network_client: NetworkClient,
    receiver: HealthCheckerNetworkEvents,
}

impl<NetworkClient: NetworkClientInterface<HealthCheckerMsg>>
    HealthCheckNetworkInterface<NetworkClient>
{
    pub fn new(network_client: NetworkClient, receiver: HealthCheckerNetworkEvents) -> Self {
        Self {
            health_check_data: RwLock::new(HashMap::new()),
            network_client,
            receiver,
        }
    }

    // TODO: migrate this over to the network client once we
    // deduplicate the work.
    /// Returns all connected peers
    pub fn connected_peers(&self) -> Vec<PeerId> {
        self.health_check_data.read().keys().cloned().collect()
    }

    /// Disconnect a peer, and keep track of the associated state
    /// Note: This removes the peer outright for now until we add GCing, and historical state management
    pub async fn disconnect_peer(
        &mut self,
        peer_network_id: PeerNetworkId,
        disconnect_reason: DisconnectReason,
    ) -> Result<(), Error> {
        // Possibly already disconnected, but try anyways
        let _ = self.update_connection_state(peer_network_id, ConnectionState::Disconnecting);
        let result = self
            .network_client
            .disconnect_from_peer(peer_network_id, disconnect_reason)
            .await;
        let peer_id = peer_network_id.peer_id();
        if result.is_ok() {
            self.health_check_data.write().remove(&peer_id);
        }
        result
    }

    /// Update connection state of peer globally
    fn update_connection_state(
        &self,
        peer_network_id: PeerNetworkId,
        state: ConnectionState,
    ) -> Result<(), Error> {
        self.network_client
            .get_peers_and_metadata()
            .update_connection_state(peer_network_id, state)
    }

    /// Creates and saves new peer health data for the specified peer
    pub fn create_peer_and_health_data(&mut self, peer_id: PeerId, round: u64) {
        self.health_check_data
            .write()
            .entry(peer_id)
            .and_modify(|health_check_data| health_check_data.round = round)
            .or_insert_with(|| HealthCheckData::new(round));
    }

    /// Removes the peer and any associated health data
    pub fn remove_peer_and_health_data(&mut self, peer_id: &PeerId) {
        self.health_check_data.write().remove(peer_id);
    }

    /// Increments the number of failures for the specified round.
    /// If the round is in the past, nothing is done.
    pub fn increment_peer_round_failure(&mut self, peer_id: PeerId, round: u64) {
        if let Some(health_check_data) = self.health_check_data.write().get_mut(&peer_id) {
            if health_check_data.round <= round {
                health_check_data.failures += 1;
            }
        }
    }

    /// Resets the number of peer failures for the given peer.
    /// If the peer is not found, nothing is done.
    pub fn reset_peer_failures(&mut self, peer_id: PeerId) {
        if let Some(health_check_data) = self.health_check_data.write().get_mut(&peer_id) {
            health_check_data.failures = 0;
        }
    }

    /// Resets the state if the given round is newer than the
    /// currently stored round. Otherwise, nothing is done.
    pub fn reset_peer_round_state(&mut self, peer_id: PeerId, round: u64) {
        if let Some(health_check_data) = self.health_check_data.write().get_mut(&peer_id) {
            if round > health_check_data.round {
                health_check_data.round = round;
                health_check_data.failures = 0;
            }
        }
    }

    /// Returns the number of peer failures currently recorded
    pub fn get_peer_failures(&self, peer_id: PeerId) -> Option<u64> {
        self.health_check_data
            .read()
            .get(&peer_id)
            .map(|health_check_data| health_check_data.failures)
    }

    pub fn get_peers_and_metadata(&self) -> Arc<PeersAndMetadata> {
        self.network_client.get_peers_and_metadata()
    }

    // TODO: we shouldn't need to expose this
    pub fn network_client(&self) -> NetworkClient {
        self.network_client.clone()
    }
}

impl<NetworkClient: Unpin> Stream for HealthCheckNetworkInterface<NetworkClient> {
    type Item = Event<HealthCheckerMsg>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.get_mut().receiver).poll_next(cx)
    }
}

impl<NetworkClient: Unpin> FusedStream for HealthCheckNetworkInterface<NetworkClient> {
    fn is_terminated(&self) -> bool {
        self.receiver.is_terminated()
    }
}

// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    application::{
        interface::NetworkClientInterface,
        storage::LockingHashMap,
        types::{PeerError, PeerState},
    },
    error::NetworkError,
    protocols::{
        health_checker::{HealthCheckerMsg, HealthCheckerNetworkEvents},
        network::Event,
    },
};
use aptos_config::network_id::PeerNetworkId;
use aptos_types::PeerId;
use futures::{stream::FusedStream, Stream};
use std::{
    collections::hash_map::Entry,
    pin::Pin,
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
    health_check_data: LockingHashMap<PeerId, HealthCheckData>,
    network_client: NetworkClient,
    receiver: HealthCheckerNetworkEvents,
}

impl<NetworkClient: NetworkClientInterface<HealthCheckerMsg>>
    HealthCheckNetworkInterface<NetworkClient>
{
    pub fn new(network_client: NetworkClient, receiver: HealthCheckerNetworkEvents) -> Self {
        Self {
            health_check_data: LockingHashMap::new(),
            network_client,
            receiver,
        }
    }

    /// Disconnect a peer, and keep track of the associated state
    /// Note: This removes the peer outright for now until we add GCing, and historical state management
    pub async fn disconnect_peer(
        &mut self,
        peer_network_id: PeerNetworkId,
    ) -> Result<(), NetworkError> {
        // Possibly already disconnected, but try anyways
        let _ = self.update_state(peer_network_id, PeerState::Disconnecting);
        let result = self
            .network_client
            .disconnect_from_peer(peer_network_id)
            .await
            .map_err(NetworkError::from);
        let peer_id = peer_network_id.peer_id();
        if result.is_ok() {
            self.health_check_data.remove(&peer_id);
        }
        result
    }

    pub fn connected_peers(&self) -> Vec<PeerId> {
        self.health_check_data.keys()
    }

    /// Update state of peer globally
    fn update_state(
        &self,
        peer_network_id: PeerNetworkId,
        state: PeerState,
    ) -> Result<(), PeerError> {
        self.network_client.get_peer_metadata_storage().write(
            peer_network_id,
            |entry| match entry {
                Entry::Vacant(..) => Err(PeerError::NotFound),
                Entry::Occupied(inner) => {
                    inner.get_mut().status = state;
                    Ok(())
                },
            },
        )
    }

    pub fn health_check_data(&self) -> &LockingHashMap<PeerId, HealthCheckData> {
        &self.health_check_data
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

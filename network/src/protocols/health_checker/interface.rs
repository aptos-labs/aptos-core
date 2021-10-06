// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    application::{
        interface::NetworkInterface,
        storage::{LockingHashMap, PeerMetadataStorage},
        types::{PeerError, PeerState},
    },
    error::NetworkError,
    protocols::{
        health_checker::{
            HealthCheckerMsg, HealthCheckerNetworkEvents, HealthCheckerNetworkSender,
        },
        network::Event,
    },
};
use async_trait::async_trait;
use diem_config::network_id::PeerNetworkId;
use diem_types::PeerId;
use futures::{stream::FusedStream, Stream};
use std::{
    collections::hash_map::Entry,
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
pub struct HealthCheckNetworkInterface {
    peer_metadata_storage: Arc<PeerMetadataStorage>,
    app_data: LockingHashMap<PeerId, HealthCheckData>,
    sender: HealthCheckerNetworkSender,
    receiver: HealthCheckerNetworkEvents,
}

impl HealthCheckNetworkInterface {
    pub fn new(
        peer_metadata_storage: Arc<PeerMetadataStorage>,
        sender: HealthCheckerNetworkSender,
        receiver: HealthCheckerNetworkEvents,
    ) -> Self {
        Self {
            peer_metadata_storage,
            app_data: LockingHashMap::new(),
            sender,
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
        let peer_id = peer_network_id.peer_id();
        let result = self.sender.disconnect_peer(peer_id).await;
        if result.is_ok() {
            self.app_data().remove(&peer_id);
        }
        result
    }

    pub fn connected_peers(&self) -> Vec<PeerId> {
        self.app_data.keys()
    }

    /// Update state of peer globally
    fn update_state(
        &self,
        peer_network_id: PeerNetworkId,
        state: PeerState,
    ) -> Result<(), PeerError> {
        self.peer_metadata_storage()
            .write(peer_network_id, |entry| match entry {
                Entry::Vacant(..) => Err(PeerError::NotFound),
                Entry::Occupied(inner) => {
                    inner.get_mut().status = state;
                    Ok(())
                }
            })
    }
}

#[async_trait]
impl NetworkInterface<HealthCheckerMsg, HealthCheckerNetworkSender>
    for HealthCheckNetworkInterface
{
    type AppDataKey = PeerId;
    type AppData = HealthCheckData;

    fn peer_metadata_storage(&self) -> &PeerMetadataStorage {
        &self.peer_metadata_storage
    }

    fn sender(&self) -> HealthCheckerNetworkSender {
        self.sender.clone()
    }

    fn app_data(&self) -> &LockingHashMap<PeerId, HealthCheckData> {
        &self.app_data
    }
}

impl Stream for HealthCheckNetworkInterface {
    type Item = Event<HealthCheckerMsg>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.get_mut().receiver).poll_next(cx)
    }
}

impl FusedStream for HealthCheckNetworkInterface {
    fn is_terminated(&self) -> bool {
        self.receiver.is_terminated()
    }
}

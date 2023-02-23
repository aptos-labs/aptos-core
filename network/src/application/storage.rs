// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    application::{
        error::Error,
        metadata::{ConnectionState, PeerMetadata},
    },
    transport::{ConnectionId, ConnectionMetadata},
    ProtocolId,
};
use aptos_config::network_id::{NetworkId, PeerNetworkId};
use aptos_infallible::RwLock;
use aptos_types::PeerId;
use std::{
    collections::{hash_map::Entry, HashMap},
    sync::Arc,
};

/// A simple container that tracks all peers and peer metadata for the node.
/// This container is updated by both the networking code (e.g., for new
/// peer connections and lost peer connections), as well as individual
/// applications (e.g., peer monitoring service).
#[derive(Debug)]
pub struct PeersAndMetadata {
    peers_and_metadata: HashMap<NetworkId, RwLock<HashMap<PeerId, PeerMetadata>>>,
}

impl PeersAndMetadata {
    pub fn new(network_ids: &[NetworkId]) -> Arc<PeersAndMetadata> {
        // Create the container
        let mut peers_and_metadata = PeersAndMetadata {
            peers_and_metadata: HashMap::new(),
        };

        // Initialize each network mapping
        network_ids.iter().for_each(|network_id| {
            peers_and_metadata
                .peers_and_metadata
                .insert(*network_id, RwLock::new(HashMap::new()));
        });

        Arc::new(peers_and_metadata)
    }

    // TODO: cache this for frequent uses
    /// Returns all connected peers that support at least one of
    /// the given protocols.
    pub fn get_connected_supported_peers(
        &self,
        protocol_ids: &[ProtocolId],
    ) -> Result<Vec<PeerNetworkId>, Error> {
        let mut connected_supported_peers = Vec::new();
        for network_id in self.get_registered_networks() {
            let peer_metadata = self.get_peer_metadata_for_network(&network_id)?;
            for (peer_id, peer_metadata) in peer_metadata.read().iter() {
                if peer_metadata.is_connected() && peer_metadata.supports_any_protocol(protocol_ids)
                {
                    let peer_network_id = PeerNetworkId::new(network_id, *peer_id);
                    connected_supported_peers.push(peer_network_id);
                }
            }
        }

        Ok(connected_supported_peers)
    }

    /// Returns metadata for all peers currently connected to the node
    pub fn get_connected_peers_and_metadata(
        &self,
    ) -> Result<HashMap<PeerNetworkId, PeerMetadata>, Error> {
        let mut connected_peers_and_metadata = HashMap::new();
        for network_id in self.get_registered_networks() {
            let peer_metadata = self.get_peer_metadata_for_network(&network_id)?;
            for (peer_id, peer_metadata) in peer_metadata.read().iter() {
                if peer_metadata.is_connected() {
                    let peer_network_id = PeerNetworkId::new(network_id, *peer_id);
                    connected_peers_and_metadata.insert(peer_network_id, peer_metadata.clone());
                }
            }
        }

        Ok(connected_peers_and_metadata)
    }

    /// Returns the networks currently held in the container
    pub fn get_registered_networks(&self) -> impl Iterator<Item = NetworkId> + '_ {
        self.peers_and_metadata.keys().copied()
    }

    /// Returns the metadata for the specified peer
    pub fn get_metadata_for_peer(
        &self,
        peer_network_id: PeerNetworkId,
    ) -> Result<PeerMetadata, Error> {
        let peer_metadata = self.get_peer_metadata_for_network(&peer_network_id.network_id())?;
        peer_metadata
            .read()
            .get(&peer_network_id.peer_id())
            .cloned()
            .ok_or_else(|| {
                Error::UnexpectedError(format!(
                    "No metadata was found for the given peer: {:?}",
                    peer_network_id
                ))
            })
    }

    /// Updates the connection metadata associated with the given peer.
    /// If no peer metadata exists, a new one is created.
    pub fn insert_connection_metadata(
        &self,
        peer_network_id: PeerNetworkId,
        connection_metadata: ConnectionMetadata,
    ) -> Result<(), Error> {
        let peer_metadata_for_network =
            self.get_peer_metadata_for_network(&peer_network_id.network_id())?;
        peer_metadata_for_network
            .write()
            .entry(peer_network_id.peer_id())
            .and_modify(|peer_metadata| {
                peer_metadata.connection_metadata = connection_metadata.clone()
            })
            .or_insert_with(|| PeerMetadata::new(connection_metadata));

        Ok(())
    }

    /// Updates the connection state associated with the given peer.
    /// If no peer metadata exists, an error is returned.
    pub fn update_connection_state(
        &self,
        peer_network_id: PeerNetworkId,
        connection_state: ConnectionState,
    ) -> Result<(), Error> {
        let peer_metadata_for_network =
            self.get_peer_metadata_for_network(&peer_network_id.network_id())?;
        if let Some(peer_metadata) = peer_metadata_for_network
            .write()
            .get_mut(&peer_network_id.peer_id())
        {
            peer_metadata.connection_state = connection_state;
        } else {
            return Err(Error::UnexpectedError(format!(
                "No peer metadata was found for the given peer: {:?}",
                peer_network_id
            )));
        }

        Ok(())
    }

    /// Removes the peer metadata from the container. If the peer
    /// doesn't exist, or the connection id doesn't match, an error is
    /// returned. Otherwise, the existing peer metadata is returned.
    pub fn remove_peer_metadata(
        &self,
        peer_network_id: PeerNetworkId,
        connection_id: ConnectionId,
    ) -> Result<PeerMetadata, Error> {
        let peer_metadata_for_network =
            self.get_peer_metadata_for_network(&peer_network_id.network_id())?;

        if let Entry::Occupied(entry) = peer_metadata_for_network
            .write()
            .entry(peer_network_id.peer_id())
        {
            // Don't remove the peer if the connection doesn't match!
            // For now, remove the peer entirely, we could in the future
            // have multiple connections for a peer
            let active_connection_id = entry.get().connection_metadata.connection_id;
            if active_connection_id == connection_id {
                Ok(entry.remove())
            } else {
                Err(Error::UnexpectedError(format!(
                    "The peer connection id did not match! Given: {:?}, found: {:?}.",
                    connection_id, active_connection_id
                )))
            }
        } else {
            Err(Error::UnexpectedError(format!(
                "No peer metadata was found for the given peer: {:?}",
                peer_network_id
            )))
        }
    }

    /// A helper method that returns the peers and metadata for the specified network
    fn get_peer_metadata_for_network(
        &self,
        network_id: &NetworkId,
    ) -> Result<&RwLock<HashMap<PeerId, PeerMetadata>>, Error> {
        self.peers_and_metadata.get(network_id).ok_or_else(|| {
            Error::UnexpectedError(format!(
                "No peers or metadata was found for the given network: {:?}",
                network_id
            ))
        })
    }
}

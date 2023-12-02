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
use aptos_config::{
    config::{Peer, PeerSet},
    network_id::{NetworkId, PeerNetworkId},
};
use aptos_infallible::RwLock;
use aptos_peer_monitoring_service_types::PeerMonitoringMetadata;
use aptos_types::{account_address::AccountAddress, PeerId};
use arc_swap::ArcSwap;
use std::{
    collections::{hash_map::Entry, HashMap},
    ops::Deref,
    sync::{Arc, RwLockWriteGuard},
};

/// A simple container that tracks all peers and peer metadata for the node.
/// This container is updated by both the networking code (e.g., for new
/// peer connections and lost peer connections), as well as individual
/// applications (e.g., peer monitoring service).
#[derive(Debug)]
pub struct PeersAndMetadata {
    peers_and_metadata: RwLock<HashMap<NetworkId, HashMap<PeerId, PeerMetadata>>>,
    trusted_peers: HashMap<NetworkId, Arc<ArcSwap<PeerSet>>>,

    // We maintain a cached copy of the peers and metadata. This is useful to
    // reduce lock contention, as we expect very heavy and frequent reads,
    // but infrequent writes. The cache is updated on all underlying updates.
    //
    // TODO: should we remove this when generational versioning is supported?
    cached_peers_and_metadata: Arc<ArcSwap<HashMap<NetworkId, HashMap<PeerId, PeerMetadata>>>>,
}

impl PeersAndMetadata {
    pub fn new(network_ids: &[NetworkId]) -> Arc<PeersAndMetadata> {
        // Create the container
        let mut peers_and_metadata = PeersAndMetadata {
            peers_and_metadata: RwLock::new(HashMap::new()),
            trusted_peers: HashMap::new(),
            cached_peers_and_metadata: Arc::new(ArcSwap::from(Arc::new(HashMap::new()))),
        };

        // Initialize each network mapping and trusted peer set
        network_ids.iter().for_each(|network_id| {
            // Update the peers and metadata map
            peers_and_metadata
                .peers_and_metadata
                .write()
                .insert(*network_id, HashMap::new());

            // Update the trusted peer set
            peers_and_metadata.trusted_peers.insert(
                *network_id,
                Arc::new(ArcSwap::from(Arc::new(PeerSet::new()))),
            );
        });

        // Initialize the cached peers and metadata
        let cached_peers_and_metadata = peers_and_metadata.peers_and_metadata.read().clone();
        peers_and_metadata.set_cached_peers_and_metadata(cached_peers_and_metadata);

        // Return the peers and metadata container
        Arc::new(peers_and_metadata)
    }

    /// Returns all peers. Note: this will return disconnected and unhealthy peers, so
    /// it is not recommended for applications to use this interface. Instead,
    /// `get_connected_peers_and_metadata()` should be used.
    pub fn get_all_peers(&self) -> Result<Vec<PeerNetworkId>, Error> {
        // Get the cached peers and metadata
        let cached_peers_and_metadata = self.cached_peers_and_metadata.load();

        // Collect all peers
        let mut all_peers = Vec::new();
        for (network_id, peers_and_metadata) in cached_peers_and_metadata.iter() {
            for (peer_id, _) in peers_and_metadata.iter() {
                let peer_network_id = PeerNetworkId::new(*network_id, *peer_id);
                all_peers.push(peer_network_id);
            }
        }
        Ok(all_peers)
    }

    /// Returns metadata for all peers currently connected to the node
    pub fn get_connected_peers_and_metadata(
        &self,
    ) -> Result<HashMap<PeerNetworkId, PeerMetadata>, Error> {
        // Get the cached peers and metadata
        let cached_peers_and_metadata = self.cached_peers_and_metadata.load();

        // Collect all connected peers
        let mut connected_peers_and_metadata = HashMap::new();
        for (network_id, peers_and_metadata) in cached_peers_and_metadata.iter() {
            for (peer_id, peer_metadata) in peers_and_metadata.iter() {
                if peer_metadata.is_connected() {
                    let peer_network_id = PeerNetworkId::new(*network_id, *peer_id);
                    connected_peers_and_metadata.insert(peer_network_id, peer_metadata.clone());
                }
            }
        }
        Ok(connected_peers_and_metadata)
    }

    /// Returns all connected peers that support at least one of
    /// the given protocols.
    pub fn get_connected_supported_peers(
        &self,
        protocol_ids: &[ProtocolId],
    ) -> Result<Vec<PeerNetworkId>, Error> {
        // Get the cached peers and metadata
        let cached_peers_and_metadata = self.cached_peers_and_metadata.load();

        // Collect all connected peers that support at least one of the given protocols
        let mut connected_supported_peers = Vec::new();
        for (network_id, peers_and_metadata) in cached_peers_and_metadata.iter() {
            for (peer_id, peer_metadata) in peers_and_metadata.iter() {
                if peer_metadata.is_connected() && peer_metadata.supports_any_protocol(protocol_ids)
                {
                    let peer_network_id = PeerNetworkId::new(*network_id, *peer_id);
                    connected_supported_peers.push(peer_network_id);
                }
            }
        }
        Ok(connected_supported_peers)
    }

    /// Returns the metadata for the specified peer
    pub fn get_metadata_for_peer(
        &self,
        peer_network_id: PeerNetworkId,
    ) -> Result<PeerMetadata, Error> {
        // Get the cached peers and metadata
        let cached_peers_and_metadata = self.cached_peers_and_metadata.load();

        // Fetch the peers and metadata for the given network
        let network_id = peer_network_id.network_id();
        let peer_metadata_for_network = cached_peers_and_metadata
            .get(&network_id)
            .ok_or_else(|| missing_network_metadata_error(&network_id))?;

        // Get the metadata for the peer
        peer_metadata_for_network
            .get(&peer_network_id.peer_id())
            .cloned()
            .ok_or_else(|| missing_peer_metadata_error(&peer_network_id))
    }

    /// Returns the networks currently held in the container
    pub fn get_registered_networks(&self) -> impl Iterator<Item = NetworkId> + '_ {
        // Get the cached peers and metadata
        let cached_peers_and_metadata = self.cached_peers_and_metadata.load();

        // Collect all registered networks
        cached_peers_and_metadata
            .keys()
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
    }

    /// Updates the connection metadata associated with the given peer.
    /// If no peer metadata exists, a new one is created.
    pub fn insert_connection_metadata(
        &self,
        peer_network_id: PeerNetworkId,
        connection_metadata: ConnectionMetadata,
    ) -> Result<(), Error> {
        // Grab the write lock for the peer metadata
        let mut peers_and_metadata = self.peers_and_metadata.write();

        // Fetch the peer metadata for the given network
        let peer_metadata_for_network =
            get_peer_metadata_for_network(&peer_network_id, &mut peers_and_metadata)?;

        // Update the metadata for the peer or insert a new entry
        peer_metadata_for_network
            .entry(peer_network_id.peer_id())
            .and_modify(|peer_metadata| {
                peer_metadata.connection_metadata = connection_metadata.clone()
            })
            .or_insert_with(|| PeerMetadata::new(connection_metadata));

        // Update the cached peers and metadata
        self.set_cached_peers_and_metadata(peers_and_metadata.clone());

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
        // Grab the write lock for the peer metadata
        let mut peers_and_metadata = self.peers_and_metadata.write();

        // Fetch the peer metadata for the given network
        let peer_metadata_for_network =
            get_peer_metadata_for_network(&peer_network_id, &mut peers_and_metadata)?;

        // Remove the peer metadata for the peer
        let peer_metadata = if let Entry::Occupied(entry) =
            peer_metadata_for_network.entry(peer_network_id.peer_id())
        {
            // Don't remove the peer if the connection doesn't match!
            // For now, remove the peer entirely, we could in the future
            // have multiple connections for a peer
            let active_connection_id = entry.get().connection_metadata.connection_id;
            if active_connection_id == connection_id {
                entry.remove()
            } else {
                return Err(Error::UnexpectedError(format!(
                    "The peer connection id did not match! Given: {:?}, found: {:?}.",
                    connection_id, active_connection_id
                )));
            }
        } else {
            // Unable to find the peer metadata for the given peer
            return Err(missing_peer_metadata_error(&peer_network_id));
        };

        // Update the cached peers and metadata
        self.set_cached_peers_and_metadata(peers_and_metadata.clone());

        Ok(peer_metadata)
    }

    /// Updates the connection state associated with the given peer.
    /// If no peer metadata exists, an error is returned.
    pub fn update_connection_state(
        &self,
        peer_network_id: PeerNetworkId,
        connection_state: ConnectionState,
    ) -> Result<(), Error> {
        // Grab the write lock for the peer metadata
        let mut peers_and_metadata = self.peers_and_metadata.write();

        // Fetch the peer metadata for the given network
        let peer_metadata_for_network =
            get_peer_metadata_for_network(&peer_network_id, &mut peers_and_metadata)?;

        // Update the connection state for the peer
        if let Some(peer_metadata) = peer_metadata_for_network.get_mut(&peer_network_id.peer_id()) {
            peer_metadata.connection_state = connection_state;
        } else {
            // Unable to find the peer metadata for the given peer
            return Err(missing_peer_metadata_error(&peer_network_id));
        }

        // Update the cached peers and metadata
        self.set_cached_peers_and_metadata(peers_and_metadata.clone());

        Ok(())
    }

    /// Updates the peer monitoring state associated with the given peer.
    /// If no peer metadata exists, an error is returned.
    pub fn update_peer_monitoring_metadata(
        &self,
        peer_network_id: PeerNetworkId,
        peer_monitoring_metadata: PeerMonitoringMetadata,
    ) -> Result<(), Error> {
        // Grab the write lock for the peer metadata
        let mut peers_and_metadata = self.peers_and_metadata.write();

        // Fetch the peer metadata for the given network
        let peer_metadata_for_network =
            get_peer_metadata_for_network(&peer_network_id, &mut peers_and_metadata)?;

        // Update the peer monitoring metadata for the peer
        if let Some(peer_metadata) = peer_metadata_for_network.get_mut(&peer_network_id.peer_id()) {
            peer_metadata.peer_monitoring_metadata = peer_monitoring_metadata;
        } else {
            return Err(missing_peer_metadata_error(&peer_network_id));
        }

        // Update the cached peers and metadata
        self.set_cached_peers_and_metadata(peers_and_metadata.clone());

        Ok(())
    }

    /// Updates the cached peers and metadata using the given map
    fn set_cached_peers_and_metadata(
        &self,
        cached_peers_and_metadata: HashMap<NetworkId, HashMap<PeerId, PeerMetadata>>,
    ) {
        self.cached_peers_and_metadata
            .store(Arc::new(cached_peers_and_metadata));
    }

    /// Returns a clone of the trusted peer set for the given network ID
    pub fn get_trusted_peers(&self, network_id: &NetworkId) -> Result<PeerSet, Error> {
        let trusted_peers = self.get_trusted_peer_set_for_network(network_id)?;
        Ok(trusted_peers.load().clone().deref().clone())
    }

    /// Returns the trusted peer set for the given network ID
    fn get_trusted_peer_set_for_network(
        &self,
        network_id: &NetworkId,
    ) -> Result<Arc<ArcSwap<PeerSet>>, Error> {
        self.trusted_peers.get(network_id).cloned().ok_or_else(|| {
            Error::UnexpectedError(format!(
                "No trusted peers were found for the given network id: {:?}",
                network_id
            ))
        })
    }

    /// Returns the trusted peer state for the given peer (if one exists)
    pub fn get_trusted_peer_state(
        &self,
        peer_network_id: &PeerNetworkId,
    ) -> Result<Option<Peer>, Error> {
        let trusted_peers = self.get_trusted_peer_set_for_network(&peer_network_id.network_id())?;
        let trusted_peer_state = trusted_peers
            .load()
            .get(&peer_network_id.peer_id())
            .cloned();
        Ok(trusted_peer_state)
    }

    /// Updates the trusted peer set for the given network ID
    pub fn set_trusted_peers(
        &self,
        network_id: &NetworkId,
        trusted_peer_set: PeerSet,
    ) -> Result<(), Error> {
        let trusted_peers = self.get_trusted_peer_set_for_network(network_id)?;
        trusted_peers.store(Arc::new(trusted_peer_set));
        Ok(())
    }

    #[cfg(test)]
    /// Returns all internal maps (for testing purposes only)
    pub(crate) fn get_all_internal_maps(
        &self,
    ) -> (
        HashMap<NetworkId, HashMap<PeerId, PeerMetadata>>,
        HashMap<NetworkId, Arc<ArcSwap<PeerSet>>>,
        Arc<ArcSwap<HashMap<NetworkId, HashMap<PeerId, PeerMetadata>>>>,
    ) {
        let peers_and_metadata = self.peers_and_metadata.read().clone();
        let trusted_peers = self.trusted_peers.clone();
        let cached_peers_and_metadata = self.cached_peers_and_metadata.clone();

        (peers_and_metadata, trusted_peers, cached_peers_and_metadata)
    }
}

/// Returns the peer metadata for the given network
fn get_peer_metadata_for_network<'a>(
    peer_network_id: &'a PeerNetworkId,
    peers_and_metadata: &'a mut RwLockWriteGuard<
        HashMap<NetworkId, HashMap<AccountAddress, PeerMetadata>>,
    >,
) -> Result<&'a mut HashMap<AccountAddress, PeerMetadata>, Error> {
    match peers_and_metadata.get_mut(&peer_network_id.network_id()) {
        Some(peer_metadata_for_network) => Ok(peer_metadata_for_network),
        None => Err(missing_network_metadata_error(
            &peer_network_id.network_id(),
        )),
    }
}

/// A simple helper for returning a missing network metadata error
fn missing_network_metadata_error(network_id: &NetworkId) -> Error {
    Error::UnexpectedError(format!(
        "No metadata was found for the given network: {:?}",
        network_id
    ))
}

/// A simple helper for returning a missing peer metadata error
/// for the specified peer.
fn missing_peer_metadata_error(peer_network_id: &PeerNetworkId) -> Error {
    Error::UnexpectedError(format!(
        "No metadata was found for the given peer: {:?}",
        peer_network_id
    ))
}

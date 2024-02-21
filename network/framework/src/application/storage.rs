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
use aptos_types::PeerId;
use std::{
    collections::{hash_map::Entry, HashMap},
    sync::Arc,
};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU32, Ordering};
use aptos_config::config::{PeerRole, RoleType};
use aptos_config::network_id::NetworkContext;
use std::fmt;
use tokio::sync::mpsc::error::TrySendError;
use serde::Serialize;
use aptos_logger::info;

/// A simple container that tracks all peers and peer metadata for the node.
/// This container is updated by both the networking code (e.g., for new
/// peer connections and lost peer connections), as well as individual
/// applications (e.g., peer monitoring service).
#[derive(Debug)]
pub struct PeersAndMetadata {
    network_ids: Vec<NetworkId>,
    peers_and_metadata: RwLock<HashMap<NetworkId, HashMap<PeerId, PeerMetadata>>>,
    generation: AtomicU32,

    // trusted_peers have separate locking and access
    trusted_peers: HashMap<NetworkId, Arc<RwLock<PeerSet>>>,

    subscribers: RwLock<Vec<tokio::sync::mpsc::Sender<ConnectionNotification>>>,
}

pub static PEERS_AND_METADATA_SINGLETON: OnceLock<Arc<PeersAndMetadata>> = OnceLock::new();

impl PeersAndMetadata {
    pub fn new(network_ids: &[NetworkId]) -> Arc<PeersAndMetadata> {
        // Create the container
        let network_ids = network_ids.to_vec();
        let mut peers_and_metadata = PeersAndMetadata {
            network_ids,
            peers_and_metadata: RwLock::new(HashMap::new()),
            generation: AtomicU32::new(1),
            trusted_peers: HashMap::new(),
            subscribers: RwLock::new(vec![]),
        };

        // Initialize each network mapping and trusted peer set
        {
            let mut wat = peers_and_metadata.peers_and_metadata.write();
            peers_and_metadata.network_ids.iter().for_each(|network_id| {
                wat.insert(*network_id, HashMap::new());

                peers_and_metadata
                    .trusted_peers
                    .insert(*network_id, Arc::new(RwLock::new(PeerSet::new())));
            });
        }

        Arc::new(peers_and_metadata)
    }

    /// Returns the networks currently held in the container
    pub fn get_registered_networks(&self) -> impl Iterator<Item = NetworkId> + '_ {
        self.network_ids.iter().copied()
    }

    /// Returns all peers. Note: this will return disconnected and unhealthy peers, so
    /// it is not recommended for applications to use this interface. Instead,
    /// `get_connected_peers_and_metadata()` should be used.
    pub fn get_all_peers(&self) -> Vec<PeerNetworkId> {
        let mut all_peers = Vec::new();
        let read = self.peers_and_metadata.read();
        for (network_id, peers) in read.iter() {
            for (peer_id, _) in peers.iter() {
                let peer_network_id = PeerNetworkId::new(*network_id, *peer_id);
                all_peers.push(peer_network_id);
            }
        }
        all_peers
    }

    /// Return copy of list of peers if generation is different than previously held.
    /// May optionally filter for connected peers only.
    /// May optionally filter on match for any of a set of ProtocolId.
    pub fn get_all_peers_generational(
        &self,
        generation: u32,
        require_connected: bool,
        protocol_ids: &[ProtocolId],
    ) -> Option<(Vec<PeerNetworkId>, u32)> {
        let generation_test = self.generation.load(Ordering::SeqCst);
        if generation == generation_test {
            return None;
        }
        let mut all_peers = Vec::new();
        let read = self.peers_and_metadata.read();
        for (network_id, peers) in read.iter() {
            for (peer_id, peer_metadata) in peers.iter() {
                if require_connected && !peer_metadata.is_connected() {
                    continue;
                }
                if !protocol_ids.is_empty() && !peer_metadata.supports_any_protocol(protocol_ids) {
                    continue;
                }
                let peer_network_id = PeerNetworkId::new(*network_id, *peer_id);
                all_peers.push(peer_network_id);
            }
        }
        let generation_actual = self.generation.load(Ordering::SeqCst);
        Some((all_peers, generation_actual))
    }

    /// Return copy of list of peers if generation is different than previously held.
    /// May optionally filter for connected peers only.
    /// May optionally filter on match for any of a set of ProtocolId.
    pub fn get_all_peers_and_metadata_generational(
        &self,
        generation: u32,
        require_connected: bool,
        protocol_ids: &[ProtocolId],
    ) -> Option<(Vec<(PeerNetworkId,PeerMetadata)>, u32)> {
        let generation_test = self.generation.load(Ordering::SeqCst);
        if generation == generation_test {
            return None;
        }
        let mut all_data = Vec::new();
        let read = self.peers_and_metadata.read();
        for (network_id, peers) in read.iter() {
            for (peer_id, peer_metadata) in peers.iter() {
                if require_connected && !peer_metadata.is_connected() {
                    continue;
                }
                if (!protocol_ids.is_empty()) && !peer_metadata.supports_any_protocol(protocol_ids) {
                    continue;
                }
                let peer_network_id = PeerNetworkId::new(*network_id, *peer_id);
                all_data.push((peer_network_id, peer_metadata.clone()));
            }
        }
        let generation_actual = self.generation.load(Ordering::SeqCst);
        Some((all_data, generation_actual))
    }

    /// Returns all connected peers that support at least one of
    /// the given protocols.
    pub fn get_connected_supported_peers(
        &self,
        protocol_ids: &[ProtocolId],
    ) -> Result<Vec<PeerNetworkId>, Error> {
        let mut connected_supported_peers = Vec::new();
        let read = self.peers_and_metadata.read();
        for (network_id, peers) in read.iter() {
            for (peer_id, peer_metadata) in peers.iter() {
                if peer_metadata.is_connected() && peer_metadata.supports_any_protocol(protocol_ids)
                {
                    let peer_network_id = PeerNetworkId::new(*network_id, *peer_id);
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
        let read = self.peers_and_metadata.read();
        for (network_id, peers) in read.iter() {
            for (peer_id, peer_metadata) in peers.iter() {
                if peer_metadata.is_connected() {
                    let peer_network_id = PeerNetworkId::new(*network_id, *peer_id);
                    connected_peers_and_metadata.insert(peer_network_id, peer_metadata.clone());
                }
            }
        }

        Ok(connected_peers_and_metadata)
    }

    /// Returns the metadata for the specified peer
    pub fn get_metadata_for_peer(
        &self,
        peer_network_id: PeerNetworkId,
    ) -> Result<PeerMetadata, Error> {
        let read = self.peers_and_metadata.read();
        match read.get(&peer_network_id.network_id()) {
            None => {Err(missing_metadata_error(&peer_network_id))}
            Some(peers) => {
                match peers.get(&peer_network_id.peer_id()) {
                    None => {Err(missing_metadata_error(&peer_network_id))}
                    Some(pm) => {
                        Ok(pm.clone())
                    }
                }
            }
        }
    }

    /// Returns the trusted peer set for the given network ID
    pub fn get_trusted_peers(&self, network_id: &NetworkId) -> Result<Arc<RwLock<PeerSet>>, Error> {
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
        let network_id = peer_network_id.network_id();
        match self.trusted_peers.get(&network_id) {
            None => {
                Ok(None)
            }
            Some(wat) => {
                Ok(wat.read().get(&peer_network_id.peer_id()).cloned())
            }
        }
    }

    /// Updates the trusted peer set for the given network ID
    pub fn set_trusted_peers(
        &self,
        network_id: &NetworkId,
        trusted_peer_set: PeerSet,
    ) -> Result<(), Error> {
        let trusted_peers = self.trusted_peers.get(network_id).ok_or_else(|| Error::UnexpectedError(format!("unknown network: {:?}", network_id)))?;
        let mut ps = trusted_peers.write();
        ps.clear();
        ps.clone_from(&trusted_peer_set);
        Ok(())
    }

    fn broadcast(&self, event: ConnectionNotification) {
        let mut listeners = self.subscribers.write();
        let mut to_del = vec![];
        for i in 0..listeners.len() {
            let dest = listeners.get_mut(i).unwrap();
            match dest.try_send(event.clone()) {
                Ok(_) => {}
                Err(err) => match err {
                    TrySendError::Full(_) => {
                        // meh, drop message, maybe counter?
                    }
                    TrySendError::Closed(_) => {
                        to_del.push(i);
                    }
                }
            }
        }
        for evict in to_del.into_iter() {
            let llast = listeners.len() - 1;
            if evict != llast {
                listeners.swap(evict, llast);
            }
            listeners.pop();
        }
    }

    pub fn subscribe(&self) -> tokio::sync::mpsc::Receiver<ConnectionNotification> {
        let (sender, receiver) = tokio::sync::mpsc::channel(10); // TODO configure or name the constant or something
        let mut listeners = self.subscribers.write();
        listeners.push(sender);
        receiver
    }

    #[cfg(test)]
    pub fn close_subscribers(&self) {
        let mut listeners = self.subscribers.write();
        // drop all the senders to close them
        listeners.clear();
    }

    /// Updates the connection metadata associated with the given peer.
    /// If no peer metadata exists, a new one is created.
    pub fn insert_connection_metadata(
        &self,
        peer_network_id: PeerNetworkId,
        connection_metadata: ConnectionMetadata,
    ) -> Result<(), Error> {
        info!(
            peer = peer_network_id.to_string(),
            op = "icm",
            "pamtrace"
        );
        let mut writer = self.peers_and_metadata.write();
        let peer_metadata_for_network = writer.get_mut(&peer_network_id.network_id()).unwrap();
        self.generation.fetch_add(1 , Ordering::SeqCst);
        let net_context = NetworkContext::new(peer_role_to_role_type(connection_metadata.role), peer_network_id.network_id(), peer_network_id.peer_id());
        peer_metadata_for_network.entry(peer_network_id.peer_id())
            .and_modify(|peer_metadata| {
                peer_metadata.connection_metadata = connection_metadata.clone()
            })
            .or_insert_with(|| {
                PeerMetadata::new(connection_metadata.clone())
            });
        let event = ConnectionNotification::NewPeer(connection_metadata, net_context);
        self.broadcast(event);

        Ok(())
    }

    /// Updates the connection state associated with the given peer.
    /// If no peer metadata exists, an error is returned.
    pub fn update_connection_state(
        &self,
        peer_network_id: PeerNetworkId,
        connection_state: ConnectionState,
    ) -> Result<(), Error> {
        info!(
            peer = peer_network_id.to_string(),
            op = "ucs",
            "pamtrace"
        );
        let mut writer = self.peers_and_metadata.write();
        let peer_metadata_for_network = writer.get_mut(&peer_network_id.network_id()).unwrap();

        // Update the connection state for the peer or return a missing metadata error
        if let Some(peer_metadata) = peer_metadata_for_network
            .get_mut(&peer_network_id.peer_id())
        {
            self.generation.fetch_add(1 , Ordering::SeqCst);
            if connection_state != peer_metadata.connection_state {
                // update counter, if this gets messy, replace with full re-count below...
                match connection_state {
                    ConnectionState::Connected => {
                        info!("PeersAndMetadata {} inc", peer_metadata.connection_metadata.origin);
                    }
                    ConnectionState::Disconnecting | ConnectionState::Disconnected => {
                        if peer_metadata.connection_state == ConnectionState::Connected {
                            info!("PeersAndMetadata {} dec", peer_metadata.connection_metadata.origin);
                        }
                    }
                }
                peer_metadata.connection_state = connection_state;
            }
            Ok(())
        } else {
            Err(missing_metadata_error(&peer_network_id))
        }
    }

    /// Updates the peer monitoring state associated with the given peer.
    /// If no peer metadata exists, an error is returned.
    pub fn update_peer_monitoring_metadata(
        &self,
        peer_network_id: PeerNetworkId,
        peer_monitoring_metadata: PeerMonitoringMetadata,
    ) -> Result<(), Error> {
        info!(
            peer = peer_network_id.to_string(),
            op = "upmm",
            "pamtrace"
        );
        let mut writer = self.peers_and_metadata.write();
        let peer_metadata_for_network = writer.get_mut(&peer_network_id.network_id()).unwrap();

        // Update the peer monitoring metadata for the peer or return a missing metadata error
        if let Some(peer_metadata) = peer_metadata_for_network
            .get_mut(&peer_network_id.peer_id())
        {
            self.generation.fetch_add(1 , Ordering::SeqCst);
            peer_metadata.peer_monitoring_metadata = peer_monitoring_metadata;
            Ok(())
        } else {
            Err(missing_metadata_error(&peer_network_id))
        }
    }

    /// Removes the peer metadata from the container. If the peer
    /// doesn't exist, or the connection id doesn't match, an error is
    /// returned. Otherwise, the existing peer metadata is returned.
    pub fn remove_peer_metadata(
        &self,
        peer_network_id: PeerNetworkId,
        connection_id: ConnectionId,
    ) -> Result<PeerMetadata, Error> {
        info!(
            peer = peer_network_id.to_string(),
            op = "rpm",
            "pamtrace"
        );
        let mut writer = self.peers_and_metadata.write();
        let peer_metadata_for_network = writer.get_mut(&peer_network_id.network_id()).unwrap();

        // Remove the peer metadata for the peer or return a missing metadata error
        if let Entry::Occupied(entry) = peer_metadata_for_network
            .entry(peer_network_id.peer_id())
        {
            // Don't remove the peer if the connection doesn't match!
            // For now, remove the peer entirely, we could in the future
            // have multiple connections for a peer
            let active_connection_id = entry.get().connection_metadata.connection_id;
            if active_connection_id == connection_id {
                self.generation.fetch_add(1 , Ordering::SeqCst);

                let peer_metadata = entry.remove();
                // TODO: fix network context and disconnect reason?
                let nc = NetworkContext::new(RoleType::Validator, peer_network_id.network_id(), PeerId::ZERO);
                let event = ConnectionNotification::LostPeer(peer_metadata.connection_metadata.clone(), nc, DisconnectReason::Requested);
                self.broadcast(event);
                Ok(peer_metadata)
            } else {
                Err(Error::UnexpectedError(format!(
                    "The peer connection id did not match! Given: {:?}, found: {:?}.",
                    connection_id, active_connection_id
                )))
            }
        } else {
            Err(missing_metadata_error(&peer_network_id))
        }
    }

    #[cfg(test)]
    #[cfg(disabled)]
    /// Returns all internal maps (for testing purposes only)
    /// TODO: unused?
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

/// A simple helper for returning a missing metadata error
/// for the specified peer.
fn missing_metadata_error(peer_network_id: &PeerNetworkId) -> Error {
    Error::UnexpectedError(format!(
        "No metadata was found for the given peer: {:?}",
        peer_network_id
    ))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum DisconnectReason {
    Requested,
    ConnectionLost,
}

impl fmt::Display for DisconnectReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            DisconnectReason::Requested => "Requested",
            DisconnectReason::ConnectionLost => "ConnectionLost",
        };
        write!(f, "{}", s)
    }
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

pub fn peer_role_to_role_type(role: PeerRole) -> RoleType {
    match role {
        PeerRole::Validator => {RoleType::Validator}
        PeerRole::PreferredUpstream => {RoleType::Validator}
        PeerRole::Upstream => {RoleType::Validator}
        PeerRole::ValidatorFullNode => {RoleType::FullNode}
        PeerRole::Downstream => {RoleType::FullNode}
        PeerRole::Known => {RoleType::FullNode}
        PeerRole::Unknown => {RoleType::FullNode}
    }
}

// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    application::{
        error::Error,
        metadata::{ConnectionState, PeerMetadata},
    },
    counters,
    peer_manager::ConnectionNotification,
    transport::{ConnectionId, ConnectionMetadata},
    ProtocolId,
};
use velor_config::{
    config::{Peer, PeerSet},
    network_id::{NetworkId, PeerNetworkId},
};
use velor_infallible::{Mutex, RwLock};
use velor_logger::{sample, sample::SampleRate, warn};
use velor_peer_monitoring_service_types::PeerMonitoringMetadata;
use velor_types::{account_address::AccountAddress, PeerId};
use arc_swap::ArcSwap;
use std::{
    collections::{hash_map::Entry, HashMap},
    ops::Deref,
    sync::{Arc, RwLockWriteGuard},
    time::Duration,
};
use tokio::sync::mpsc::error::TrySendError;

// notification_backlog is how many ConnectionNotification items can be queued waiting for an app to receive them.
// Beyond this, new messages will be dropped if the app is not handling them fast enough.
// We make this big enough to fit an initial burst of _all_ the connected peers getting notified.
// Having 100 connected peers is common, 500 not unexpected
const NOTIFICATION_BACKLOG: usize = 1000;

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

    subscribers: Mutex<Vec<tokio::sync::mpsc::Sender<ConnectionNotification>>>,
}

impl PeersAndMetadata {
    pub fn new(network_ids: &[NetworkId]) -> Arc<PeersAndMetadata> {
        // Create the container
        let mut peers_and_metadata = PeersAndMetadata {
            peers_and_metadata: RwLock::new(HashMap::new()),
            trusted_peers: HashMap::new(),
            cached_peers_and_metadata: Arc::new(ArcSwap::from(Arc::new(HashMap::new()))),
            subscribers: Mutex::new(vec![]),
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
    pub fn get_all_peers(&self) -> Vec<PeerNetworkId> {
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
        all_peers
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
            .or_insert_with(|| PeerMetadata::new(connection_metadata.clone()));

        // Update the cached peers and metadata
        self.set_cached_peers_and_metadata(peers_and_metadata.clone());

        let event =
            ConnectionNotification::NewPeer(connection_metadata, peer_network_id.network_id());
        self.broadcast(event);

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
                let peer_metadata = entry.remove();
                let event = ConnectionNotification::LostPeer(
                    peer_metadata.connection_metadata.clone(),
                    peer_network_id.network_id(),
                );
                self.broadcast(event);
                peer_metadata
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

    fn broadcast(&self, event: ConnectionNotification) {
        let mut listeners = self.subscribers.lock();
        let mut to_del = vec![];
        for i in 0..listeners.len() {
            let dest = listeners.get_mut(i).unwrap();
            if let Err(err) = dest.try_send(event.clone()) {
                match err {
                    TrySendError::Full(_) => {
                        // Tried to send to an app, but the app isn't handling its messages fast enough.
                        // Drop message. Maybe increment a metrics counter?
                        sample!(
                            SampleRate::Duration(Duration::from_secs(1)),
                            warn!("PeersAndMetadata.broadcast() failed, some app is slow"),
                        );
                    },
                    TrySendError::Closed(_) => {
                        to_del.push(i);
                    },
                }
            }
        }
        for evict in to_del.into_iter() {
            listeners.swap_remove(evict);
        }
    }

    /// subscribe() returns a channel for receiving NewPeer/LostPeer events.
    /// subscribe() immediately sends all* current connections as NewPeer events.
    /// (* capped at NOTIFICATION_BACKLOG, currently 1000, use get_connected_peers() to be sure)
    pub fn subscribe(&self) -> tokio::sync::mpsc::Receiver<ConnectionNotification> {
        let (sender, receiver) = tokio::sync::mpsc::channel(NOTIFICATION_BACKLOG);
        let peers_and_metadata = self.peers_and_metadata.read();
        'outer: for (network_id, network_peers_and_metadata) in peers_and_metadata.iter() {
            for (_addr, peer_metadata) in network_peers_and_metadata.iter() {
                let event = ConnectionNotification::NewPeer(
                    peer_metadata.connection_metadata.clone(),
                    *network_id,
                );
                if let Err(err) = sender.try_send(event) {
                    warn!("could not send initial NewPeer on subscribe(): {:?}", err);
                    break 'outer;
                }
            }
        }
        // I expect the peers_and_metadata read lock to still be in effect until after listeners.push() below
        let mut listeners = self.subscribers.lock();
        listeners.push(sender);
        receiver
    }

    #[cfg(test)]
    pub fn close_subscribers(&self) {
        let mut listeners = self.subscribers.lock();
        // drop all the senders to close them
        listeners.clear();
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

    /// Sorts the give peer slice in the order of decreasing latency.
    pub fn sort_peers_by_latency(&self, network_id: NetworkId, peers: &mut [PeerId]) {
        let _timer = counters::OP_MEASURE
            .with_label_values(&["sort_peers"])
            .start_timer();

        let cached_peers_and_metadata = self.cached_peers_and_metadata.load();

        peers.sort_unstable_by(|peer_network_a, peer_network_b| {
            let get_latency = |&network_id, peer| -> f64 {
                cached_peers_and_metadata
                    .get(&network_id)
                    .and_then(|peers| peers.get(peer))
                    .and_then(|peer| {
                        peer.get_peer_monitoring_metadata()
                            .average_ping_latency_secs
                    })
                    .unwrap_or_default()
            };

            let a_latency = get_latency(&network_id, peer_network_a);
            let b_latency = get_latency(&network_id, peer_network_b);
            b_latency
                .partial_cmp(&a_latency)
                .expect("latency is never NaN")
        })
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

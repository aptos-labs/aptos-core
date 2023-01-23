// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    application::types::{PeerInfo, PeerState},
    transport::ConnectionMetadata,
};
use aptos_config::{
    config::Error,
    network_id::{NetworkId, PeerNetworkId},
};
use aptos_infallible::RwLock;
use aptos_types::{account_address::AccountAddress, PeerId};
use std::{
    collections::{hash_map::Entry, HashMap},
    fmt::Debug,
    sync::Arc,
};

// TODO: refactor and clean up this interface.

/// Metadata storage for peers across all of networking.  Splits storage of information across
/// networks to prevent different networks from affecting each other
#[derive(Debug)]
pub struct PeerMetadataStorage {
    storage: HashMap<NetworkId, RwLock<HashMap<PeerId, PeerInfo>>>,
}

impl PeerMetadataStorage {
    #[cfg(any(test, feature = "testing", feature = "fuzzing"))]
    pub fn test() -> Arc<PeerMetadataStorage> {
        PeerMetadataStorage::new(&[NetworkId::Validator])
    }

    /// Create a new `PeerMetadataStorage` `NetworkId`s must be known at construction time
    pub fn new(network_ids: &[NetworkId]) -> Arc<PeerMetadataStorage> {
        let mut peer_metadata_storage = PeerMetadataStorage {
            storage: HashMap::new(),
        };
        network_ids.iter().for_each(|network_id| {
            peer_metadata_storage
                .storage
                .insert(*network_id, RwLock::new(HashMap::new()));
        });
        Arc::new(peer_metadata_storage)
    }

    pub fn networks(&self) -> impl Iterator<Item = NetworkId> + '_ {
        self.storage.keys().copied()
    }

    /// Handle common logic of getting a network
    fn get_network(&self, network_id: NetworkId) -> &RwLock<HashMap<AccountAddress, PeerInfo>> {
        self.storage
            .get(&network_id)
            .unwrap_or_else(|| panic!("Unexpected network requested: {}", network_id))
    }

    pub fn read(&self, peer_network_id: PeerNetworkId) -> Option<PeerInfo> {
        let network = self.get_network(peer_network_id.network_id());
        network.read().get(&peer_network_id.peer_id()).cloned()
    }

    pub fn read_filtered<F: FnMut(&(&PeerId, &PeerInfo)) -> bool>(
        &self,
        network_id: NetworkId,
        filter: F,
    ) -> HashMap<PeerNetworkId, PeerInfo> {
        let network = self.get_network(network_id);
        let filtered_results: HashMap<PeerId, PeerInfo> = network
            .read()
            .iter()
            .filter(filter)
            .map(|(key, value)| (*key, value.clone()))
            .collect();
        filtered_results
            .iter()
            .map(|(peer_id, peer_info)| {
                (PeerNetworkId::new(network_id, *peer_id), peer_info.clone())
            })
            .collect()
    }

    pub fn keys(&self, network_id: NetworkId) -> Vec<PeerNetworkId> {
        let network = self.get_network(network_id);
        network
            .read()
            .keys()
            .into_iter()
            .map(|peer_id| PeerNetworkId::new(network_id, *peer_id))
            .collect()
    }

    /// Read a clone of the entire state
    pub fn read_all(&self, network_id: NetworkId) -> HashMap<PeerNetworkId, PeerInfo> {
        let network = self.get_network(network_id);
        network
            .read()
            .iter()
            .map(|(peer_id, peer_info)| {
                (PeerNetworkId::new(network_id, *peer_id), peer_info.clone())
            })
            .collect()
    }

    /// Insert new entry
    pub fn insert(&self, peer_network_id: PeerNetworkId, new_value: PeerInfo) {
        let _ = self
            .get_network(peer_network_id.network_id())
            .write()
            .insert(peer_network_id.peer_id(), new_value);
    }

    /// Remove old entries
    pub fn remove(&self, peer_network_id: &PeerNetworkId) {
        let _ = self
            .get_network(peer_network_id.network_id())
            .write()
            .remove(&peer_network_id.peer_id());
    }

    pub fn insert_connection(
        &self,
        network_id: NetworkId,
        connection_metadata: ConnectionMetadata,
    ) {
        let network = self.get_network(network_id);
        network
            .write()
            .entry(connection_metadata.remote_peer_id)
            .and_modify(|entry| entry.active_connection = connection_metadata.clone())
            .or_insert_with(|| PeerInfo::new(connection_metadata));
    }

    pub fn remove_connection(
        &self,
        network_id: NetworkId,
        connection_metadata: &ConnectionMetadata,
    ) {
        let network = self.get_network(network_id);

        // Don't remove the peer if the connection doesn't match!
        if let Entry::Occupied(entry) = network.write().entry(connection_metadata.remote_peer_id) {
            // For now, remove the peer entirely, we could in the future have multiple connections for a peer
            if entry.get().active_connection.connection_id == connection_metadata.connection_id {
                entry.remove();
            }
        }
    }

    pub fn update_peer_state(
        &self,
        peer_network_id: PeerNetworkId,
        peer_state: PeerState,
    ) -> Result<(), Error> {
        let network = self.get_network(peer_network_id.network_id());
        if let Entry::Occupied(mut entry) = network.write().entry(peer_network_id.peer_id()) {
            entry.get_mut().status = peer_state;
            Ok(())
        } else {
            Err(Error::Unexpected(format!(
                "Peer not found in storage! Peer: {:?}",
                peer_network_id
            )))
        }
    }
}

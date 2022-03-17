// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    application::types::{PeerError, PeerInfo},
    transport::ConnectionMetadata,
};
use aptos_config::network_id::{NetworkId, PeerNetworkId};
use aptos_infallible::{RwLock, RwLockWriteGuard};
use aptos_types::{account_address::AccountAddress, PeerId};
use std::{
    collections::{hash_map::Entry, HashMap},
    fmt::Debug,
    hash::Hash,
    sync::Arc,
};

/// Metadata storage for peers across all of networking.  Splits storage of information across
/// networks to prevent different networks from affecting each other
#[derive(Debug)]
pub struct PeerMetadataStorage {
    storage: HashMap<NetworkId, LockingHashMap<PeerId, PeerInfo>>,
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
                .insert(*network_id, LockingHashMap::new());
        });
        Arc::new(peer_metadata_storage)
    }

    pub fn networks(&self) -> impl Iterator<Item = NetworkId> + '_ {
        self.storage.keys().copied()
    }

    /// Handle common logic of getting a network
    fn get_network(&self, network_id: NetworkId) -> &LockingHashMap<AccountAddress, PeerInfo> {
        self.storage
            .get(&network_id)
            .unwrap_or_else(|| panic!("Unexpected network requested: {}", network_id))
    }

    pub fn read(&self, peer_network_id: PeerNetworkId) -> Option<PeerInfo> {
        let network = self.get_network(peer_network_id.network_id());
        network.read(&peer_network_id.peer_id())
    }

    pub fn read_filtered<F: FnMut(&(&PeerId, &PeerInfo)) -> bool>(
        &self,
        network_id: NetworkId,
        filter: F,
    ) -> HashMap<PeerNetworkId, PeerInfo> {
        to_peer_network_ids(
            network_id,
            self.get_network(network_id).read_filtered(filter),
        )
    }

    pub fn keys(&self, network_id: NetworkId) -> Vec<PeerNetworkId> {
        self.get_network(network_id)
            .keys()
            .into_iter()
            .map(|peer_id| PeerNetworkId::new(network_id, peer_id))
            .collect()
    }

    /// Read a clone of the entire state
    pub fn read_all(&self, network_id: NetworkId) -> HashMap<PeerNetworkId, PeerInfo> {
        to_peer_network_ids(network_id, self.get_network(network_id).read_all())
    }

    /// Insert new entry
    pub fn insert(&self, peer_network_id: PeerNetworkId, new_value: PeerInfo) {
        self.get_network(peer_network_id.network_id())
            .insert(peer_network_id.peer_id(), new_value)
    }

    /// Remove old entries
    pub fn remove(&self, peer_network_id: &PeerNetworkId) {
        self.get_network(peer_network_id.network_id())
            .remove(&peer_network_id.peer_id())
    }

    /// Take in a function to modify the data, must handle concurrency control with the input function
    pub fn write<F: FnOnce(&mut Entry<PeerId, PeerInfo>) -> Result<(), PeerError>>(
        &self,
        peer_network_id: PeerNetworkId,
        modifier: F,
    ) -> Result<(), PeerError> {
        self.get_network(peer_network_id.network_id())
            .write(peer_network_id.peer_id(), modifier)
    }

    /// Get the underlying `RwLock` of the map.  Usage is discouraged as it leads to the possiblity of
    /// leaving the lock held for a long period of time.  However, not everything fits into the `write`
    /// model.
    pub fn write_lock(
        &self,
        network_id: NetworkId,
    ) -> RwLockWriteGuard<'_, HashMap<PeerId, PeerInfo>> {
        self.get_network(network_id).write_lock()
    }

    pub fn insert_connection(
        &self,
        network_id: NetworkId,
        connection_metadata: ConnectionMetadata,
    ) {
        self.write_lock(network_id)
            .entry(connection_metadata.remote_peer_id)
            .and_modify(|entry| entry.active_connection = connection_metadata.clone())
            .or_insert_with(|| PeerInfo::new(connection_metadata));
    }

    pub fn remove_connection(
        &self,
        network_id: NetworkId,
        connection_metadata: &ConnectionMetadata,
    ) {
        let mut map = self.write_lock(network_id);

        // Don't remove the peer if the connection doesn't match!
        if let Entry::Occupied(entry) = map.entry(connection_metadata.remote_peer_id) {
            // For now, remove the peer entirely, we could in the future have multiple connections for a peer
            if entry.get().active_connection.connection_id == connection_metadata.connection_id {
                entry.remove();
            }
        }
    }
}

fn to_peer_network_ids(
    network_id: NetworkId,
    map: HashMap<PeerId, PeerInfo>,
) -> HashMap<PeerNetworkId, PeerInfo> {
    map.into_iter()
        .map(|(peer_id, peer_info)| (PeerNetworkId::new(network_id, peer_id), peer_info))
        .collect()
}

/// A generic locking hash map with ability to read before write consistency
#[derive(Debug)]
pub struct LockingHashMap<Key: Clone + Debug + Eq + Hash, Value: Clone + Debug> {
    map: RwLock<HashMap<Key, Value>>,
}

impl<Key, Value> LockingHashMap<Key, Value>
where
    Key: Clone + Debug + Eq + Hash,
    Value: Clone + Debug,
{
    pub fn new() -> Self {
        Self {
            map: RwLock::new(HashMap::new()),
        }
    }

    /// Get a clone of the value
    pub fn read(&self, key: &Key) -> Option<Value> {
        self.map.read().get(key).cloned()
    }

    /// Filtered read clone based on keys or values
    pub fn read_filtered<F: FnMut(&(&Key, &Value)) -> bool>(
        &self,
        filter: F,
    ) -> HashMap<Key, Value> {
        self.map
            .read()
            .iter()
            .filter(filter)
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    }

    /// All keys of the hash map
    pub fn keys(&self) -> Vec<Key> {
        self.map.read().keys().cloned().collect()
    }

    /// Read a clone of the entire state
    pub fn read_all(&self) -> HashMap<Key, Value> {
        self.map.read().clone()
    }

    /// Insert new entry
    pub fn insert(&self, key: Key, new_value: Value) {
        let mut map = self.map.write();
        map.entry(key)
            .and_modify(|value| *value = new_value.clone())
            .or_insert_with(|| new_value);
    }

    /// Remove old entries
    pub fn remove(&self, key: &Key) {
        let mut map = self.map.write();
        map.remove(key);
    }

    /// Take in a function to modify the data, must handle concurrency control with the input function
    pub fn write<F: FnOnce(&mut Entry<Key, Value>) -> Result<(), PeerError>>(
        &self,
        key: Key,
        modifier: F,
    ) -> Result<(), PeerError> {
        let mut map = self.map.write();
        modifier(&mut map.entry(key))
    }

    /// Get the underlying `RwLock` of the map.  Usage is discouraged as it leads to the possiblity of
    /// leaving the lock held for a long period of time.  However, not everything fits into the `write`
    /// model.
    pub fn write_lock(&self) -> RwLockWriteGuard<'_, HashMap<Key, Value>> {
        self.map.write()
    }
}

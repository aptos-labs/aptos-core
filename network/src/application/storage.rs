// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0
use crate::{
    application::{
        management::ConnectionStorage,
        types::{PeerError, PeerInfo},
    },
    transport::ConnectionMetadata,
};
use diem_infallible::RwLock;
use diem_types::PeerId;
use std::{
    collections::{hash_map::Entry, HashMap},
    fmt::Debug,
    hash::Hash,
};

pub type PeerMetadataStorage = LockingHashMap<PeerId, PeerInfo>;

/// A generic locking hash map with ability to read before write consistency
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
    pub fn read_filtered<F: FnMut(&(Key, Value)) -> bool>(&self, filter: F) -> HashMap<Key, Value> {
        self.map.read().clone().into_iter().filter(filter).collect()
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
}

impl ConnectionStorage for PeerMetadataStorage {
    fn insert_connection(&self, connection_metadata: ConnectionMetadata) {
        let peer_id = connection_metadata.remote_peer_id;
        self.map
            .write()
            .entry(peer_id)
            .and_modify(|entry| entry.active_connection = connection_metadata.clone())
            .or_insert_with(|| PeerInfo::new(connection_metadata));
    }

    fn remove_connection(&self, connection_metadata: ConnectionMetadata) {
        let peer_id = connection_metadata.remote_peer_id;
        let mut remove = true;
        let mut map = self.map.write();

        // Don't remove the peer if the connection doesn't match!
        if let Some(peer_info) = map.get(&peer_id) {
            remove = peer_info.active_connection == connection_metadata;
        }
        // For now, remove the peer entirely, we could in the future have multiple connections for a peer
        if remove {
            map.remove(&peer_id);
        }
    }
}

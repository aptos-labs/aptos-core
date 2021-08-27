// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    application::{
        interface::{NetworkInterface, PeerStateChange},
        management::{ConnectionStorage, PeerMetadataManagement},
        storage::{LockingHashMap, PeerMetadataStorage},
        types::{PeerError, PeerState},
    },
    transport::ConnectionMetadata,
};
use diem_types::PeerId;
use std::{collections::hash_map::Entry, sync::Arc};

/// Dummy network so we can test the interfaces
struct DummyNetworkInterface {
    peer_metadata_storage: Arc<PeerMetadataStorage>,
    app_data: LockingHashMap<PeerId, usize>,
}

impl NetworkInterface for DummyNetworkInterface {
    type Sender = ();
    type AppData = usize;

    fn peer_metadata_storage(&self) -> &PeerMetadataStorage {
        &self.peer_metadata_storage
    }

    fn sender(&self) -> Self::Sender {}

    fn insert_app_data(&self, peer_id: PeerId, data: Self::AppData) {
        self.app_data.insert(peer_id, data)
    }

    fn remove_app_data(&self, peer_id: &PeerId) {
        self.app_data.remove(peer_id)
    }

    fn read_app_data(&self, peer_id: &PeerId) -> Option<Self::AppData> {
        self.app_data.read(peer_id)
    }

    fn write_app_data<F: FnOnce(&mut Entry<PeerId, Self::AppData>) -> Result<(), PeerError>>(
        &self,
        peer_id: PeerId,
        modifier: F,
    ) -> Result<(), PeerError> {
        self.app_data.write(peer_id, modifier)
    }
}

impl PeerStateChange for DummyNetworkInterface {
    fn peer_metadata_storage(&self) -> &PeerMetadataStorage {
        &self.peer_metadata_storage
    }
}

#[test]
fn test_interface() {
    let peer_metadata_storage = Arc::new(PeerMetadataStorage::new());
    let interface = DummyNetworkInterface {
        peer_metadata_storage: peer_metadata_storage.clone(),
        app_data: LockingHashMap::new(),
    };
    let peer_management = PeerMetadataManagement::new(peer_metadata_storage);
    let peer_1 = PeerId::random();
    let peer_2 = PeerId::random();
    assert_eq!(0, interface.peers().len());
    assert_eq!(0, interface.connected_peers().len());

    peer_management.insert_connection(ConnectionMetadata::mock(peer_1));
    peer_management.insert_connection(ConnectionMetadata::mock(peer_2));
    assert_eq!(2, interface.peers().len());
    assert_eq!(2, interface.connected_peers().len());

    interface
        .update_state(peer_1, PeerState::Disconnected)
        .unwrap();
    assert_eq!(2, interface.peers().len());
    assert_eq!(1, interface.connected_peers().len());

    peer_management.remove_connection(ConnectionMetadata::mock(peer_2));
    assert_eq!(1, interface.peers().len());
    assert_eq!(0, interface.connected_peers().len());
}

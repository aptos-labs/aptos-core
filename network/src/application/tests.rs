// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    application::{
        interface::{NetworkClient, NetworkClientInterface},
        storage::PeerMetadataStorage,
        types::{PeerError, PeerInfo, PeerState},
    },
    transport::ConnectionMetadata,
};
use aptos_config::network_id::{NetworkId, PeerNetworkId};
use aptos_types::PeerId;
use serde::{Deserialize, Serialize};
use std::{
    collections::{hash_map::Entry, HashMap},
    sync::Arc,
};

#[derive(Clone, Serialize, Deserialize)]
struct DummyMessage {}

/// Retrieve only connected peers
fn connected_peers(
    peer_metadata_storage: Arc<PeerMetadataStorage>,
    network_id: NetworkId,
) -> HashMap<PeerNetworkId, PeerInfo> {
    filtered_peers(peer_metadata_storage, network_id, |(_, peer_info)| {
        peer_info.status == PeerState::Connected
    })
}

/// Filter peers with according `filter`
fn filtered_peers<F: FnMut(&(&PeerId, &PeerInfo)) -> bool>(
    peer_metadata_storage: Arc<PeerMetadataStorage>,
    network_id: NetworkId,
    filter: F,
) -> HashMap<PeerNetworkId, PeerInfo> {
    peer_metadata_storage.read_filtered(network_id, filter)
}

/// Retrieve PeerInfo for the node
fn peers(
    peer_metadata_storage: Arc<PeerMetadataStorage>,
    network_id: NetworkId,
) -> HashMap<PeerNetworkId, PeerInfo> {
    peer_metadata_storage.read_all(network_id)
}

#[test]
fn test_interface() {
    let peer_metadata_storage = PeerMetadataStorage::test();
    let network_client: NetworkClient<DummyMessage> = NetworkClient::new(
        vec![],
        vec![],
        HashMap::new(),
        peer_metadata_storage.clone(),
    );

    let network_id = NetworkId::Validator;
    let peer_1 = PeerId::random();
    let peer_2 = PeerId::random();
    assert_eq!(
        0,
        peers(network_client.get_peer_metadata_storage(), network_id).len()
    );
    assert_eq!(
        0,
        connected_peers(network_client.get_peer_metadata_storage(), network_id).len()
    );

    // Insert 2 connections, and we should have two active peers
    let connection_1 = ConnectionMetadata::mock(peer_1);
    let connection_2 = ConnectionMetadata::mock(peer_2);
    peer_metadata_storage.insert_connection(network_id, connection_1);
    peer_metadata_storage.insert_connection(network_id, connection_2.clone());
    assert_eq!(
        2,
        peers(network_client.get_peer_metadata_storage(), network_id).len()
    );
    assert_eq!(
        2,
        connected_peers(network_client.get_peer_metadata_storage(), network_id).len()
    );

    // Disconnecting / disconnected are not counted in active
    update_state(
        peer_metadata_storage.clone(),
        PeerNetworkId::new(network_id, peer_1),
        PeerState::Disconnecting,
    );
    assert_eq!(
        2,
        peers(network_client.get_peer_metadata_storage(), network_id).len()
    );
    assert_eq!(
        1,
        connected_peers(network_client.get_peer_metadata_storage(), network_id).len()
    );

    // Removing a connection with a different connection id doesn't remove it from storage
    let different_connection_2 = ConnectionMetadata::mock(peer_2);
    peer_metadata_storage.remove_connection(network_id, &different_connection_2);
    assert_eq!(
        2,
        peers(network_client.get_peer_metadata_storage(), network_id).len()
    );
    assert_eq!(
        1,
        connected_peers(network_client.get_peer_metadata_storage(), network_id).len()
    );

    // Removing the same connection id removes it
    peer_metadata_storage.remove_connection(network_id, &connection_2);
    assert_eq!(
        1,
        peers(network_client.get_peer_metadata_storage(), network_id).len()
    );
    assert_eq!(
        0,
        connected_peers(network_client.get_peer_metadata_storage(), network_id).len()
    );
}

fn update_state(
    peer_metadata_storage: Arc<PeerMetadataStorage>,
    peer_network_id: PeerNetworkId,
    state: PeerState,
) {
    peer_metadata_storage
        .write(peer_network_id, |entry| match entry {
            Entry::Vacant(..) => Err(PeerError::NotFound),
            Entry::Occupied(inner) => {
                inner.get_mut().status = state;
                Ok(())
            },
        })
        .unwrap()
}

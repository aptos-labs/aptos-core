// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    application::{
        error::Error,
        interface::{NetworkClient, NetworkClientInterface, NetworkServiceEvents},
        metadata::{ConnectionState, PeerMetadata},
        storage::PeersAndMetadata,
    },
    peer_manager::{
        ConnectionRequestSender, PeerManagerNotification, PeerManagerRequest,
        PeerManagerRequestSender,
    },
    protocols::{
        network::{Event, NetworkEvents, NetworkSender, NewNetworkEvents, NewNetworkSender},
        rpc::InboundRpcRequest,
        wire::handshake::v1::{ProtocolId, ProtocolIdSet},
    },
    transport::ConnectionMetadata,
};
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_config::network_id::{NetworkId, PeerNetworkId};
use aptos_types::PeerId;
use futures::channel::oneshot;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Debug, hash::Hash, sync::Arc, time::Duration};
use tokio::time::timeout;

// Useful test constants for timeouts
const MAX_CHANNEL_TIMEOUT_SECS: u64 = 1;
const MAX_MESSAGE_TIMEOUT_SECS: u64 = 2;

/// Represents a test message sent across the network
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct DummyMessage {
    pub message_contents: Option<u64>, // Dummy contents for verification
}

impl DummyMessage {
    pub fn new(message_contents: u64) -> Self {
        Self {
            message_contents: Some(message_contents),
        }
    }

    pub fn new_empty() -> Self {
        Self {
            message_contents: None,
        }
    }
}

#[test]
fn test_peers_and_metadata_simple_interface() {
    // Create the peers and metadata container
    let network_ids = vec![NetworkId::Validator, NetworkId::Vfn];
    let peers_and_metadata = PeersAndMetadata::new(&network_ids);

    // Verify the registered networks and that there are no available peers
    check_registered_networks(&peers_and_metadata, network_ids);
    check_connected_peers_and_metadata(&peers_and_metadata, vec![]);

    // Create two peers and initialize the connection metadata
    let (peer_network_id_1, connection_1) = create_peer_and_connection(
        NetworkId::Validator,
        vec![ProtocolId::MempoolDirectSend, ProtocolId::StorageServiceRpc],
        peers_and_metadata.clone(),
    );
    let (peer_network_id_2, connection_2) = create_peer_and_connection(
        NetworkId::Vfn,
        vec![ProtocolId::MempoolDirectSend, ProtocolId::ConsensusRpcBcs],
        peers_and_metadata.clone(),
    );

    // Verify the number of connected peers
    check_connected_peers_and_metadata(&peers_and_metadata, vec![
        peer_network_id_1,
        peer_network_id_2,
    ]);

    // Verify the supported peers by protocol type
    check_connected_supported_peers(&peers_and_metadata, &[ProtocolId::MempoolDirectSend], vec![
        peer_network_id_1,
        peer_network_id_2,
    ]);
    check_connected_supported_peers(&peers_and_metadata, &[ProtocolId::StorageServiceRpc], vec![
        peer_network_id_1,
    ]);
    check_connected_supported_peers(&peers_and_metadata, &[ProtocolId::ConsensusRpcBcs], vec![
        peer_network_id_2,
    ]);
    check_connected_supported_peers(
        &peers_and_metadata,
        &[ProtocolId::PeerMonitoringServiceRpc],
        vec![],
    );

    // Mark peer 1 as disconnected and verify it is no longer included
    mark_peer_disconnecting(&peers_and_metadata, peer_network_id_1);
    check_connected_peers_and_metadata(&peers_and_metadata, vec![peer_network_id_2]);
    check_connected_supported_peers(&peers_and_metadata, &[ProtocolId::MempoolDirectSend], vec![
        peer_network_id_2,
    ]);
    check_connected_supported_peers(
        &peers_and_metadata,
        &[ProtocolId::StorageServiceRpc],
        vec![],
    );

    // Mark peer 2 as disconnected and verify it is no longer included
    mark_peer_disconnecting(&peers_and_metadata, peer_network_id_2);
    check_connected_peers_and_metadata(&peers_and_metadata, vec![]);
    check_connected_supported_peers(
        &peers_and_metadata,
        &[ProtocolId::MempoolDirectSend],
        vec![],
    );

    // Reconnect both peers
    connect_peer(&peers_and_metadata, peer_network_id_1);
    connect_peer(&peers_and_metadata, peer_network_id_2);

    // Verify that removing a connection with a different connection id doesn't remove the peer
    remove_peer_metadata(
        &peers_and_metadata,
        peer_network_id_2,
        connection_1.connection_id.get_inner() + 9879,
    )
    .unwrap_err();
    check_connected_peers_and_metadata(&peers_and_metadata, vec![
        peer_network_id_1,
        peer_network_id_2,
    ]);
    check_connected_supported_peers(&peers_and_metadata, &[ProtocolId::MempoolDirectSend], vec![
        peer_network_id_1,
        peer_network_id_2,
    ]);

    // Verify that removing a connection with the same connection id works
    remove_peer_metadata(
        &peers_and_metadata,
        peer_network_id_2,
        connection_2.connection_id.get_inner(),
    )
    .unwrap();
    check_connected_peers_and_metadata(&peers_and_metadata, vec![peer_network_id_1]);
    check_connected_supported_peers(&peers_and_metadata, &[ProtocolId::MempoolDirectSend], vec![
        peer_network_id_1,
    ]);
    check_connected_supported_peers(&peers_and_metadata, &[ProtocolId::ConsensusRpcBcs], vec![]);
}

#[test]
fn test_peers_and_metadata_simple_errors() {
    // Create the peers and metadata container
    let network_ids = vec![NetworkId::Validator, NetworkId::Vfn];
    let peers_and_metadata = PeersAndMetadata::new(&network_ids);

    // Create two peers and initialize the connection metadata
    let (peer_network_1, _) = create_peer_and_connection(
        NetworkId::Validator,
        vec![ProtocolId::MempoolDirectSend, ProtocolId::StorageServiceRpc],
        peers_and_metadata.clone(),
    );
    let _ = create_peer_and_connection(
        NetworkId::Vfn,
        vec![ProtocolId::MempoolDirectSend, ProtocolId::ConsensusRpcBcs],
        peers_and_metadata.clone(),
    );

    // Verify that fetching metadata for an invalid peer returns an error
    let invalid_peer = PeerNetworkId::new(NetworkId::Validator, PeerId::random());
    peers_and_metadata
        .get_metadata_for_peer(invalid_peer)
        .unwrap_err();

    // Verify that updating the connection state for an invalid peer returns an error
    peers_and_metadata
        .update_connection_state(invalid_peer, ConnectionState::Connected)
        .unwrap_err();

    // Verify that removing the metadata for an invalid peer returns an error
    remove_peer_metadata(&peers_and_metadata, invalid_peer, 10).unwrap_err();

    // Verify that fetching metadata for a valid peer ID without a network entry returns an error
    let invalid_peer_network = PeerNetworkId::new(NetworkId::Public, peer_network_1.peer_id());
    peers_and_metadata
        .get_metadata_for_peer(invalid_peer_network)
        .unwrap_err();
}

#[test]
fn test_peers_and_metadata_cache() {
    // Create the peers and metadata container
    let network_ids = vec![NetworkId::Validator, NetworkId::Vfn];
    let peers_and_metadata = PeersAndMetadata::new(&network_ids);

    // Verify the connected supported peers cache is empty
    check_connected_supported_peers_cache_length(&peers_and_metadata, 0);

    // Attempt to get connected supported peers and verify the cache is updated
    let protocol_ids_1 = [ProtocolId::MempoolDirectSend, ProtocolId::ConsensusRpcBcs];
    check_connected_supported_peers(&peers_and_metadata, &protocol_ids_1, vec![]);
    check_connected_supported_peers_cache(&peers_and_metadata, &protocol_ids_1, vec![]);

    // Create a new peer and initialize the connection metadata
    let (peer_network_id_1, mut connection_1) = create_peer_and_connection(
        NetworkId::Validator,
        protocol_ids_1.to_vec(),
        peers_and_metadata.clone(),
    );

    // Verify the connected supported peers cache has been reset
    check_connected_supported_peers_cache_length(&peers_and_metadata, 0);

    // Get the connected supported peers and verify the cache has been updated
    check_connected_supported_peers(&peers_and_metadata, &protocol_ids_1, vec![
        peer_network_id_1,
    ]);
    check_connected_supported_peers_cache(&peers_and_metadata, &protocol_ids_1, vec![
        peer_network_id_1,
    ]);

    // Create a new peer and initialize the connection metadata
    let (peer_network_id_2, _) = create_peer_and_connection(
        NetworkId::Vfn,
        protocol_ids_1.to_vec(),
        peers_and_metadata.clone(),
    );

    // Verify the connected supported peers cache has been reset
    check_connected_supported_peers_cache_length(&peers_and_metadata, 0);

    // Verify we now have two supported peers for the same protocol ID set
    let both_peers = vec![peer_network_id_1, peer_network_id_2];
    check_connected_supported_peers(&peers_and_metadata, &protocol_ids_1, both_peers);

    // Update the connection metadata for peer 1 (with a new set of supported protocols)
    let protocol_ids_2 = [ProtocolId::StorageServiceRpc];
    connection_1.application_protocols = ProtocolIdSet::from_iter(protocol_ids_2);
    update_connection_metadata(&peers_and_metadata, peer_network_id_1, connection_1);

    // Verify the connected supported peers cache has been reset
    check_connected_supported_peers_cache_length(&peers_and_metadata, 0);

    // Verify we now have only one peer for the new protocol ID set
    check_connected_supported_peers(&peers_and_metadata, &protocol_ids_2, vec![
        peer_network_id_1,
    ]);
    check_connected_supported_peers_cache(&peers_and_metadata, &protocol_ids_2, vec![
        peer_network_id_1,
    ]);

    // Verify we now have only one peer for the old protocol ID set
    check_connected_supported_peers(&peers_and_metadata, &protocol_ids_1, vec![
        peer_network_id_2,
    ]);
    check_connected_supported_peers_cache(&peers_and_metadata, &protocol_ids_1, vec![
        peer_network_id_2,
    ]);
}

#[test]
fn test_peers_and_metadata_cache_connection_states() {
    // Create the peers and metadata container
    let network_ids = vec![NetworkId::Validator, NetworkId::Vfn];
    let peers_and_metadata = PeersAndMetadata::new(&network_ids);

    // Create a new peer and initialize the connection metadata
    let protocol_ids = [ProtocolId::MempoolDirectSend, ProtocolId::ConsensusRpcBcs];
    let (peer_network_id_1, connection_1) = create_peer_and_connection(
        NetworkId::Validator,
        protocol_ids.to_vec(),
        peers_and_metadata.clone(),
    );

    // Verify the connected supported peers cache has been reset
    check_connected_supported_peers_cache_length(&peers_and_metadata, 0);

    // Get the connected supported peers and verify the cache has been updated
    check_connected_supported_peers(&peers_and_metadata, &protocol_ids, vec![peer_network_id_1]);
    check_connected_supported_peers_cache(&peers_and_metadata, &protocol_ids, vec![
        peer_network_id_1,
    ]);

    // Create a new peer and initialize the connection metadata
    let (peer_network_id_2, _) = create_peer_and_connection(
        NetworkId::Vfn,
        protocol_ids.to_vec(),
        peers_and_metadata.clone(),
    );

    // Verify the connected supported peers cache has been reset
    check_connected_supported_peers_cache_length(&peers_and_metadata, 0);

    // Verify we now have two supported peers for the same protocol ID set
    let both_peers = vec![peer_network_id_1, peer_network_id_2];
    check_connected_supported_peers(&peers_and_metadata, &protocol_ids, both_peers.clone());

    // Disconnect peer 2 and verify the cache is reset
    disconnect_peer(&peers_and_metadata, peer_network_id_2);
    check_connected_supported_peers_cache_length(&peers_and_metadata, 0);

    // Verify we now have only one peer for the protocol ID set
    check_connected_supported_peers(&peers_and_metadata, &protocol_ids, vec![peer_network_id_1]);
    check_connected_supported_peers_cache(&peers_and_metadata, &protocol_ids, vec![
        peer_network_id_1,
    ]);

    // Reconnect peer 2 and verify the cache is reset
    connect_peer(&peers_and_metadata, peer_network_id_2);
    check_connected_supported_peers_cache_length(&peers_and_metadata, 0);

    // Verify we now have two supported peers for the same protocol ID set
    check_connected_supported_peers(&peers_and_metadata, &protocol_ids, both_peers.clone());

    // Mark peer 1 as disconnecting and verify the cache is reset
    mark_peer_disconnecting(&peers_and_metadata, peer_network_id_1);
    check_connected_supported_peers_cache_length(&peers_and_metadata, 0);

    // Verify we now have only one peer for the protocol ID set
    check_connected_supported_peers(&peers_and_metadata, &protocol_ids, vec![peer_network_id_2]);
    check_connected_supported_peers_cache(&peers_and_metadata, &protocol_ids, vec![
        peer_network_id_2,
    ]);

    // Reconnect peer 1 and verify the cache is reset
    connect_peer(&peers_and_metadata, peer_network_id_1);
    check_connected_supported_peers_cache_length(&peers_and_metadata, 0);

    // Verify we now have two supported peers for the same protocol ID set
    check_connected_supported_peers(&peers_and_metadata, &protocol_ids, both_peers);

    // Remove peer metadata for peer 1 and verify the cache is reset
    remove_peer_metadata(
        &peers_and_metadata,
        peer_network_id_1,
        connection_1.connection_id.get_inner(),
    )
    .unwrap();
    check_connected_supported_peers_cache_length(&peers_and_metadata, 0);

    // Verify the connected supported peers
    check_connected_supported_peers(&peers_and_metadata, &protocol_ids, vec![peer_network_id_2]);
    check_connected_supported_peers_cache(&peers_and_metadata, &protocol_ids, vec![
        peer_network_id_2,
    ]);
}

#[test]
fn test_peers_and_metadata_cache_hit() {
    // Create the peers and metadata container
    let network_ids = vec![NetworkId::Validator, NetworkId::Vfn, NetworkId::Public];
    let peers_and_metadata = PeersAndMetadata::new(&network_ids);

    // Verify the connected supported peers cache is empty
    check_connected_supported_peers_cache_length(&peers_and_metadata, 0);

    // Create a new peer and initialize the connection metadata
    let protocol_ids = [ProtocolId::ConsensusRpcBcs];
    let (peer_network_id_1, _) = create_peer_and_connection(
        NetworkId::Vfn,
        protocol_ids.to_vec(),
        peers_and_metadata.clone(),
    );

    // Verify the connected supported peers cache has been reset
    check_connected_supported_peers_cache_length(&peers_and_metadata, 0);

    // Get the connected supported peers and verify the cache is updated
    check_connected_supported_peers(&peers_and_metadata, &protocol_ids, vec![peer_network_id_1]);
    check_connected_supported_peers_cache_length(&peers_and_metadata, 1);

    // Repeatedly get the connected supported peers and verify the response
    for _ in 0..10 {
        check_connected_supported_peers(&peers_and_metadata, &protocol_ids, vec![
            peer_network_id_1,
        ]);
    }
    check_connected_supported_peers_cache_length(&peers_and_metadata, 1);

    // Manually overwrite the data in the cache and verify it is used
    let new_connected_supported_peers = vec![
        PeerNetworkId::new(NetworkId::Validator, PeerId::random()),
        PeerNetworkId::new(NetworkId::Public, PeerId::random()),
    ];
    let protocol_id_set = ProtocolIdSet::from_iter(protocol_ids);
    peers_and_metadata
        .get_connected_supported_peers_cache()
        .write()
        .insert(protocol_id_set, new_connected_supported_peers.clone());

    // Repeatedly get the connected supported peers and verify the response
    for _ in 0..10 {
        check_connected_supported_peers(
            &peers_and_metadata,
            &protocol_ids,
            new_connected_supported_peers.clone(),
        );
    }
    check_connected_supported_peers_cache_length(&peers_and_metadata, 1);
}

#[test]
fn test_peers_and_metadata_cache_protocol_ordering() {
    // Create the peers and metadata container
    let network_ids = vec![NetworkId::Validator, NetworkId::Vfn, NetworkId::Public];
    let peers_and_metadata = PeersAndMetadata::new(&network_ids);

    // Attempt to get connected supported peers and verify the cache is updated
    let protocol_ids = [
        ProtocolId::ConsensusRpcBcs,
        ProtocolId::ConsensusRpcJson,
        ProtocolId::MempoolDirectSend,
    ];
    check_connected_supported_peers(&peers_and_metadata, &protocol_ids, vec![]);
    check_connected_supported_peers_cache(&peers_and_metadata, &protocol_ids, vec![]);

    // Create a new peer and initialize the connection metadata
    let (peer_network_id_1, _) = create_peer_and_connection(
        NetworkId::Public,
        protocol_ids.to_vec(),
        peers_and_metadata.clone(),
    );

    // Verify the connected supported peers cache has been reset
    check_connected_supported_peers_cache_length(&peers_and_metadata, 0);

    // Get the connected supported peers and verify the cache has been updated
    check_connected_supported_peers(&peers_and_metadata, &protocol_ids, vec![peer_network_id_1]);
    check_connected_supported_peers_cache(&peers_and_metadata, &protocol_ids, vec![
        peer_network_id_1,
    ]);

    // Create another peer, but with the same protocol ids in reverse order
    let reverse_protocol_ids = [
        ProtocolId::MempoolDirectSend,
        ProtocolId::ConsensusRpcJson,
        ProtocolId::ConsensusRpcBcs,
    ];
    let (peer_network_id_2, _) = create_peer_and_connection(
        NetworkId::Vfn,
        reverse_protocol_ids.to_vec(),
        peers_and_metadata.clone(),
    );

    // Verify the connected supported peers cache has been reset
    check_connected_supported_peers_cache_length(&peers_and_metadata, 0);

    // Verify we now have two supported peers for the same protocol ID set
    let both_peers = vec![peer_network_id_1, peer_network_id_2];
    check_connected_supported_peers(
        &peers_and_metadata,
        &reverse_protocol_ids,
        both_peers.clone(),
    );
    check_connected_supported_peers(&peers_and_metadata, &protocol_ids, both_peers);

    // Verify there's only one entry in the cache
    check_connected_supported_peers_cache_length(&peers_and_metadata, 1);

    // Create another peer, but with the same protocol ids in a different order
    let shuffled_protocol_ids = [
        ProtocolId::ConsensusRpcJson,
        ProtocolId::MempoolDirectSend,
        ProtocolId::ConsensusRpcBcs,
    ];
    let (peer_network_id_3, _) = create_peer_and_connection(
        NetworkId::Validator,
        shuffled_protocol_ids.to_vec(),
        peers_and_metadata.clone(),
    );

    // Verify the connected supported peers cache has been reset
    check_connected_supported_peers_cache_length(&peers_and_metadata, 0);

    // Verify we now have three supported peers for the same protocol ID set
    let all_peers = vec![peer_network_id_1, peer_network_id_2, peer_network_id_3];
    check_connected_supported_peers(
        &peers_and_metadata,
        &shuffled_protocol_ids,
        all_peers.clone(),
    );
    check_connected_supported_peers(
        &peers_and_metadata,
        &reverse_protocol_ids,
        all_peers.clone(),
    );
    check_connected_supported_peers(&peers_and_metadata, &protocol_ids, all_peers);

    // Verify there's still only one entry in the cache
    check_connected_supported_peers_cache_length(&peers_and_metadata, 1);
}

#[test]
fn test_peers_and_metadata_cache_protocol_overlap() {
    // Create the peers and metadata container
    let network_ids = vec![NetworkId::Validator, NetworkId::Vfn, NetworkId::Public];
    let peers_and_metadata = PeersAndMetadata::new(&network_ids);

    // Create a new peer and initialize the connection metadata
    let peer_protocols_1 = [ProtocolId::ConsensusRpcBcs];
    let (peer_network_id_1, _) = create_peer_and_connection(
        NetworkId::Public,
        peer_protocols_1.to_vec(),
        peers_and_metadata.clone(),
    );

    // Get the connected supported peers and verify the cache has been updated
    let supported_protocols = [
        ProtocolId::ConsensusRpcBcs,
        ProtocolId::StorageServiceRpc,
        ProtocolId::MempoolDirectSend,
    ];
    check_connected_supported_peers(&peers_and_metadata, &supported_protocols, vec![
        peer_network_id_1,
    ]);
    check_connected_supported_peers_cache(&peers_and_metadata, &supported_protocols, vec![
        peer_network_id_1,
    ]);

    // Create another peer, but with different protocol support
    let peer_protocols_2 = [ProtocolId::StorageServiceRpc, ProtocolId::HealthCheckerRpc];
    let (peer_network_id_2, connection_2) = create_peer_and_connection(
        NetworkId::Vfn,
        peer_protocols_2.to_vec(),
        peers_and_metadata.clone(),
    );

    // Verify the connected supported peers cache has been reset
    check_connected_supported_peers_cache_length(&peers_and_metadata, 0);

    // Verify we now have two supported peers for the supported protocol set
    let both_peers = vec![peer_network_id_1, peer_network_id_2];
    check_connected_supported_peers(
        &peers_and_metadata,
        &supported_protocols,
        both_peers.clone(),
    );
    check_connected_supported_peers(&peers_and_metadata, &supported_protocols, both_peers);

    // Verify there's only one entry in the cache
    check_connected_supported_peers_cache_length(&peers_and_metadata, 1);

    // Create another peer, but with different protocol support
    let peer_protocols_3 = [ProtocolId::MempoolDirectSend, ProtocolId::StorageServiceRpc];
    let (peer_network_id_3, _) = create_peer_and_connection(
        NetworkId::Validator,
        peer_protocols_3.to_vec(),
        peers_and_metadata.clone(),
    );

    // Verify the connected supported peers cache has been reset
    check_connected_supported_peers_cache_length(&peers_and_metadata, 0);

    // Verify we now have three supported peers for the supported protocol set
    let all_peers = vec![peer_network_id_1, peer_network_id_2, peer_network_id_3];
    check_connected_supported_peers(&peers_and_metadata, &supported_protocols, all_peers);

    // Verify there's still only one entry in the cache
    check_connected_supported_peers_cache_length(&peers_and_metadata, 1);

    // Verify individual protocol supports
    check_connected_supported_peers(&peers_and_metadata, &peer_protocols_1, vec![
        peer_network_id_1,
    ]);
    check_connected_supported_peers(&peers_and_metadata, &peer_protocols_2, vec![
        peer_network_id_2,
        peer_network_id_3,
    ]);
    check_connected_supported_peers(&peers_and_metadata, &peer_protocols_3, vec![
        peer_network_id_2,
        peer_network_id_3,
    ]);

    // Verify there's now four entries in the cache
    check_connected_supported_peers_cache_length(&peers_and_metadata, 4);

    // Remove peer metadata for peer 2 and verify the cache is reset
    remove_peer_metadata(
        &peers_and_metadata,
        peer_network_id_2,
        connection_2.connection_id.get_inner(),
    )
    .unwrap();
    check_connected_supported_peers_cache_length(&peers_and_metadata, 0);
}

#[test]
fn test_network_client_available_peers() {
    // Create the peers and metadata container
    let network_ids = vec![NetworkId::Validator, NetworkId::Vfn, NetworkId::Public];
    let peers_and_metadata = PeersAndMetadata::new(&network_ids);

    // Create the network client
    let network_client: NetworkClient<DummyMessage> = NetworkClient::new(
        vec![
            ProtocolId::MempoolDirectSend,
            ProtocolId::ConsensusDirectSendJson,
        ],
        vec![ProtocolId::StorageServiceRpc],
        HashMap::new(),
        peers_and_metadata.clone(),
    );

    // Verify the registered networks and that there are no available peers
    check_registered_networks(&peers_and_metadata, network_ids);
    check_available_peers(&network_client, vec![]);

    // Create three peers and initialize the connection metadata
    let (peer_network_id_1, _) = create_peer_and_connection(
        NetworkId::Validator,
        vec![ProtocolId::MempoolDirectSend, ProtocolId::StorageServiceRpc],
        peers_and_metadata.clone(),
    );
    let (peer_network_id_2, connection_2) = create_peer_and_connection(
        NetworkId::Vfn,
        vec![
            ProtocolId::ConsensusDirectSendJson,
            ProtocolId::ConsensusRpcBcs,
        ],
        peers_and_metadata.clone(),
    );
    let (peer_network_id_3, mut connection_3) = create_peer_and_connection(
        NetworkId::Public,
        vec![ProtocolId::ConsensusRpcBcs, ProtocolId::HealthCheckerRpc],
        peers_and_metadata,
    );

    // Verify the correct number of available and connected peers
    let peers_and_metadata = network_client.get_peers_and_metadata();
    check_available_peers(&network_client, vec![peer_network_id_1, peer_network_id_2]);
    check_connected_peers_and_metadata(&peers_and_metadata, vec![
        peer_network_id_1,
        peer_network_id_2,
        peer_network_id_3,
    ]);

    // Mark peer 3 as disconnected
    disconnect_peer(&peers_and_metadata, peer_network_id_3);

    // Verify the correct number of available and connected peers
    check_available_peers(&network_client, vec![peer_network_id_1, peer_network_id_2]);
    check_connected_peers_and_metadata(&peers_and_metadata, vec![
        peer_network_id_1,
        peer_network_id_2,
    ]);

    // Remove peer 2
    remove_peer_metadata(
        &peers_and_metadata,
        peer_network_id_2,
        connection_2.connection_id.get_inner(),
    )
    .unwrap();

    // Verify the correct number of available and connected peers
    check_available_peers(&network_client, vec![peer_network_id_1]);
    check_connected_peers_and_metadata(&peers_and_metadata, vec![peer_network_id_1]);

    // Update peer 3 to reconnected with new protocol support
    connection_3.application_protocols = ProtocolIdSet::from_iter([ProtocolId::MempoolDirectSend]);
    update_connection_metadata(&peers_and_metadata, peer_network_id_3, connection_3);
    connect_peer(&peers_and_metadata, peer_network_id_3);

    // Verify the correct number of available and connected peers
    check_available_peers(&network_client, vec![peer_network_id_1, peer_network_id_3]);
    check_connected_peers_and_metadata(&peers_and_metadata, vec![
        peer_network_id_1,
        peer_network_id_3,
    ]);

    // Reconnect peer 2
    update_connection_metadata(&peers_and_metadata, peer_network_id_2, connection_2);

    // Verify the correct number of available and connected peers
    check_available_peers(&network_client, vec![
        peer_network_id_1,
        peer_network_id_2,
        peer_network_id_3,
    ]);
    check_connected_peers_and_metadata(&peers_and_metadata, vec![
        peer_network_id_1,
        peer_network_id_2,
        peer_network_id_3,
    ]);
}

#[tokio::test]
async fn test_network_client_missing_network_sender() {
    // Create the peers and metadata container
    let network_ids = vec![NetworkId::Validator, NetworkId::Vfn, NetworkId::Public];
    let peers_and_metadata = PeersAndMetadata::new(&network_ids);

    // Create the network client
    let network_client: NetworkClient<DummyMessage> = NetworkClient::new(
        vec![
            ProtocolId::MempoolDirectSend,
            ProtocolId::ConsensusDirectSendJson,
        ],
        vec![ProtocolId::ConsensusRpcBcs],
        HashMap::new(),
        peers_and_metadata.clone(),
    );

    // Verify the registered networks and that there are no available peers
    check_registered_networks(&peers_and_metadata, network_ids);
    check_available_peers(&network_client, vec![]);

    // Create two peers and initialize the connection metadata
    let _ = create_peer_and_connection(
        NetworkId::Validator,
        vec![ProtocolId::MempoolDirectSend, ProtocolId::StorageServiceRpc],
        peers_and_metadata.clone(),
    );
    let _ = create_peer_and_connection(
        NetworkId::Public,
        vec![
            ProtocolId::ConsensusDirectSendCompressed,
            ProtocolId::ConsensusRpcBcs,
        ],
        peers_and_metadata.clone(),
    );

    // Verify that sending a message to a peer without a network sender fails
    let bad_peer_network_id = PeerNetworkId::new(NetworkId::Vfn, PeerId::random());
    network_client
        .send_to_peer(DummyMessage::new_empty(), bad_peer_network_id)
        .unwrap_err();
    network_client
        .send_to_peer_rpc(
            DummyMessage::new_empty(),
            Duration::from_secs(MAX_MESSAGE_TIMEOUT_SECS),
            bad_peer_network_id,
        )
        .await
        .unwrap_err();

    // Verify that sending a message to all peers without a network simply logs the errors
    network_client
        .send_to_peers(DummyMessage::new_empty(), &[bad_peer_network_id])
        .unwrap();
}

#[tokio::test]
async fn test_network_client_senders_no_matching_protocols() {
    // Create the peers and metadata container
    let network_ids = vec![NetworkId::Validator, NetworkId::Vfn, NetworkId::Public];
    let peers_and_metadata = PeersAndMetadata::new(&network_ids);

    // Create a network client with network senders
    let (network_senders, _network_events, _outbound_request_receivers, _inbound_request_senders) =
        create_network_sender_and_events(&network_ids);
    let network_client: NetworkClient<DummyMessage> = NetworkClient::new(
        vec![ProtocolId::ConsensusDirectSendBcs],
        vec![ProtocolId::StorageServiceRpc],
        network_senders,
        peers_and_metadata.clone(),
    );

    // Verify the registered networks and that there are no available peers
    check_registered_networks(&peers_and_metadata, network_ids);
    check_available_peers(&network_client, vec![]);

    // Create two peers and initialize the connection metadata
    let (peer_network_id_1, _) = create_peer_and_connection(
        NetworkId::Validator,
        vec![ProtocolId::StorageServiceRpc],
        peers_and_metadata.clone(),
    );
    let (peer_network_id_2, _) = create_peer_and_connection(
        NetworkId::Vfn,
        vec![ProtocolId::ConsensusDirectSendBcs],
        peers_and_metadata.clone(),
    );

    // Verify that there are available peers
    check_available_peers(&network_client, vec![peer_network_id_1, peer_network_id_2]);

    // Verify that sending a message to a peer without a matching protocol fails
    network_client
        .send_to_peer(DummyMessage::new_empty(), peer_network_id_1)
        .unwrap_err();
    network_client
        .send_to_peer_rpc(
            DummyMessage::new_empty(),
            Duration::from_secs(MAX_MESSAGE_TIMEOUT_SECS),
            peer_network_id_2,
        )
        .await
        .unwrap_err();
}

#[tokio::test]
async fn test_network_client_network_senders_direct_send() {
    // Create the peers and metadata container
    let network_ids = [NetworkId::Validator, NetworkId::Vfn];
    let peers_and_metadata = PeersAndMetadata::new(&network_ids);

    // Create two peers and initialize the connection metadata
    let (peer_network_id_1, _) = create_peer_and_connection(
        NetworkId::Validator,
        vec![ProtocolId::MempoolDirectSend],
        peers_and_metadata.clone(),
    );
    let (peer_network_id_2, _) = create_peer_and_connection(
        NetworkId::Vfn,
        vec![
            ProtocolId::ConsensusDirectSendCompressed,
            ProtocolId::ConsensusDirectSendJson,
            ProtocolId::ConsensusDirectSendBcs,
        ],
        peers_and_metadata.clone(),
    );

    // Create a network client with network senders
    let (
        network_senders,
        network_events,
        mut outbound_request_receivers,
        mut inbound_request_senders,
    ) = create_network_sender_and_events(&network_ids);
    let network_client: NetworkClient<DummyMessage> = NetworkClient::new(
        vec![
            ProtocolId::MempoolDirectSend,
            ProtocolId::ConsensusDirectSendBcs,
            ProtocolId::ConsensusDirectSendJson,
            ProtocolId::ConsensusDirectSendCompressed,
        ],
        vec![],
        network_senders,
        peers_and_metadata.clone(),
    );

    // Extract the network and events
    let mut network_and_events = network_events.into_network_and_events();
    let mut validator_network_events = network_and_events.remove(&NetworkId::Validator).unwrap();
    let mut vfn_network_events = network_and_events.remove(&NetworkId::Vfn).unwrap();

    // Verify that direct send messages are sent on matching networks and protocols
    let dummy_message = DummyMessage::new(10101);
    for peer_network_id in &[peer_network_id_1, peer_network_id_2] {
        network_client
            .send_to_peer(dummy_message.clone(), *peer_network_id)
            .unwrap();
    }
    wait_for_network_event(
        peer_network_id_1,
        &mut outbound_request_receivers,
        &mut inbound_request_senders,
        &mut validator_network_events,
        false,
        Some(ProtocolId::MempoolDirectSend),
        None,
        dummy_message.clone(),
    )
    .await;
    wait_for_network_event(
        peer_network_id_2,
        &mut outbound_request_receivers,
        &mut inbound_request_senders,
        &mut vfn_network_events,
        false,
        Some(ProtocolId::ConsensusDirectSendBcs),
        None,
        dummy_message,
    )
    .await;

    // Verify that broadcast messages are sent on matching networks and protocols
    let dummy_message = DummyMessage::new(2323);
    network_client
        .send_to_peers(dummy_message.clone(), &[
            peer_network_id_1,
            peer_network_id_2,
        ])
        .unwrap();
    wait_for_network_event(
        peer_network_id_1,
        &mut outbound_request_receivers,
        &mut inbound_request_senders,
        &mut validator_network_events,
        false,
        Some(ProtocolId::MempoolDirectSend),
        None,
        dummy_message.clone(),
    )
    .await;
    wait_for_network_event(
        peer_network_id_2,
        &mut outbound_request_receivers,
        &mut inbound_request_senders,
        &mut vfn_network_events,
        false,
        Some(ProtocolId::ConsensusDirectSendBcs),
        None,
        dummy_message,
    )
    .await;
}

#[tokio::test]
async fn test_network_client_network_senders_rpc() {
    // Create the peers and metadata container
    let network_ids = [NetworkId::Validator, NetworkId::Vfn];
    let peers_and_metadata = PeersAndMetadata::new(&network_ids);

    // Create two peers and initialize the connection metadata
    let (peer_network_id_1, _) = create_peer_and_connection(
        NetworkId::Validator,
        vec![ProtocolId::StorageServiceRpc],
        peers_and_metadata.clone(),
    );
    let (peer_network_id_2, _) = create_peer_and_connection(
        NetworkId::Vfn,
        vec![
            ProtocolId::ConsensusRpcCompressed,
            ProtocolId::ConsensusRpcJson,
            ProtocolId::ConsensusRpcBcs,
        ],
        peers_and_metadata.clone(),
    );

    // Create a network client with network senders
    let (
        network_senders,
        network_events,
        mut outbound_request_receivers,
        mut inbound_request_senders,
    ) = create_network_sender_and_events(&network_ids);
    let network_client: NetworkClient<DummyMessage> = NetworkClient::new(
        vec![],
        vec![
            ProtocolId::StorageServiceRpc,
            ProtocolId::ConsensusRpcJson,
            ProtocolId::ConsensusRpcBcs,
            ProtocolId::ConsensusRpcCompressed,
        ],
        network_senders,
        peers_and_metadata.clone(),
    );

    // Extract the network and events
    let mut network_and_events = network_events.into_network_and_events();
    let mut validator_network_events = network_and_events.remove(&NetworkId::Validator).unwrap();
    let mut vfn_network_events = network_and_events.remove(&NetworkId::Vfn).unwrap();

    // Verify that rpc messages are sent on matching networks and protocols
    let dummy_message = DummyMessage::new(999);
    let rpc_timeout = Duration::from_secs(MAX_MESSAGE_TIMEOUT_SECS);
    for peer_network_id in [peer_network_id_1, peer_network_id_2] {
        let network_client = network_client.clone();
        let dummy_message = dummy_message.clone();

        // We need to spawn this on a separate thread, otherwise we'll block waiting for the response
        tokio::spawn(async move {
            network_client
                .send_to_peer_rpc(dummy_message.clone(), rpc_timeout, peer_network_id)
                .await
                .unwrap()
        });
    }
    wait_for_network_event(
        peer_network_id_1,
        &mut outbound_request_receivers,
        &mut inbound_request_senders,
        &mut validator_network_events,
        true,
        None,
        Some(ProtocolId::StorageServiceRpc),
        dummy_message.clone(),
    )
    .await;
    wait_for_network_event(
        peer_network_id_2,
        &mut outbound_request_receivers,
        &mut inbound_request_senders,
        &mut vfn_network_events,
        true,
        None,
        Some(ProtocolId::ConsensusRpcJson),
        dummy_message,
    )
    .await;
}

/// Verifies that the available peers are correct
fn check_available_peers(
    network_client: &NetworkClient<DummyMessage>,
    expected_peers: Vec<PeerNetworkId>,
) {
    let available_peers = network_client.get_available_peers().unwrap();
    compare_vectors_ignore_order(available_peers, expected_peers);
}

/// Verifies that the registered networks are correct
fn check_registered_networks(
    peers_and_metadata: &Arc<PeersAndMetadata>,
    expected_networks: Vec<NetworkId>,
) {
    // Get the registered networks
    let registered_networks = peers_and_metadata.get_registered_networks().collect();
    compare_vectors_ignore_order(registered_networks, expected_networks);
}

/// Verifies that the connected peers and metadata are correct
fn check_connected_peers_and_metadata(
    peers_and_metadata: &Arc<PeersAndMetadata>,
    expected_peers: Vec<PeerNetworkId>,
) {
    let connected_peers_and_metadata = peers_and_metadata
        .get_connected_peers_and_metadata()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    compare_vectors_ignore_order(connected_peers_and_metadata, expected_peers);
}

/// Verifies that the connected and supported peers are correct
fn check_connected_supported_peers(
    peers_and_metadata: &Arc<PeersAndMetadata>,
    protocol_ids: &[ProtocolId],
    expected_peers: Vec<PeerNetworkId>,
) {
    let connected_and_supported_peers = peers_and_metadata
        .get_connected_supported_peers(protocol_ids)
        .unwrap();
    compare_vectors_ignore_order(connected_and_supported_peers, expected_peers);
}

/// Verifies that the connected and supported peer cache length is correct
fn check_connected_supported_peers_cache_length(
    peers_and_metadata: &Arc<PeersAndMetadata>,
    expected_length: usize,
) {
    assert_eq!(
        peers_and_metadata
            .get_connected_supported_peers_cache()
            .read()
            .len(),
        expected_length
    );
}

/// Verifies that the connected and supported peer cache is valid
fn check_connected_supported_peers_cache(
    peers_and_metadata: &Arc<PeersAndMetadata>,
    protocol_ids: &[ProtocolId],
    expected_peers: Vec<PeerNetworkId>,
) {
    // Get the cache
    let connected_supported_peers_cache = peers_and_metadata
        .get_connected_supported_peers_cache()
        .read();

    // Verify an entry is found and that it is valid
    let protocol_ids = ProtocolIdSet::from_iter(protocol_ids);
    assert!(connected_supported_peers_cache.contains_key(&protocol_ids));
    let connected_and_supported_peers = connected_supported_peers_cache
        .get(&protocol_ids)
        .unwrap()
        .clone();
    compare_vectors_ignore_order(connected_and_supported_peers, expected_peers);
}

/// Compares two vectors and asserts equality, but
/// ignores item ordering in the vectors.
fn compare_vectors_ignore_order<T: Clone + Debug + Ord>(
    mut vector_1: Vec<T>,
    mut vector_2: Vec<T>,
) {
    vector_1.sort();
    vector_2.sort();
    assert_eq!(vector_1, vector_2);
}

/// Returns an aptos channel for testing
fn create_aptos_channel<K: Eq + Hash + Clone, T>(
) -> (aptos_channel::Sender<K, T>, aptos_channel::Receiver<K, T>) {
    aptos_channel::new(QueueStyle::FIFO, 10, None)
}

/// Creates a set of network senders and events for the specified
/// network IDs. Also returns the internal inbound and outbound
/// channels for emulating network message sends across the wire.
fn create_network_sender_and_events(
    network_ids: &[NetworkId],
) -> (
    HashMap<NetworkId, NetworkSender<DummyMessage>>,
    NetworkServiceEvents<DummyMessage>,
    HashMap<NetworkId, aptos_channel::Receiver<(PeerId, ProtocolId), PeerManagerRequest>>,
    HashMap<NetworkId, aptos_channel::Sender<(PeerId, ProtocolId), PeerManagerNotification>>,
) {
    let mut network_senders = HashMap::new();
    let mut network_and_events = HashMap::new();
    let mut outbound_request_receivers = HashMap::new();
    let mut inbound_request_senders = HashMap::new();

    for network_id in network_ids {
        // Create the peer manager and connection channels
        let (inbound_request_sender, inbound_request_receiver) = create_aptos_channel();
        let (outbound_request_sender, outbound_request_receiver) = create_aptos_channel();
        let (connection_outbound_sender, _connection_outbound_receiver) = create_aptos_channel();
        let (_connection_inbound_sender, connection_inbound_receiver) = create_aptos_channel();

        // Create the network sender and events
        let network_sender = NetworkSender::new(
            PeerManagerRequestSender::new(outbound_request_sender),
            ConnectionRequestSender::new(connection_outbound_sender),
        );
        let network_events =
            NetworkEvents::new(inbound_request_receiver, connection_inbound_receiver);

        // Save the sender, events and receivers
        network_senders.insert(*network_id, network_sender);
        network_and_events.insert(*network_id, network_events);
        outbound_request_receivers.insert(*network_id, outbound_request_receiver);
        inbound_request_senders.insert(*network_id, inbound_request_sender);
    }

    // Create the network service events
    let network_service_events = NetworkServiceEvents::new(network_and_events);

    (
        network_senders,
        network_service_events,
        outbound_request_receivers,
        inbound_request_senders,
    )
}

/// Creates a new peer and connection metadata using the
/// given network and protocols.
fn create_peer_and_connection(
    network_id: NetworkId,
    protocol_ids: Vec<ProtocolId>,
    peers_and_metadata: Arc<PeersAndMetadata>,
) -> (PeerNetworkId, ConnectionMetadata) {
    // Create the peer and connection metadata
    let peer_network_id = PeerNetworkId::new(network_id, PeerId::random());
    let mut connection = ConnectionMetadata::mock(peer_network_id.peer_id());
    connection.application_protocols = ProtocolIdSet::from_iter(protocol_ids);

    // Insert the connection into peers and metadata
    peers_and_metadata
        .insert_connection_metadata(peer_network_id, connection.clone())
        .unwrap();

    (peer_network_id, connection)
}

/// Marks the specified peer as disconnected
fn disconnect_peer(peers_and_metadata: &Arc<PeersAndMetadata>, peer_network_id: PeerNetworkId) {
    peers_and_metadata
        .update_connection_state(peer_network_id, ConnectionState::Disconnected)
        .unwrap();
}

/// Marks the specified peer as connected
fn connect_peer(peers_and_metadata: &Arc<PeersAndMetadata>, peer_network_id: PeerNetworkId) {
    peers_and_metadata
        .update_connection_state(peer_network_id, ConnectionState::Connected)
        .unwrap();
}

/// Marks the specified peer as disconnecting
fn mark_peer_disconnecting(
    peers_and_metadata: &Arc<PeersAndMetadata>,
    peer_network_id: PeerNetworkId,
) {
    peers_and_metadata
        .update_connection_state(peer_network_id, ConnectionState::Disconnecting)
        .unwrap();
}

/// Attempts to remove peer and metadata
fn remove_peer_metadata(
    peers_and_metadata: &Arc<PeersAndMetadata>,
    peer_network_id: PeerNetworkId,
    connection_id: u32,
) -> Result<PeerMetadata, Error> {
    peers_and_metadata.remove_peer_metadata(peer_network_id, connection_id.into())
}

/// Updates the connection metadata for the specified peer
fn update_connection_metadata(
    peers_and_metadata: &Arc<PeersAndMetadata>,
    peer_network_id_3: PeerNetworkId,
    connection_3: ConnectionMetadata,
) {
    peers_and_metadata
        .insert_connection_metadata(peer_network_id_3, connection_3)
        .unwrap();
}

/// Waits for a network event on the expected channels and
/// verifies the message contents.
async fn wait_for_network_event(
    expected_peer_network_id: PeerNetworkId,
    outbound_request_receivers: &mut HashMap<
        NetworkId,
        aptos_channel::Receiver<(PeerId, ProtocolId), PeerManagerRequest>,
    >,
    inbound_request_senders: &mut HashMap<
        NetworkId,
        aptos_channel::Sender<(PeerId, ProtocolId), PeerManagerNotification>,
    >,
    network_events: &mut NetworkEvents<DummyMessage>,
    is_rpc_request: bool,
    expected_direct_send_protocol_id: Option<ProtocolId>,
    expected_rpc_protocol_id: Option<ProtocolId>,
    expected_dummy_message: DummyMessage,
) {
    let expected_peer_id = expected_peer_network_id.peer_id();
    let expected_network_id = expected_peer_network_id.network_id();
    let message_wait_time = Duration::from_secs(MAX_MESSAGE_TIMEOUT_SECS);
    let channel_wait_time = Duration::from_secs(MAX_CHANNEL_TIMEOUT_SECS);

    // We first expect the message to be appear on the outbound request receivers
    let outbound_request_receiver = outbound_request_receivers
        .get_mut(&expected_network_id)
        .unwrap();
    match timeout(channel_wait_time, outbound_request_receiver.select_next_some()).await {
        Ok(peer_manager_request) => {
            let (protocol_id, peer_manager_notification) = match peer_manager_request {
                PeerManagerRequest::SendRpc(peer_id, outbound_rpc_request) => {
                    // Verify the request is correct
                    assert!(is_rpc_request);
                    assert_eq!(peer_id, expected_peer_id);
                    assert_eq!(Some(outbound_rpc_request.protocol_id), expected_rpc_protocol_id);
                    assert_eq!(outbound_rpc_request.timeout, message_wait_time);

                    // Create and return the peer manager notification
                    let inbound_rpc_request = InboundRpcRequest {
                        protocol_id: outbound_rpc_request.protocol_id,
                        data: outbound_rpc_request.data,
                        res_tx: oneshot::channel().0,
                    };
                    (outbound_rpc_request.protocol_id, PeerManagerNotification::RecvRpc(peer_id, inbound_rpc_request))
                }
                PeerManagerRequest::SendDirectSend(peer_id, message) => {
                    // Verify the request is correct
                    assert!(!is_rpc_request);
                    assert_eq!(peer_id, expected_peer_id);
                    assert_eq!(Some(message.protocol_id), expected_direct_send_protocol_id);

                    // Create and return the peer manager notification
                    (message.protocol_id, PeerManagerNotification::RecvMessage(peer_id, message))
                }
            };

            // Pass the message from the outbound request receivers to the inbound request
            // senders. This emulates network wire transfer.
            let inbound_request_sender = inbound_request_senders.get_mut(&expected_network_id).unwrap();
            inbound_request_sender.push((expected_peer_id, protocol_id), peer_manager_notification).unwrap();
        }
        Err(elapsed) => panic!(
            "Timed out while waiting to receive a message on the outbound receivers channel. Elapsed: {:?}",
            elapsed
        ),
    }

    // Now, verify the message is received by the network events and contains the correct contents
    match timeout(channel_wait_time, network_events.select_next_some()).await {
        Ok(dummy_event) => match dummy_event {
            Event::Message(peer_id, dummy_message) => {
                assert!(!is_rpc_request);
                assert_eq!(peer_id, expected_peer_id);
                assert_eq!(dummy_message, expected_dummy_message);
            },
            Event::RpcRequest(peer_id, dummy_message, protocol_id, _) => {
                assert!(is_rpc_request);
                assert_eq!(peer_id, expected_peer_id);
                assert_eq!(dummy_message, expected_dummy_message);
                assert_eq!(Some(protocol_id), expected_rpc_protocol_id);
            },
            _ => panic!("Invalid dummy event found: {:?}", dummy_event),
        },
        Err(elapsed) => panic!(
            "Timed out while waiting to receive a message on the network events receiver. Elapsed: {:?}",
            elapsed
        ),
    }
}

// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    moderator::UnhealthyPeerState,
    tests::{mock::MockClient, utils},
};
use velor_config::{
    config::{PeerRole, StorageServiceConfig},
    network_id::{NetworkId, PeerNetworkId},
};
use velor_netcore::transport::ConnectionOrigin;
use velor_network::{
    application::metadata::ConnectionState,
    protocols::wire::handshake::v1::{MessagingProtocolVersion, ProtocolIdSet},
    transport::{ConnectionId, ConnectionMetadata},
};
use velor_storage_service_types::{
    requests::{DataRequest, StorageServiceRequest, TransactionsWithProofRequest},
    responses::StorageServiceResponse,
    StorageServiceError,
};
use velor_time_service::MockTimeService;
use velor_types::{account_address::AccountAddress, network_address::NetworkAddress, PeerId};
use claims::assert_matches;
use dashmap::DashMap;
use std::{str::FromStr, sync::Arc, time::Duration};

#[tokio::test]
async fn test_request_moderator_ignore_pfn() {
    // Create test data
    let highest_synced_version = 100;
    let highest_synced_epoch = 10;

    // Create a storage service config for testing
    let max_invalid_requests_per_peer = 5;
    let storage_service_config = StorageServiceConfig {
        max_invalid_requests_per_peer,
        ..Default::default()
    };

    // Create the storage client and server
    let (mut mock_client, mut service, _, _, _) =
        MockClient::new(None, Some(storage_service_config));
    utils::update_storage_server_summary(
        &mut service,
        highest_synced_version,
        highest_synced_epoch,
    );

    // Get the request moderator and verify the initial state
    let request_moderator = service.get_request_moderator();
    let unhealthy_peer_states = request_moderator.get_unhealthy_peer_states();
    assert!(unhealthy_peer_states.is_empty());

    // Spawn the server
    tokio::spawn(service.start());

    // Process several invalid PFN requests
    let pfn_peer_network_id = PeerNetworkId::new(NetworkId::Public, PeerId::random());
    for _ in 0..max_invalid_requests_per_peer {
        // Send the invalid request
        let response = send_invalid_transaction_request(
            highest_synced_version,
            &mut mock_client,
            pfn_peer_network_id,
        )
        .await;

        // Verify we get an invalid request error
        assert_matches!(
            response.unwrap_err(),
            StorageServiceError::InvalidRequest(_)
        );
    }

    // Send another request and verify the PFN is now ignored
    let response = send_invalid_transaction_request(
        highest_synced_version,
        &mut mock_client,
        pfn_peer_network_id,
    )
    .await;
    assert_matches!(
        response.unwrap_err(),
        StorageServiceError::TooManyInvalidRequests(_)
    );

    // Process many invalid requests from a VFN and verify it is never ignored
    let vfn_peer_network_id = PeerNetworkId::new(NetworkId::Vfn, PeerId::random());
    for _ in 0..max_invalid_requests_per_peer * 2 {
        // Send the invalid request
        let response = send_invalid_transaction_request(
            highest_synced_version,
            &mut mock_client,
            vfn_peer_network_id,
        )
        .await;

        // Verify we get an invalid request error
        assert_matches!(
            response.unwrap_err(),
            StorageServiceError::InvalidRequest(_)
        );
    }

    // Verify the unhealthy peer states
    assert_eq!(unhealthy_peer_states.len(), 2);

    // Verify the unhealthy peer state for the PFN
    let unhealthy_pfn_state = unhealthy_peer_states.get(&pfn_peer_network_id).unwrap();
    assert!(unhealthy_pfn_state.is_ignored());

    // Verify the unhealthy peer state for the VFN
    let unhealthy_vfn_state = unhealthy_peer_states.get(&vfn_peer_network_id).unwrap();
    assert!(!unhealthy_vfn_state.is_ignored());
}

#[tokio::test]
async fn test_request_moderator_increase_time() {
    // Create test data
    let highest_synced_version = 500;
    let highest_synced_epoch = 3;

    // Create a storage service config for testing
    let max_invalid_requests_per_peer = 3;
    let min_time_to_ignore_peers_secs = 10;
    let storage_service_config = StorageServiceConfig {
        max_invalid_requests_per_peer,
        min_time_to_ignore_peers_secs,
        ..Default::default()
    };

    // Create the storage client and server
    let (mut mock_client, mut service, _, time_service, peers_and_metadata) =
        MockClient::new(None, Some(storage_service_config));
    utils::update_storage_server_summary(
        &mut service,
        highest_synced_version,
        highest_synced_epoch,
    );

    // Get the request moderator and unhealthy peer states
    let request_moderator = service.get_request_moderator();
    let unhealthy_peer_states = request_moderator.get_unhealthy_peer_states();

    // Create and connect a new peer
    let peer_network_id = PeerNetworkId::new(NetworkId::Public, PeerId::random());
    peers_and_metadata
        .insert_connection_metadata(
            peer_network_id,
            create_connection_metadata(peer_network_id.peer_id(), 0),
        )
        .unwrap();

    // Spawn the server
    tokio::spawn(service.start());

    // Go through several iterations of ignoring and refreshing a bad peer
    for i in 0..10 {
        // Process enough invalid requests to ignore the peer
        for _ in 0..max_invalid_requests_per_peer {
            // Send the invalid request
            let response = send_invalid_transaction_request(
                highest_synced_version,
                &mut mock_client,
                peer_network_id,
            )
            .await;

            // Verify we get an invalid request error
            assert_matches!(
                response.unwrap_err(),
                StorageServiceError::InvalidRequest(_)
            );
        }

        // Send the invalid request
        let response = send_invalid_transaction_request(
            highest_synced_version,
            &mut mock_client,
            peer_network_id,
        )
        .await;

        // Verify we get an error for too many invalid requests
        assert_matches!(
            response.unwrap_err(),
            StorageServiceError::TooManyInvalidRequests(_)
        );

        // Verify the peer is now ignored
        assert!(unhealthy_peer_states
            .get(&peer_network_id)
            .unwrap()
            .is_ignored());

        // Wait until the peer is no longer ignored
        wait_for_request_moderator_to_unblock_peer(
            unhealthy_peer_states.clone(),
            &time_service,
            &peer_network_id,
            min_time_to_ignore_peers_secs * 2_i32.pow(i) as u64,
        )
        .await;
    }
}

#[tokio::test]
async fn test_request_moderator_peer_garbage_collect() {
    // Create test data
    let highest_synced_version = 500;
    let highest_synced_epoch = 3;

    // Create a storage service config for testing
    let max_invalid_requests_per_peer = 3;
    let storage_service_config = StorageServiceConfig {
        max_invalid_requests_per_peer,
        ..Default::default()
    };

    // Create the storage client and server
    let (mut mock_client, mut service, _, time_service, peers_and_metadata) =
        MockClient::new(None, Some(storage_service_config));
    utils::update_storage_server_summary(
        &mut service,
        highest_synced_version,
        highest_synced_epoch,
    );

    // Get the request moderator and unhealthy peer states
    let request_moderator = service.get_request_moderator();
    let unhealthy_peer_states = request_moderator.get_unhealthy_peer_states();

    // Connect multiple peers
    let peer_network_ids = [
        PeerNetworkId::new(NetworkId::Validator, PeerId::random()),
        PeerNetworkId::new(NetworkId::Vfn, PeerId::random()),
        PeerNetworkId::new(NetworkId::Public, PeerId::random()),
    ];
    for (index, peer_network_id) in peer_network_ids.iter().enumerate() {
        peers_and_metadata
            .insert_connection_metadata(
                *peer_network_id,
                create_connection_metadata(peer_network_id.peer_id(), index as u32),
            )
            .unwrap();
    }

    // Spawn the server
    tokio::spawn(service.start());

    // Send an invalid request from the first two peers
    for peer_network_id in peer_network_ids.iter().take(2) {
        // Send the invalid request
        send_invalid_transaction_request(
            highest_synced_version,
            &mut mock_client,
            *peer_network_id,
        )
        .await
        .unwrap_err();

        // Verify the peer is now tracked as unhealthy
        assert!(unhealthy_peer_states.contains_key(peer_network_id));
    }

    // Verify that only the first two peers are being tracked
    assert_eq!(unhealthy_peer_states.len(), 2);

    // Disconnect the first peer
    peers_and_metadata
        .update_connection_state(peer_network_ids[0], ConnectionState::Disconnecting)
        .unwrap();

    // Elapse enough time for the peer monitor loop to garbage collect the peer
    wait_for_request_moderator_to_garbage_collect(
        unhealthy_peer_states.clone(),
        &time_service,
        &peer_network_ids[0],
    )
    .await;

    // Verify that only the second peer is being tracked
    assert_eq!(unhealthy_peer_states.len(), 1);

    // Disconnect the second peer
    peers_and_metadata
        .remove_peer_metadata(peer_network_ids[1], ConnectionId::from(1))
        .unwrap();

    // Elapse enough time for the peer monitor loop to garbage collect the peer
    wait_for_request_moderator_to_garbage_collect(
        unhealthy_peer_states.clone(),
        &time_service,
        &peer_network_ids[1],
    )
    .await;

    // Verify that no peer is being tracked
    assert!(unhealthy_peer_states.is_empty());

    // Reconnect the first peer
    peers_and_metadata
        .update_connection_state(peer_network_ids[0], ConnectionState::Connected)
        .unwrap();

    // Send an invalid request from the first peer
    send_invalid_transaction_request(
        highest_synced_version,
        &mut mock_client,
        peer_network_ids[0],
    )
    .await
    .unwrap_err();

    // Verify the peer is now tracked as unhealthy
    assert!(unhealthy_peer_states.contains_key(&peer_network_ids[0]));

    // Process enough invalid requests to ignore the third peer
    for _ in 0..max_invalid_requests_per_peer {
        send_invalid_transaction_request(
            highest_synced_version,
            &mut mock_client,
            peer_network_ids[2],
        )
        .await
        .unwrap_err();
    }

    // Verify the third peer is now tracked and blocked
    assert_eq!(unhealthy_peer_states.len(), 2);
    assert!(unhealthy_peer_states
        .get(&peer_network_ids[2])
        .unwrap()
        .is_ignored());

    // Disconnect the third peer
    peers_and_metadata
        .remove_peer_metadata(peer_network_ids[2], ConnectionId::from(2))
        .unwrap();

    // Elapse enough time for the peer monitor loop to garbage collect the peer
    wait_for_request_moderator_to_garbage_collect(
        unhealthy_peer_states.clone(),
        &time_service,
        &peer_network_ids[2],
    )
    .await;

    // Verify that the peer is no longer being tracked
    assert!(!unhealthy_peer_states.contains_key(&peer_network_ids[2]));
    assert_eq!(unhealthy_peer_states.len(), 1);
}

/// Advances the given timer by the amount of time it takes to refresh the moderator
async fn advance_moderator_refresh_time(mock_time: &MockTimeService) {
    let default_storage_config = StorageServiceConfig::default();
    let moderator_refresh_interval_ms =
        default_storage_config.request_moderator_refresh_interval_ms;
    mock_time
        .advance_ms_async(moderator_refresh_interval_ms)
        .await;
}

/// A simple utility function to create a new connection metadata for tests
fn create_connection_metadata(peer_id: AccountAddress, connection_id: u32) -> ConnectionMetadata {
    ConnectionMetadata::new(
        peer_id,
        ConnectionId::from(connection_id),
        NetworkAddress::from_str("/ip4/127.0.0.1/tcp/8081").unwrap(),
        ConnectionOrigin::Inbound,
        MessagingProtocolVersion::V1,
        ProtocolIdSet::empty(),
        PeerRole::Unknown,
    )
}

/// Sends a request to get a transaction list with proof at an invalid version
async fn send_invalid_transaction_request(
    highest_synced_version: u64,
    mock_client: &mut MockClient,
    peer_network_id: PeerNetworkId,
) -> Result<StorageServiceResponse, StorageServiceError> {
    // Create a data request for the missing transaction data
    let request = StorageServiceRequest::new(
        DataRequest::GetTransactionsWithProof(TransactionsWithProofRequest {
            proof_version: highest_synced_version + 1,
            start_version: highest_synced_version + 1,
            end_version: highest_synced_version + 2,
            include_events: false,
        }),
        true,
    );

    // Send the request and get the response
    let receiver = mock_client
        .send_request(
            request,
            Some(peer_network_id.peer_id()),
            Some(peer_network_id.network_id()),
        )
        .await;
    mock_client.wait_for_response(receiver).await
}

/// Waits for the request moderator to garbage collect the peer state
async fn wait_for_request_moderator_to_garbage_collect(
    unhealthy_peer_states: Arc<DashMap<PeerNetworkId, UnhealthyPeerState>>,
    mock_time: &MockTimeService,
    peer_network_id: &PeerNetworkId,
) {
    // Wait for the request moderator to garbage collect the peer state
    let garbage_collect = async move {
        loop {
            // Elapse enough time to force the moderator to refresh peer states
            advance_moderator_refresh_time(mock_time).await;

            // Check if the peer is still being tracked
            if !unhealthy_peer_states.contains_key(peer_network_id) {
                return; // The peer has been garbage collected
            }

            // Wait before retrying
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    };

    // Spawn the task with a timeout
    utils::spawn_with_timeout(
        garbage_collect,
        "Timed-out while waiting for the request moderator to perform garbage collection",
    )
    .await;
}

/// Waits for the request moderator to refresh the peer state
/// and stop ignoring the specified peer.
async fn wait_for_request_moderator_to_unblock_peer(
    unhealthy_peer_states: Arc<DashMap<PeerNetworkId, UnhealthyPeerState>>,
    mock_time: &MockTimeService,
    peer_network_id: &PeerNetworkId,
    min_time_to_ignore_peers_secs: u64,
) {
    // Wait for the request moderator to stop ignoring the specified peer
    let unblock_peer = async move {
        loop {
            // Elapse enough time to allow the peer to be unblocked
            mock_time
                .advance_secs_async(min_time_to_ignore_peers_secs)
                .await;

            // Elapse enough time to force the moderator to refresh peer states
            advance_moderator_refresh_time(mock_time).await;

            // Check if the peer is still being ignored
            let unhealthy_peer_state = unhealthy_peer_states.get(peer_network_id).unwrap();
            if !unhealthy_peer_state.is_ignored() {
                return; // The peer is no longer being ignored
            }

            // Wait before retrying
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    };

    // Spawn the task with a timeout
    utils::spawn_with_timeout(
        unblock_peer,
        "Timed-out while waiting for the request moderator to unblock the peer",
    )
    .await;
}

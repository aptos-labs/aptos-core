// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::{mock::MockClient, utils};
use aptos_config::{
    config::{PeerRole, StorageServiceConfig},
    network_id::{NetworkId, PeerNetworkId},
};
use aptos_netcore::transport::ConnectionOrigin;
use aptos_network::{
    application::metadata::ConnectionState,
    protocols::wire::handshake::v1::{MessagingProtocolVersion, ProtocolIdSet},
    transport::{ConnectionId, ConnectionMetadata},
};
use aptos_storage_service_types::{
    requests::{DataRequest, StorageServiceRequest},
    responses::StorageServiceResponse,
    StorageServiceError,
};
use aptos_types::{network_address::NetworkAddress, PeerId};
use claims::assert_matches;
use std::str::FromStr;

#[tokio::test]
async fn test_rate_limiter_disabled_by_default() {
    // Create a default storage service config (rate limiting disabled)
    let highest_synced_version = 100;
    let highest_synced_epoch = 10;
    let storage_service_config = StorageServiceConfig::default();
    assert!(storage_service_config
        .max_requests_per_second_per_peer
        .is_none());

    // Create the storage client and server
    let (mut mock_client, mut service, _, _, _) =
        MockClient::new(None, Some(storage_service_config));
    utils::update_storage_server_summary(
        &mut service,
        highest_synced_version,
        highest_synced_epoch,
    );

    // Verify the rate limit states are initially empty
    let peer_rate_limit_states = service.get_request_moderator().get_peer_rate_limit_states();
    assert!(peer_rate_limit_states.is_empty());

    // Spawn the server
    tokio::spawn(service.start());

    // Send many requests from a public peer and verify none are rate limited
    let peer_network_id = PeerNetworkId::new(NetworkId::Public, PeerId::random());
    for _ in 0..100 {
        let response = send_storage_server_summary_request(&mut mock_client, peer_network_id).await;
        assert!(response.is_ok());
    }

    // Verify no rate limit state was created
    assert!(peer_rate_limit_states.is_empty());
}

#[tokio::test]
async fn test_rate_limiter_enforced_for_public_peers() {
    // Create a storage service config with rate limiting enabled (1 req/sec)
    let max_requests_per_second = 1;
    let storage_service_config = StorageServiceConfig {
        max_requests_per_second_per_peer: Some(max_requests_per_second),
        ..Default::default()
    };

    // Create the storage client and server
    let (mut mock_client, service, _, _, _) = MockClient::new(None, Some(storage_service_config));

    // Spawn the server
    tokio::spawn(service.start());

    // The first request from a public peer should succeed
    let peer_network_id = PeerNetworkId::new(NetworkId::Public, PeerId::random());
    let response = send_storage_server_summary_request(&mut mock_client, peer_network_id).await;
    assert!(response.is_ok());

    // Subsequent requests in the same second should be rate limited
    let response = send_storage_server_summary_request(&mut mock_client, peer_network_id).await;
    assert_matches!(
        response.unwrap_err(),
        StorageServiceError::TooManyRequests(_)
    );
}

#[tokio::test]
async fn test_rate_limiter_not_enforced_for_non_public_peers() {
    // Create a storage service config with rate limiting enabled (1 req/sec)
    let storage_service_config = StorageServiceConfig {
        max_requests_per_second_per_peer: Some(1),
        ..Default::default()
    };

    // Create the storage client and server
    let (mut mock_client, service, _, _, _) = MockClient::new(None, Some(storage_service_config));

    // Spawn the server
    tokio::spawn(service.start());

    // Verify that validator and VFN peers are never rate limited regardless of request volume
    for network_id in [NetworkId::Validator, NetworkId::Vfn] {
        let peer_network_id = PeerNetworkId::new(network_id, PeerId::random());
        for _ in 0..10 {
            let response =
                send_storage_server_summary_request(&mut mock_client, peer_network_id).await;
            assert!(
                response.is_ok(),
                "Expected no rate limiting for {:?}",
                network_id
            );
        }
    }
}

#[tokio::test]
async fn test_rate_limiter_independent_per_peer() {
    // Create a storage service config with rate limiting enabled (2 reqs/sec)
    let storage_service_config = StorageServiceConfig {
        max_requests_per_second_per_peer: Some(2),
        ..Default::default()
    };

    // Create the storage client and server
    let (mut mock_client, service, _, _, _) = MockClient::new(None, Some(storage_service_config));

    // Spawn the server
    tokio::spawn(service.start());

    // Exhaust the token bucket for the first peer
    let peer_network_id_1 = PeerNetworkId::new(NetworkId::Public, PeerId::random());
    for _ in 0..2 {
        let response =
            send_storage_server_summary_request(&mut mock_client, peer_network_id_1).await;
        assert!(response.is_ok());
    }

    // The third request from peer 1 should be rate limited
    let response = send_storage_server_summary_request(&mut mock_client, peer_network_id_1).await;
    assert_matches!(
        response.unwrap_err(),
        StorageServiceError::TooManyRequests(_)
    );

    // A request from a different peer should succeed since rate limiting is per peer
    let peer_network_id_2 = PeerNetworkId::new(NetworkId::Public, PeerId::random());
    let response = send_storage_server_summary_request(&mut mock_client, peer_network_id_2).await;
    assert!(response.is_ok());

    // Verify that peer 1 is still rate limited
    let response = send_storage_server_summary_request(&mut mock_client, peer_network_id_1).await;
    assert_matches!(
        response.unwrap_err(),
        StorageServiceError::TooManyRequests(_)
    );

    // Verify that peer 2 is not rate limited
    let response = send_storage_server_summary_request(&mut mock_client, peer_network_id_2).await;
    assert!(response.is_ok());
}

#[tokio::test]
async fn test_rate_limiter_garbage_collect_disconnected_peers() {
    // Create a storage service config with rate limiting enabled
    let storage_service_config = StorageServiceConfig {
        max_requests_per_second_per_peer: Some(1),
        ..Default::default()
    };

    // Create the storage client and server
    let (mut mock_client, service, _, time_service, peers_and_metadata) =
        MockClient::new(None, Some(storage_service_config));

    // Register a public peer connection
    let peer_network_id = PeerNetworkId::new(NetworkId::Public, PeerId::random());
    let connection_metadata = ConnectionMetadata::new(
        peer_network_id.peer_id(),
        ConnectionId::from(0),
        NetworkAddress::from_str("/ip4/127.0.0.1/tcp/8081").unwrap(),
        ConnectionOrigin::Inbound,
        MessagingProtocolVersion::V1,
        ProtocolIdSet::empty(),
        PeerRole::Unknown,
    );
    peers_and_metadata
        .insert_connection_metadata(peer_network_id, connection_metadata)
        .unwrap();

    // Get the rate limit states
    let peer_rate_limit_states = service.get_request_moderator().get_peer_rate_limit_states();

    // Spawn the server
    tokio::spawn(service.start());

    // Send a request to populate the rate limit state for the peer
    send_storage_server_summary_request(&mut mock_client, peer_network_id)
        .await
        .ok();
    assert!(peer_rate_limit_states.contains_key(&peer_network_id));

    // Disconnect the peer and wait for the moderator to garbage collect
    peers_and_metadata
        .update_connection_state(peer_network_id, ConnectionState::Disconnecting)
        .unwrap();

    // Advance time to trigger the moderator refresh loop
    let default_config = StorageServiceConfig::default();
    loop {
        time_service
            .advance_ms_async(default_config.request_moderator_refresh_interval_ms)
            .await;
        if !peer_rate_limit_states.contains_key(&peer_network_id) {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    assert!(!peer_rate_limit_states.contains_key(&peer_network_id));
}

#[tokio::test]
async fn test_rate_limiter_refill_allows_new_requests() {
    // Create a storage service config with rate limiting enabled (2 reqs/sec)
    let max_requests_per_second = 2;
    let storage_service_config = StorageServiceConfig {
        max_requests_per_second_per_peer: Some(max_requests_per_second),
        ..Default::default()
    };

    // Create the storage client and server
    let (mut mock_client, service, _, _, _) = MockClient::new(None, Some(storage_service_config));

    // Get the rate limit states before spawning the server
    let peer_rate_limit_states = service.get_request_moderator().get_peer_rate_limit_states();

    // Spawn the server
    tokio::spawn(service.start());

    // Exhaust the token bucket for the peer
    let peer_network_id = PeerNetworkId::new(NetworkId::Public, PeerId::random());
    for _ in 0..max_requests_per_second {
        let response = send_storage_server_summary_request(&mut mock_client, peer_network_id).await;
        assert!(response.is_ok());
    }

    // Verify the peer is now rate limited
    let response = send_storage_server_summary_request(&mut mock_client, peer_network_id).await;
    assert_matches!(
        response.unwrap_err(),
        StorageServiceError::TooManyRequests(_)
    );

    // Manually return tokens to simulate the bucket refilling
    peer_rate_limit_states
        .get(&peer_network_id)
        .unwrap()
        .lock()
        .return_tokens(max_requests_per_second);

    // Verify the peer can now send requests again
    let response = send_storage_server_summary_request(&mut mock_client, peer_network_id).await;
    assert!(response.is_ok());
}

/// Sends a storage server summary request from the given peer and returns the response
async fn send_storage_server_summary_request(
    mock_client: &mut MockClient,
    peer_network_id: PeerNetworkId,
) -> Result<StorageServiceResponse, StorageServiceError> {
    let request = StorageServiceRequest::new(DataRequest::GetStorageServerSummary, false);
    let receiver = mock_client
        .send_request(
            request,
            Some(peer_network_id.peer_id()),
            Some(peer_network_id.network_id()),
        )
        .await;
    mock_client.wait_for_response(receiver).await
}

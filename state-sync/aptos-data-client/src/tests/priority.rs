// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::Error,
    tests::{mock::MockNetwork, utils},
};
use aptos_config::{
    config::{AptosDataClientConfig, BaseConfig, RoleType},
    network_id::NetworkId,
};
use aptos_storage_service_types::{
    requests::{
        DataRequest, NewTransactionOutputsWithProofRequest,
        NewTransactionsOrOutputsWithProofRequest, NewTransactionsWithProofRequest,
        StorageServiceRequest, SubscribeTransactionOutputsWithProofRequest,
        SubscribeTransactionsOrOutputsWithProofRequest, SubscribeTransactionsWithProofRequest,
        SubscriptionStreamMetadata, TransactionOutputsWithProofRequest,
    },
    responses::NUM_MICROSECONDS_IN_SECOND,
};
use aptos_time_service::TimeServiceTrait;
use claims::assert_matches;

#[tokio::test]
async fn all_peer_request_selection() {
    ::aptos_logger::Logger::init_for_testing();
    let (mut mock_network, _, client, _) = MockNetwork::new(None, None, None);

    // Ensure no peers can service the given request (we have no connections)
    let server_version_request =
        StorageServiceRequest::new(DataRequest::GetServerProtocolVersion, true);
    assert_matches!(
        client.choose_peer_for_request(&server_version_request),
        Err(Error::DataIsUnavailable(_))
    );

    // Add a regular peer and verify the peer is selected as the recipient
    let regular_peer_1 = mock_network.add_peer(false);
    assert_eq!(
        client.choose_peer_for_request(&server_version_request),
        Ok(regular_peer_1)
    );

    // Add two prioritized peers
    let priority_peer_1 = mock_network.add_peer(true);
    let priority_peer_2 = mock_network.add_peer(true);

    // Request data that is not being advertised and verify we get an error
    let output_data_request =
        DataRequest::GetTransactionOutputsWithProof(TransactionOutputsWithProofRequest {
            proof_version: 100,
            start_version: 0,
            end_version: 100,
        });
    let storage_request = StorageServiceRequest::new(output_data_request, false);
    assert_matches!(
        client.choose_peer_for_request(&storage_request),
        Err(Error::DataIsUnavailable(_))
    );

    // Advertise the data for the regular peer and verify it is now selected
    client.update_summary(regular_peer_1, utils::create_storage_summary(100));
    assert_eq!(
        client.choose_peer_for_request(&storage_request),
        Ok(regular_peer_1)
    );

    // Advertise the data for the priority peer and verify the priority peer is selected
    client.update_summary(priority_peer_2, utils::create_storage_summary(100));
    let peer_for_request = client.choose_peer_for_request(&storage_request).unwrap();
    assert_eq!(peer_for_request, priority_peer_2);

    // Reconnect priority peer 1 and remove the advertised data for priority peer 2
    mock_network.reconnect_peer(priority_peer_1);
    client.update_summary(priority_peer_2, utils::create_storage_summary(0));

    // Request the data again and verify the regular peer is chosen
    assert_eq!(
        client.choose_peer_for_request(&storage_request),
        Ok(regular_peer_1)
    );

    // Advertise the data for priority peer 1 and verify the priority peer is selected
    client.update_summary(priority_peer_1, utils::create_storage_summary(100));
    let peer_for_request = client.choose_peer_for_request(&storage_request).unwrap();
    assert_eq!(peer_for_request, priority_peer_1);

    // Advertise the data for priority peer 2 and verify either priority peer is selected
    client.update_summary(priority_peer_2, utils::create_storage_summary(100));
    let peer_for_request = client.choose_peer_for_request(&storage_request).unwrap();
    assert!(peer_for_request == priority_peer_1 || peer_for_request == priority_peer_2);
}

#[tokio::test]
async fn prioritized_peer_request_selection() {
    ::aptos_logger::Logger::init_for_testing();
    let (mut mock_network, _, client, _) = MockNetwork::new(None, None, None);

    // Ensure the properties hold for storage summary and version requests
    let storage_summary_request = DataRequest::GetStorageServerSummary;
    let get_version_request = DataRequest::GetServerProtocolVersion;
    for data_request in [storage_summary_request, get_version_request] {
        let storage_request = StorageServiceRequest::new(data_request, true);

        // Ensure no peers can service the request (we have no connections)
        assert_matches!(
            client.choose_peer_for_request(&storage_request),
            Err(Error::DataIsUnavailable(_))
        );

        // Add a regular peer and verify the peer is selected as the recipient
        let regular_peer_1 = mock_network.add_peer(false);
        assert_eq!(
            client.choose_peer_for_request(&storage_request),
            Ok(regular_peer_1)
        );

        // Add a priority peer and verify the peer is selected as the recipient
        let priority_peer_1 = mock_network.add_peer(true);
        assert_eq!(
            client.choose_peer_for_request(&storage_request),
            Ok(priority_peer_1)
        );

        // Disconnect the priority peer and verify the regular peer is now chosen
        mock_network.disconnect_peer(priority_peer_1);
        assert_eq!(
            client.choose_peer_for_request(&storage_request),
            Ok(regular_peer_1)
        );

        // Connect a new priority peer and verify it is now selected
        let priority_peer_2 = mock_network.add_peer(true);
        assert_eq!(
            client.choose_peer_for_request(&storage_request),
            Ok(priority_peer_2)
        );

        // Disconnect the priority peer and verify the regular peer is again chosen
        mock_network.disconnect_peer(priority_peer_2);
        assert_eq!(
            client.choose_peer_for_request(&storage_request),
            Ok(regular_peer_1)
        );

        // Disconnect the regular peer so that we no longer have any connections
        mock_network.disconnect_peer(regular_peer_1);
    }
}

#[tokio::test]
async fn prioritized_peer_optimistic_fetch_selection() {
    ::aptos_logger::Logger::init_for_testing();

    // Create a data client with a max lag of 100
    let max_optimistic_fetch_lag_secs = 100;
    let data_client_config = AptosDataClientConfig {
        max_optimistic_fetch_lag_secs,
        ..Default::default()
    };
    let (mut mock_network, time_service, client, _) =
        MockNetwork::new(None, Some(data_client_config), None);

    // Create test data
    let known_version = 10000000;
    let known_epoch = 10;

    // Ensure the properties hold for all optimistic fetch requests
    for data_request in enumerate_optimistic_fetch_requests(known_version, known_epoch) {
        let storage_request = StorageServiceRequest::new(data_request, true);

        // Ensure no peers can service the request (we have no connections)
        assert_matches!(
            client.choose_peer_for_request(&storage_request),
            Err(Error::DataIsUnavailable(_))
        );

        // Add a regular peer and verify the peer cannot support the request
        let regular_peer_1 = mock_network.add_peer(false);
        assert_matches!(
            client.choose_peer_for_request(&storage_request),
            Err(Error::DataIsUnavailable(_))
        );

        // Advertise the data for the regular peer and verify it is now selected
        let timestamp_usecs = time_service.now_unix_time().as_micros() as u64;
        client.update_summary(
            regular_peer_1,
            utils::create_storage_summary_with_timestamp(known_version, timestamp_usecs),
        );
        assert_eq!(
            client.choose_peer_for_request(&storage_request),
            Ok(regular_peer_1)
        );

        // Add a priority peer and verify the regular peer is still selected
        let priority_peer_1 = mock_network.add_peer(true);
        assert_eq!(
            client.choose_peer_for_request(&storage_request),
            Ok(regular_peer_1)
        );

        // Advertise the data for the priority peer and verify it is now selected
        client.update_summary(
            priority_peer_1,
            utils::create_storage_summary_with_timestamp(known_version, timestamp_usecs),
        );
        assert_eq!(
            client.choose_peer_for_request(&storage_request),
            Ok(priority_peer_1)
        );

        // Elapse enough time for both peers to be too far behind
        time_service
            .clone()
            .advance_secs(max_optimistic_fetch_lag_secs + 1);

        // Verify neither peer is now selected
        assert_matches!(
            client.choose_peer_for_request(&storage_request),
            Err(Error::DataIsUnavailable(_))
        );

        // Update the regular peer to be up-to-date and verify it is now chosen
        let timestamp_usecs = time_service.now_unix_time().as_micros() as u64;
        let regular_peer_timestamp_usecs =
            timestamp_usecs - ((max_optimistic_fetch_lag_secs / 2) * NUM_MICROSECONDS_IN_SECOND);
        client.update_summary(
            regular_peer_1,
            utils::create_storage_summary_with_timestamp(
                known_version,
                regular_peer_timestamp_usecs,
            ),
        );
        assert_eq!(
            client.choose_peer_for_request(&storage_request),
            Ok(regular_peer_1)
        );

        // Update the priority peer to be up-to-date and verify it is now chosen
        let priority_peer_timestamp_usecs =
            timestamp_usecs - ((max_optimistic_fetch_lag_secs / 2) * NUM_MICROSECONDS_IN_SECOND);
        client.update_summary(
            priority_peer_1,
            utils::create_storage_summary_with_timestamp(
                known_version,
                priority_peer_timestamp_usecs,
            ),
        );
        assert_eq!(
            client.choose_peer_for_request(&storage_request),
            Ok(priority_peer_1)
        );

        // Disconnect the priority peer and verify the regular peer is selected
        mock_network.disconnect_peer(priority_peer_1);
        assert_eq!(
            client.choose_peer_for_request(&storage_request),
            Ok(regular_peer_1)
        );

        // Elapse enough time for the regular peer to be too far behind
        time_service
            .clone()
            .advance_secs(max_optimistic_fetch_lag_secs);

        // Verify neither peer is now select
        assert_matches!(
            client.choose_peer_for_request(&storage_request),
            Err(Error::DataIsUnavailable(_))
        );

        // Disconnect the regular peer so that we no longer have any connections
        mock_network.disconnect_peer(regular_peer_1);
    }
}

#[tokio::test]
async fn prioritized_peer_subscription_requests() {
    ::aptos_logger::Logger::init_for_testing();

    // Create a data client with a max lag of 10
    let max_subscription_lag_secs = 10;
    let data_client_config = AptosDataClientConfig {
        max_subscription_lag_secs,
        ..Default::default()
    };
    let (mut mock_network, time_service, client, _) =
        MockNetwork::new(None, Some(data_client_config), None);

    // Create test data
    let known_version = 1000;
    let known_epoch = 5;

    // Ensure the properties hold for all subscription requests
    for data_request in enumerate_subscription_requests(known_version, known_epoch) {
        let storage_request = StorageServiceRequest::new(data_request, true);

        // Ensure no peers can service the request (we have no connections)
        assert_matches!(
            client.choose_peer_for_request(&storage_request),
            Err(Error::DataIsUnavailable(_))
        );

        // Add two priority peers and a regular peer
        let priority_peer_1 = mock_network.add_peer(true);
        let priority_peer_2 = mock_network.add_peer(true);
        let regular_peer_1 = mock_network.add_peer(false);

        // Verify no peers can service the request (no peers are advertising data)
        assert_matches!(
            client.choose_peer_for_request(&storage_request),
            Err(Error::DataIsUnavailable(_))
        );

        // Advertise the data for all peers
        let timestamp_usecs = time_service.now_unix_time().as_micros() as u64;
        for peer in [priority_peer_1, priority_peer_2, regular_peer_1] {
            client.update_summary(
                peer,
                utils::create_storage_summary_with_timestamp(known_version, timestamp_usecs),
            );
        }

        // Verify a priority peer is selected
        let selected_peer = client.choose_peer_for_request(&storage_request).unwrap();
        assert!(selected_peer == priority_peer_1 || selected_peer == priority_peer_2);

        // Make several more requests and verify the same priority peer is selected
        for _ in 0..10 {
            let current_selected_peer = client.choose_peer_for_request(&storage_request).unwrap();
            assert_eq!(selected_peer, current_selected_peer);
        }

        // Elapse enough time for all peers to be too far behind
        time_service
            .clone()
            .advance_secs(max_subscription_lag_secs + 1);

        // Advertise new data for all peers (except the selected peer)
        let timestamp_usecs = time_service.now_unix_time().as_micros() as u64;
        for peer in [priority_peer_1, priority_peer_2, regular_peer_1] {
            if peer != selected_peer {
                client.update_summary(
                    peer,
                    utils::create_storage_summary_with_timestamp(known_version, timestamp_usecs),
                );
            }
        }

        // Verify no peers can service the request (because the
        // previously selected peer is still too far behind).
        assert_matches!(
            client.choose_peer_for_request(&storage_request),
            Err(Error::DataIsUnavailable(_))
        );

        // Verify the other priority peer is now select (as the
        // previous request will terminate the subscription).
        let next_selected_peer = client.choose_peer_for_request(&storage_request).unwrap();
        assert!(selected_peer != next_selected_peer);
        assert!(selected_peer == priority_peer_1 || selected_peer == priority_peer_2);

        // Update the request's subscription ID and verify the other priority peer is selected
        let storage_request = update_subscription_request_id(&storage_request);
        let next_selected_peer = client.choose_peer_for_request(&storage_request).unwrap();
        assert!(selected_peer != next_selected_peer);
        assert!(next_selected_peer == priority_peer_1 || next_selected_peer == priority_peer_2);

        // Make several more requests and verify the same priority peer is selected
        for _ in 0..10 {
            let current_select_peer = client.choose_peer_for_request(&storage_request).unwrap();
            assert_eq!(current_select_peer, next_selected_peer);
        }

        // Disconnect all peers and verify no peers can service the request
        for peer in [priority_peer_1, priority_peer_2, regular_peer_1] {
            mock_network.disconnect_peer(peer);
        }
        assert_matches!(
            client.choose_peer_for_request(&storage_request),
            Err(Error::DataIsUnavailable(_))
        );
    }
}

#[tokio::test]
async fn prioritized_peer_subscription_selection() {
    ::aptos_logger::Logger::init_for_testing();

    // Create a data client with a max lag of 100
    let max_subscription_lag_secs = 100;
    let data_client_config = AptosDataClientConfig {
        max_subscription_lag_secs,
        ..Default::default()
    };
    let (mut mock_network, time_service, client, _) =
        MockNetwork::new(None, Some(data_client_config), None);

    // Create test data
    let known_version = 10000000;
    let known_epoch = 10;

    // Ensure the properties hold for all subscription requests
    for data_request in enumerate_subscription_requests(known_version, known_epoch) {
        let storage_request = StorageServiceRequest::new(data_request, true);

        // Ensure no peers can service the request (we have no connections)
        assert_matches!(
            client.choose_peer_for_request(&storage_request),
            Err(Error::DataIsUnavailable(_))
        );

        // Add a regular peer and verify the peer cannot support the request
        let regular_peer_1 = mock_network.add_peer(false);
        assert_matches!(
            client.choose_peer_for_request(&storage_request),
            Err(Error::DataIsUnavailable(_))
        );

        // Advertise the data for the regular peer and verify it is now selected
        let timestamp_usecs = time_service.now_unix_time().as_micros() as u64;
        client.update_summary(
            regular_peer_1,
            utils::create_storage_summary_with_timestamp(known_version, timestamp_usecs),
        );
        assert_eq!(
            client.choose_peer_for_request(&storage_request),
            Ok(regular_peer_1)
        );

        // Add a priority peer and verify the regular peer is still selected
        let priority_peer_1 = mock_network.add_peer(true);
        assert_eq!(
            client.choose_peer_for_request(&storage_request),
            Ok(regular_peer_1)
        );

        // Advertise the data for the priority peer and verify it is not selected
        // (the previous subscription request went to the regular peer).
        client.update_summary(
            priority_peer_1,
            utils::create_storage_summary_with_timestamp(known_version, timestamp_usecs),
        );
        assert_matches!(
            client.choose_peer_for_request(&storage_request),
            Err(Error::DataIsUnavailable(_))
        );

        // Update the request's subscription ID and verify it now goes to the priority peer
        let storage_request = update_subscription_request_id(&storage_request);
        assert_eq!(
            client.choose_peer_for_request(&storage_request),
            Ok(priority_peer_1)
        );

        // Elapse enough time for both peers to be too far behind
        time_service
            .clone()
            .advance_secs(max_subscription_lag_secs + 1);

        // Verify neither peer is now selected
        assert_matches!(
            client.choose_peer_for_request(&storage_request),
            Err(Error::DataIsUnavailable(_))
        );

        // Update the request's subscription ID
        let storage_request = update_subscription_request_id(&storage_request);

        // Update the regular peer to be up-to-date and verify it is now chosen
        let timestamp_usecs = time_service.now_unix_time().as_micros() as u64;
        let regular_peer_timestamp_usecs =
            timestamp_usecs - ((max_subscription_lag_secs / 2) * NUM_MICROSECONDS_IN_SECOND);
        client.update_summary(
            regular_peer_1,
            utils::create_storage_summary_with_timestamp(
                known_version,
                regular_peer_timestamp_usecs,
            ),
        );
        assert_eq!(
            client.choose_peer_for_request(&storage_request),
            Ok(regular_peer_1)
        );

        // Update the request's subscription ID
        let storage_request = update_subscription_request_id(&storage_request);

        // Update the priority peer to be up-to-date and verify it is now chosen
        let priority_peer_timestamp_usecs =
            timestamp_usecs - ((max_subscription_lag_secs / 2) * NUM_MICROSECONDS_IN_SECOND);
        client.update_summary(
            priority_peer_1,
            utils::create_storage_summary_with_timestamp(
                known_version,
                priority_peer_timestamp_usecs,
            ),
        );
        assert_eq!(
            client.choose_peer_for_request(&storage_request),
            Ok(priority_peer_1)
        );

        // Update the request's subscription ID
        let storage_request = update_subscription_request_id(&storage_request);

        // Disconnect the priority peer and verify the regular peer is selected
        mock_network.disconnect_peer(priority_peer_1);
        assert_eq!(
            client.choose_peer_for_request(&storage_request),
            Ok(regular_peer_1)
        );

        // Elapse enough time for the regular peer to be too far behind
        time_service.clone().advance_secs(max_subscription_lag_secs);

        // Verify neither peer is now select
        assert_matches!(
            client.choose_peer_for_request(&storage_request),
            Err(Error::DataIsUnavailable(_))
        );

        // Disconnect the regular peer so that we no longer have any connections
        mock_network.disconnect_peer(regular_peer_1);
    }
}

#[tokio::test]
async fn validator_peer_prioritization() {
    ::aptos_logger::Logger::init_for_testing();

    // Create a validator node
    let base_config = BaseConfig {
        role: RoleType::Validator,
        ..Default::default()
    };
    let (mut mock_network, _, client, _) = MockNetwork::new(Some(base_config), None, None);

    // Add a validator peer and ensure it's prioritized
    let validator_peer = mock_network.add_peer_with_network_id(NetworkId::Validator, false);
    let (priority_peers, regular_peers) = client.get_priority_and_regular_peers().unwrap();
    assert_eq!(priority_peers, vec![validator_peer]);
    assert_eq!(regular_peers, vec![]);

    // Add a vfn peer and ensure it's not prioritized
    let vfn_peer = mock_network.add_peer_with_network_id(NetworkId::Vfn, true);
    let (priority_peers, regular_peers) = client.get_priority_and_regular_peers().unwrap();
    assert_eq!(priority_peers, vec![validator_peer]);
    assert_eq!(regular_peers, vec![vfn_peer]);
}

#[tokio::test]
async fn vfn_peer_prioritization() {
    ::aptos_logger::Logger::init_for_testing();

    // Create a validator fullnode
    let base_config = BaseConfig {
        role: RoleType::FullNode,
        ..Default::default()
    };
    let (mut mock_network, _, client, _) = MockNetwork::new(Some(base_config), None, None);

    // Add a validator peer and ensure it's prioritized
    let validator_peer = mock_network.add_peer_with_network_id(NetworkId::Vfn, false);
    let (priority_peers, regular_peers) = client.get_priority_and_regular_peers().unwrap();
    assert_eq!(priority_peers, vec![validator_peer]);
    assert_eq!(regular_peers, vec![]);

    // Add a pfn peer and ensure it's not prioritized
    let pfn_peer = mock_network.add_peer_with_network_id(NetworkId::Public, true);
    let (priority_peers, regular_peers) = client.get_priority_and_regular_peers().unwrap();
    assert_eq!(priority_peers, vec![validator_peer]);
    assert_eq!(regular_peers, vec![pfn_peer]);
}

#[tokio::test]
async fn pfn_peer_prioritization() {
    ::aptos_logger::Logger::init_for_testing();

    // Create a public fullnode
    let base_config = BaseConfig {
        role: RoleType::FullNode,
        ..Default::default()
    };
    let (mut mock_network, _, client, _) =
        MockNetwork::new(Some(base_config), None, Some(vec![NetworkId::Public]));

    // Add an inbound pfn peer and ensure it's not prioritized
    let inbound_peer = mock_network.add_peer_with_network_id(NetworkId::Public, false);
    let (priority_peers, regular_peers) = client.get_priority_and_regular_peers().unwrap();
    assert_eq!(priority_peers, vec![]);
    assert_eq!(regular_peers, vec![inbound_peer]);

    // Add an outbound pfn peer and ensure it's prioritized
    let outbound_peer = mock_network.add_peer_with_network_id(NetworkId::Public, true);
    let (priority_peers, regular_peers) = client.get_priority_and_regular_peers().unwrap();
    assert_eq!(priority_peers, vec![outbound_peer]);
    assert_eq!(regular_peers, vec![inbound_peer]);
}

/// Enumerates all optimistic fetch request types
fn enumerate_optimistic_fetch_requests(known_version: u64, known_epoch: u64) -> Vec<DataRequest> {
    // Create all optimistic fetch requests
    let new_transactions_request =
        DataRequest::GetNewTransactionsWithProof(NewTransactionsWithProofRequest {
            known_version,
            known_epoch,
            include_events: false,
        });
    let new_outputs_requests =
        DataRequest::GetNewTransactionOutputsWithProof(NewTransactionOutputsWithProofRequest {
            known_version,
            known_epoch,
        });
    let new_transactions_or_outputs_request = DataRequest::GetNewTransactionsOrOutputsWithProof(
        NewTransactionsOrOutputsWithProofRequest {
            known_version,
            known_epoch,
            include_events: false,
            max_num_output_reductions: 0,
        },
    );

    // Return all optimistic fetch requests
    vec![
        new_transactions_request,
        new_outputs_requests,
        new_transactions_or_outputs_request,
    ]
}

/// Enumerates all subscription request types
fn enumerate_subscription_requests(known_version: u64, known_epoch: u64) -> Vec<DataRequest> {
    // Create all subscription requests
    let subscribe_transactions_request =
        DataRequest::SubscribeTransactionsWithProof(SubscribeTransactionsWithProofRequest {
            subscription_stream_metadata: SubscriptionStreamMetadata {
                known_version_at_stream_start: known_version,
                known_epoch_at_stream_start: known_epoch,
                subscription_stream_id: 100,
            },
            subscription_stream_index: 0,
            include_events: false,
        });
    let subscribe_outputs_request = DataRequest::SubscribeTransactionOutputsWithProof(
        SubscribeTransactionOutputsWithProofRequest {
            subscription_stream_metadata: SubscriptionStreamMetadata {
                known_version_at_stream_start: known_version,
                known_epoch_at_stream_start: known_epoch,
                subscription_stream_id: 200,
            },
            subscription_stream_index: 0,
        },
    );
    let subscribe_transactions_or_outputs_request =
        DataRequest::SubscribeTransactionsOrOutputsWithProof(
            SubscribeTransactionsOrOutputsWithProofRequest {
                subscription_stream_metadata: SubscriptionStreamMetadata {
                    known_version_at_stream_start: known_version,
                    known_epoch_at_stream_start: known_epoch,
                    subscription_stream_id: 300,
                },
                subscription_stream_index: 0,
                include_events: false,
                max_num_output_reductions: 0,
            },
        );

    // Return all subscription requests
    vec![
        subscribe_transactions_request,
        subscribe_outputs_request,
        subscribe_transactions_or_outputs_request,
    ]
}

/// Updates the subscription request ID in the given storage request
/// and returns the updated storage request.
fn update_subscription_request_id(
    storage_service_request: &StorageServiceRequest,
) -> StorageServiceRequest {
    let mut storage_service_request = storage_service_request.clone();

    // Update the subscription's request ID
    match &mut storage_service_request.data_request {
        DataRequest::SubscribeTransactionsWithProof(request) => {
            request.subscription_stream_metadata.subscription_stream_id += 1
        },
        DataRequest::SubscribeTransactionOutputsWithProof(request) => {
            request.subscription_stream_metadata.subscription_stream_id += 1
        },
        DataRequest::SubscribeTransactionsOrOutputsWithProof(request) => {
            request.subscription_stream_metadata.subscription_stream_id += 1
        },
        _ => panic!(
            "Unexpected subscription request type! {:?}",
            storage_service_request
        ),
    }

    storage_service_request
}

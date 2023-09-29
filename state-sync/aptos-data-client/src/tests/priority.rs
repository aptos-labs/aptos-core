// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    client::AptosDataClient,
    error::Error,
    tests::{mock::MockNetwork, utils},
};
use aptos_config::{
    config::{AptosDataClientConfig, BaseConfig, RoleType},
    network_id::{NetworkId, PeerNetworkId},
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
use maplit::hashset;
use ordered_float::OrderedFloat;
use std::collections::HashMap;

#[tokio::test]
async fn all_peer_request_selection() {
    // Create the mock network and client
    let (mut mock_network, _, client, _) = MockNetwork::new(None, None, None);

    // Ensure no peers can service the given request (we have no connections)
    let server_version_request =
        StorageServiceRequest::new(DataRequest::GetServerProtocolVersion, true);
    verify_request_is_unserviceable(&client, &server_version_request);

    // Add a regular peer and verify the peer is selected as the recipient
    let regular_peer_1 = mock_network.add_peer(false);
    verify_peer_selected_for_request(&client, regular_peer_1, &server_version_request);

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
    verify_request_is_unserviceable(&client, &storage_request);

    // Advertise the data for the regular peer and verify it is now selected
    client.update_peer_storage_summary(regular_peer_1, utils::create_storage_summary(100));
    verify_peer_selected_for_request(&client, regular_peer_1, &storage_request);

    // Advertise the data for the priority peer and verify the priority peer is selected
    client.update_peer_storage_summary(priority_peer_2, utils::create_storage_summary(100));
    let peer_for_request = client.choose_peer_for_request(&storage_request).unwrap();
    assert_eq!(peer_for_request, priority_peer_2);

    // Reconnect priority peer 1 and remove the advertised data for priority peer 2
    mock_network.reconnect_peer(priority_peer_1);
    client.update_peer_storage_summary(priority_peer_2, utils::create_storage_summary(0));

    // Request the data again and verify the regular peer is chosen
    verify_peer_selected_for_request(&client, regular_peer_1, &storage_request);

    // Advertise the data for priority peer 1 and verify the priority peer is selected
    client.update_peer_storage_summary(priority_peer_1, utils::create_storage_summary(100));
    let peer_for_request = client.choose_peer_for_request(&storage_request).unwrap();
    assert_eq!(peer_for_request, priority_peer_1);

    // Advertise the data for priority peer 2 and verify either priority peer is selected
    client.update_peer_storage_summary(priority_peer_2, utils::create_storage_summary(100));
    let peer_for_request = client.choose_peer_for_request(&storage_request).unwrap();
    assert!(peer_for_request == priority_peer_1 || peer_for_request == priority_peer_2);
}

#[tokio::test]
async fn prioritized_peer_request_selection() {
    // Create the mock network and client
    let (mut mock_network, _, client, _) = MockNetwork::new(None, None, None);

    // Ensure the properties hold for storage summary and version requests
    let storage_summary_request = DataRequest::GetStorageServerSummary;
    let get_version_request = DataRequest::GetServerProtocolVersion;
    for data_request in [storage_summary_request, get_version_request] {
        let storage_request = StorageServiceRequest::new(data_request, true);

        // Ensure no peers can service the request (we have no connections)
        verify_request_is_unserviceable(&client, &storage_request);

        // Add a regular peer and verify the peer is selected as the recipient
        let regular_peer_1 = mock_network.add_peer(false);
        verify_peer_selected_for_request(&client, regular_peer_1, &storage_request);

        // Add a priority peer and verify the peer is selected as the recipient
        let priority_peer_1 = mock_network.add_peer(true);
        verify_peer_selected_for_request(&client, priority_peer_1, &storage_request);

        // Disconnect the priority peer and verify the regular peer is now chosen
        mock_network.disconnect_peer(priority_peer_1);
        verify_peer_selected_for_request(&client, regular_peer_1, &storage_request);

        // Connect a new priority peer and verify it is now selected
        let priority_peer_2 = mock_network.add_peer(true);
        verify_peer_selected_for_request(&client, priority_peer_2, &storage_request);

        // Disconnect the priority peer and verify the regular peer is again chosen
        mock_network.disconnect_peer(priority_peer_2);
        verify_peer_selected_for_request(&client, regular_peer_1, &storage_request);

        // Disconnect the regular peer so that we no longer have any connections
        mock_network.disconnect_peer(regular_peer_1);
    }
}

#[tokio::test]
async fn prioritized_peer_request_latency_filtering() {
    // Create the data client config with latency filtering configurations
    let min_peers_for_latency_filtering = 100;
    let latency_filtering_reduction_factor = 2;
    let data_client_config = AptosDataClientConfig {
        min_peers_for_latency_filtering,
        min_peer_ratio_for_latency_filtering: 2,
        latency_filtering_reduction_factor,
        ..Default::default()
    };

    // Ensure the properties hold for both priority and non-priority peers
    for poll_priority_peers in [true, false] {
        // Create the mock network and client
        let (mut mock_network, _, client, _) =
            MockNetwork::new(None, Some(data_client_config), None);

        // Create the data request
        let data_request = DataRequest::GetStorageServerSummary;
        let storage_request = StorageServiceRequest::new(data_request, true);

        // Add several peers (enough to trigger latency filtering)
        let num_peers = min_peers_for_latency_filtering + 10;
        let mut peers = vec![];
        for _ in 0..num_peers {
            let peer = mock_network.add_peer(poll_priority_peers);
            peers.push(peer);
        }

        // Select a peer to service the request multiple times
        let mut peers_and_selection_counts = HashMap::new();
        for _ in 0..20_000 {
            // Select a peer to service the request
            let selected_peer = client.choose_peer_for_request(&storage_request).unwrap();

            // Update the peer selection counts
            *peers_and_selection_counts.entry(selected_peer).or_insert(0) += 1;
        }

        // Verify the highest selected peers are the lowest latency peers
        utils::verify_highest_peer_selection_latencies(
            &mut mock_network,
            &mut peers_and_selection_counts,
        );

        // Build a list of all peers sorted by their latencies
        let mut peers_and_latencies = vec![];
        for peer in peers_and_selection_counts.keys() {
            // Get the peer's ping latency
            let ping_latency = utils::get_peer_ping_latency(&mut mock_network, *peer);

            // Add the peer and latency to the list
            peers_and_latencies.push((*peer, OrderedFloat(ping_latency)));
        }
        peers_and_latencies.sort_by_key(|(_, latency)| *latency);

        // Verify that the top subset of peers have selection counts
        let peers_to_verify = (num_peers / latency_filtering_reduction_factor) as usize;
        for (peer, _) in peers_and_latencies[0..peers_to_verify].iter() {
            match peers_and_selection_counts.get(peer) {
                Some(selection_count) => assert!(*selection_count > 0),
                None => panic!("Peer {:?} was not found in the selection counts!", peer),
            }
        }

        // Verify that the bottom subset of peers do not have selection counts
        // (as they were filtered out).
        for (peer, _) in peers_and_latencies[peers_to_verify..].iter() {
            if let Some(selection_count) = peers_and_selection_counts.get(peer) {
                assert_eq!(*selection_count, 0);
            }
        }
    }
}

#[tokio::test]
async fn prioritized_peer_request_latency_filtering_ratio() {
    // Create the data client config with latency filtering configurations
    let min_peers_for_latency_filtering = 50;
    let min_peer_ratio_for_latency_filtering = 10_000; // Set to a very high value
    let latency_filtering_reduction_factor = 2;
    let data_client_config = AptosDataClientConfig {
        min_peers_for_latency_filtering,
        min_peer_ratio_for_latency_filtering,
        latency_filtering_reduction_factor,
        ..Default::default()
    };

    // Ensure the properties hold for both priority and non-priority peers
    for poll_priority_peers in [true, false] {
        // Create the mock network and client
        let (mut mock_network, _, client, _) =
            MockNetwork::new(None, Some(data_client_config), None);

        // Create the data request
        let data_request = DataRequest::GetStorageServerSummary;
        let storage_request = StorageServiceRequest::new(data_request, true);

        // Add several peers (enough to satisfy the minimum number of peers)
        let num_peers = min_peers_for_latency_filtering * 2;
        let mut peers = vec![];
        for _ in 0..num_peers {
            let peer = mock_network.add_peer(poll_priority_peers);
            peers.push(peer);
        }

        // Select a peer to service the request multiple times
        let mut peers_and_selection_counts = HashMap::new();
        for _ in 0..20_000 {
            // Select a peer to service the request
            let selected_peer = client.choose_peer_for_request(&storage_request).unwrap();

            // Update the peer selection counts
            *peers_and_selection_counts.entry(selected_peer).or_insert(0) += 1;
        }

        // Verify the highest selected peers are the lowest latency peers
        utils::verify_highest_peer_selection_latencies(
            &mut mock_network,
            &mut peers_and_selection_counts,
        );

        // Verify that the number of selected peers is more than
        // half the total peers (as filtering was disabled).
        let num_filtered_peers = (num_peers / latency_filtering_reduction_factor) as usize;
        assert!(peers_and_selection_counts.len() > num_filtered_peers);
    }
}

#[tokio::test]
async fn prioritized_peer_request_latency_selection() {
    // Create the data client config with latency filtering configurations
    let min_peers_for_latency_filtering = 50;
    let latency_filtering_reduction_factor = 2;
    let data_client_config = AptosDataClientConfig {
        min_peers_for_latency_filtering,
        latency_filtering_reduction_factor,
        ..Default::default()
    };

    // Ensure the properties hold for both priority and non-priority peers
    for poll_priority_peers in [true, false] {
        // Create the mock network and client
        let (mut mock_network, _, client, _) =
            MockNetwork::new(None, Some(data_client_config), None);

        // Create the data request
        let data_request = DataRequest::GetStorageServerSummary;
        let storage_request = StorageServiceRequest::new(data_request, true);

        // Add several peers (but not enough to trigger latency filtering)
        let num_peers = min_peers_for_latency_filtering - 1;
        let mut peers = vec![];
        for _ in 0..num_peers {
            let peer = mock_network.add_peer(poll_priority_peers);
            peers.push(peer);
        }

        // Select a peer to service the request multiple times
        let mut peers_and_selection_counts = HashMap::new();
        for _ in 0..20_000 {
            // Select a peer to service the request
            let selected_peer = client.choose_peer_for_request(&storage_request).unwrap();

            // Update the peer selection counts
            *peers_and_selection_counts.entry(selected_peer).or_insert(0) += 1;
        }

        // Verify the highest selected peers are the lowest latency peers
        utils::verify_highest_peer_selection_latencies(
            &mut mock_network,
            &mut peers_and_selection_counts,
        );

        // Verify that the number of selected peers is more than
        // half the total peers (as filtering was disabled).
        let num_filtered_peers = (num_peers / latency_filtering_reduction_factor) as usize;
        assert!(peers_and_selection_counts.len() > num_filtered_peers);
    }
}

#[tokio::test]
async fn prioritized_peer_request_missing_latencies() {
    // Create the data client config with latency filtering configurations
    let min_peers_for_latency_filtering = 50;
    let data_client_config = AptosDataClientConfig {
        min_peers_for_latency_filtering,
        ..Default::default()
    };

    // Ensure the properties hold for both priority and non-priority peers
    for poll_priority_peers in [true, false] {
        // Create the mock network and client
        let (mut mock_network, _, client, _) =
            MockNetwork::new(None, Some(data_client_config), None);

        // Create the data request
        let data_request = DataRequest::GetStorageServerSummary;
        let storage_request = StorageServiceRequest::new(data_request, true);

        // Add several peers
        let num_peers = min_peers_for_latency_filtering + 10;
        let mut peers = vec![];
        for _ in 0..num_peers {
            let peer = mock_network.add_peer(poll_priority_peers);
            peers.push(peer);
        }

        // Remove the latency metadata for some peers
        let num_peers_with_missing_latencies = (min_peers_for_latency_filtering / 3) as usize;
        let mut peers_with_missing_latencies = vec![];
        for peer in peers[0..num_peers_with_missing_latencies].iter() {
            // Remove the latency metadata for the peer
            utils::remove_latency_metadata(&client, *peer);

            // Add the peer to the set of peers with missing latencies
            peers_with_missing_latencies.push(*peer);
        }

        // Select a peer to service the request multiple times
        let mut peers_and_selection_counts = HashMap::new();
        for _ in 0..20_000 {
            // Select a peer to service the request
            let selected_peer = client.choose_peer_for_request(&storage_request).unwrap();

            // Update the peer selection counts
            *peers_and_selection_counts.entry(selected_peer).or_insert(0) += 1;
        }

        // Verify the highest selected peers are the lowest latency peers
        utils::verify_highest_peer_selection_latencies(
            &mut mock_network,
            &mut peers_and_selection_counts,
        );

        // Verify that the peers with missing latencies are not selected
        for peer in peers_with_missing_latencies {
            if let Some(selection_count) = peers_and_selection_counts.get(&peer) {
                assert_eq!(*selection_count, 0);
            }
        }
    }
}

#[tokio::test]
async fn prioritized_peer_request_no_latencies() {
    // Create the data client config with latency filtering configurations
    let min_peers_for_latency_filtering = 50;
    let data_client_config = AptosDataClientConfig {
        min_peers_for_latency_filtering,
        ..Default::default()
    };

    // Ensure the properties hold for both priority and non-priority peers
    for poll_priority_peers in [true, false] {
        // Create the mock network and client
        let (mut mock_network, _, client, _) =
            MockNetwork::new(None, Some(data_client_config), None);

        // Create the data request
        let data_request = DataRequest::GetStorageServerSummary;
        let storage_request = StorageServiceRequest::new(data_request, true);

        // Add several peers and remove their latency metadata
        let num_peers = min_peers_for_latency_filtering + 10;
        let mut peers = vec![];
        for _ in 0..num_peers {
            // Add a peer
            let peer = mock_network.add_peer(poll_priority_peers);
            peers.push(peer);

            // Remove the latency metadata for the peer
            utils::remove_latency_metadata(&client, peer)
        }

        // Select a peer to service the request multiple times
        let mut peers_and_selection_counts = HashMap::new();
        for _ in 0..20_000 {
            // Select a peer to service the request
            let selected_peer = client.choose_peer_for_request(&storage_request).unwrap();

            // Update the peer selection counts
            *peers_and_selection_counts.entry(selected_peer).or_insert(0) += 1;
        }

        // Verify that peers are still selected even though there are no recorded latencies
        for peer in peers {
            match peers_and_selection_counts.get(&peer) {
                Some(selection_count) => assert!(*selection_count > 0),
                None => panic!("Peer {:?} was not found in the selection counts!", peer),
            }
        }
    }
}

#[tokio::test]
async fn prioritized_peer_optimistic_fetch_selection() {
    // Create a data client with a max lag of 100
    let max_optimistic_fetch_lag_secs = 100;
    let data_client_config = AptosDataClientConfig {
        max_optimistic_fetch_lag_secs,
        ..Default::default()
    };

    // Create the mock network, time service and client
    let (mut mock_network, time_service, client, _) =
        MockNetwork::new(None, Some(data_client_config), None);

    // Create test data
    let known_version = 10000000;
    let known_epoch = 10;

    // Ensure the properties hold for all optimistic fetch requests
    for data_request in enumerate_optimistic_fetch_requests(known_version, known_epoch) {
        let storage_request = StorageServiceRequest::new(data_request, true);

        // Ensure no peers can service the request (we have no connections)
        verify_request_is_unserviceable(&client, &storage_request);

        // Add a regular peer and verify the peer cannot service the request
        let regular_peer_1 = mock_network.add_peer(false);
        verify_request_is_unserviceable(&client, &storage_request);

        // Advertise the data for the regular peer and verify it is now selected
        let timestamp_usecs = time_service.now_unix_time().as_micros() as u64;
        client.update_peer_storage_summary(
            regular_peer_1,
            utils::create_storage_summary_with_timestamp(known_version, timestamp_usecs),
        );
        verify_peer_selected_for_request(&client, regular_peer_1, &storage_request);

        // Add a priority peer and verify the regular peer is still selected
        let priority_peer_1 = mock_network.add_peer(true);
        verify_peer_selected_for_request(&client, regular_peer_1, &storage_request);

        // Advertise the data for the priority peer and verify it is now selected
        client.update_peer_storage_summary(
            priority_peer_1,
            utils::create_storage_summary_with_timestamp(known_version, timestamp_usecs),
        );
        verify_peer_selected_for_request(&client, priority_peer_1, &storage_request);

        // Elapse enough time for both peers to be too far behind
        time_service
            .clone()
            .advance_secs(max_optimistic_fetch_lag_secs + 1);

        // Verify neither peer is now selected
        verify_request_is_unserviceable(&client, &storage_request);

        // Update the regular peer to be up-to-date and verify it is now chosen
        let timestamp_usecs = time_service.now_unix_time().as_micros() as u64;
        let regular_peer_timestamp_usecs =
            timestamp_usecs - ((max_optimistic_fetch_lag_secs / 2) * NUM_MICROSECONDS_IN_SECOND);
        client.update_peer_storage_summary(
            regular_peer_1,
            utils::create_storage_summary_with_timestamp(
                known_version,
                regular_peer_timestamp_usecs,
            ),
        );
        verify_peer_selected_for_request(&client, regular_peer_1, &storage_request);

        // Update the priority peer to be up-to-date and verify it is now chosen
        let priority_peer_timestamp_usecs =
            timestamp_usecs - ((max_optimistic_fetch_lag_secs / 2) * NUM_MICROSECONDS_IN_SECOND);
        client.update_peer_storage_summary(
            priority_peer_1,
            utils::create_storage_summary_with_timestamp(
                known_version,
                priority_peer_timestamp_usecs,
            ),
        );
        verify_peer_selected_for_request(&client, priority_peer_1, &storage_request);

        // Disconnect the priority peer and verify the regular peer is selected
        mock_network.disconnect_peer(priority_peer_1);
        verify_peer_selected_for_request(&client, regular_peer_1, &storage_request);

        // Elapse enough time for the regular peer to be too far behind
        time_service
            .clone()
            .advance_secs(max_optimistic_fetch_lag_secs);

        // Verify neither peer is now select
        verify_request_is_unserviceable(&client, &storage_request);

        // Disconnect the regular peer so that we no longer have any connections
        mock_network.disconnect_peer(regular_peer_1);
    }
}

#[tokio::test]
async fn prioritized_peer_optimistic_fetch_latency_selection() {
    // Create a data client with a max lag of 100
    let max_optimistic_fetch_lag_secs = 100;
    let data_client_config = AptosDataClientConfig {
        max_optimistic_fetch_lag_secs,
        ..Default::default()
    };

    // Create the mock network, time service and client
    let (mut mock_network, time_service, client, _) =
        MockNetwork::new(None, Some(data_client_config), None);

    // Create test data
    let known_version = 10000000;
    let known_epoch = 10;

    // Ensure the properties hold for all optimistic fetch requests
    for data_request in enumerate_optimistic_fetch_requests(known_version, known_epoch) {
        let storage_request = StorageServiceRequest::new(data_request, true);

        // Add several regular peers and verify the peers cannot service the request
        let mut regular_peers = vec![];
        for _ in 0..5 {
            // Add a regular peer
            let regular_peer = mock_network.add_peer(false);
            regular_peers.push(regular_peer);

            // Verify the peer cannot service the request
            verify_request_is_unserviceable(&client, &storage_request);
        }

        // Advertise the data for the regular peers
        let timestamp_usecs = time_service.now_unix_time().as_micros() as u64;
        update_storage_summaries_for_peers(&client, &regular_peers, known_version, timestamp_usecs);

        // Verify the lowest latency regular peer is selected for the request
        let lowest_latency_peer = verify_lowest_latency_peer_selected(
            &mut mock_network,
            &client,
            &storage_request,
            &mut regular_peers,
        );

        // Disconnect the lowest latency peer and remove it from the list of regular peers
        disconnect_and_remove_peer(&mut mock_network, &mut regular_peers, lowest_latency_peer);

        // Verify the next lowest latency peer is now selected for the request
        let lowest_latency_peer = verify_lowest_latency_peer_selected(
            &mut mock_network,
            &client,
            &storage_request,
            &mut regular_peers,
        );

        // Add several priority peers and verify the regular peer is still selected
        let mut priority_peers = vec![];
        for _ in 0..3 {
            // Add a priority peer
            let priority_peer = mock_network.add_peer(true);
            priority_peers.push(priority_peer);

            // Verify the regular peer is still selected
            verify_peer_selected_for_request(&client, lowest_latency_peer, &storage_request);
        }

        // Advertise the data for the priority peers
        update_storage_summaries_for_peers(
            &client,
            &priority_peers,
            known_version,
            timestamp_usecs,
        );

        // Verify the lowest latency priority peer is selected for the request
        verify_lowest_latency_peer_selected(
            &mut mock_network,
            &client,
            &storage_request,
            &mut priority_peers,
        );

        // Disconnect all but one priority peer and remove them from the list of priority peers
        let last_priority_peer = priority_peers[0];
        for priority_peer in priority_peers.clone() {
            if priority_peer != last_priority_peer {
                mock_network.disconnect_peer(priority_peer);
            }
        }
        priority_peers.retain(|peer| *peer == last_priority_peer);

        // Verify the last priority peer is selected for the request
        verify_peer_selected_for_request(&client, last_priority_peer, &storage_request);

        // Disconnect the final priority peer and remove it from the list of priority peers
        disconnect_and_remove_peer(&mut mock_network, &mut priority_peers, last_priority_peer);

        // Verify the lowest latency regular peer is selected for the request
        verify_lowest_latency_peer_selected(
            &mut mock_network,
            &client,
            &storage_request,
            &mut regular_peers,
        );

        // Disconnect all regular peers and verify no peers can service the request
        for regular_peer in regular_peers {
            mock_network.disconnect_peer(regular_peer);
        }
        verify_request_is_unserviceable(&client, &storage_request);
    }
}

#[tokio::test]
async fn prioritized_peer_optimistic_fetch_missing_latencies() {
    // Create a data client with a max lag of 1000
    let max_optimistic_fetch_lag_secs = 1000;
    let data_client_config = AptosDataClientConfig {
        max_optimistic_fetch_lag_secs,
        ..Default::default()
    };

    // Create the mock network, time service and client
    let (mut mock_network, time_service, client, _) =
        MockNetwork::new(None, Some(data_client_config), None);

    // Create test data
    let known_version = 5;
    let known_epoch = 5;

    // Ensure the properties hold for all optimistic fetch requests
    for data_request in enumerate_optimistic_fetch_requests(known_version, known_epoch) {
        let storage_request = StorageServiceRequest::new(data_request, true);

        // Add several regular peers and remove their latency metadata
        let mut regular_peers = vec![];
        for _ in 0..5 {
            // Add a regular peer
            let regular_peer = mock_network.add_peer(false);
            regular_peers.push(regular_peer);

            // Remove the latency metadata for the peer
            utils::remove_latency_metadata(&client, regular_peer);
        }

        // Advertise the data for the regular peers
        let timestamp_usecs = time_service.now_unix_time().as_micros() as u64;
        update_storage_summaries_for_peers(&client, &regular_peers, known_version, timestamp_usecs);

        // Verify that a random peer is selected for the request
        let selected_peer = client.choose_peer_for_request(&storage_request).unwrap();
        assert!(regular_peers.contains(&selected_peer));

        // Disconnect the selected peer and verify another peer is selected
        disconnect_and_remove_peer(&mut mock_network, &mut regular_peers, selected_peer);
        let another_selected_peer = client.choose_peer_for_request(&storage_request).unwrap();
        assert_ne!(selected_peer, another_selected_peer);
        assert!(regular_peers.contains(&another_selected_peer));

        // Add several priority peers and remove their latency metadata
        let mut priority_peers = vec![];
        for _ in 0..3 {
            // Add a priority peer
            let priority_peer = mock_network.add_peer(true);
            priority_peers.push(priority_peer);

            // Remove the latency metadata for the peer
            utils::remove_latency_metadata(&client, priority_peer);
        }

        // Verify that a random regular peer is selected for the request
        let selected_peer = client.choose_peer_for_request(&storage_request).unwrap();
        assert!(regular_peers.contains(&selected_peer));

        // Advertise the data for the priority peers
        update_storage_summaries_for_peers(
            &client,
            &priority_peers,
            known_version,
            timestamp_usecs,
        );

        // Verify that a random priority peer is now selected for the request
        let selected_peer = client.choose_peer_for_request(&storage_request).unwrap();
        assert!(priority_peers.contains(&selected_peer));

        // Disconnect the priority peer and verify a random priority peer is selected
        disconnect_and_remove_peer(&mut mock_network, &mut priority_peers, selected_peer);
        let another_selected_peer = client.choose_peer_for_request(&storage_request).unwrap();
        assert_ne!(selected_peer, another_selected_peer);
        assert!(priority_peers.contains(&another_selected_peer));

        // Disconnect and remove all regular and priority peers
        for regular_peer in regular_peers.clone() {
            disconnect_and_remove_peer(&mut mock_network, &mut regular_peers, regular_peer);
        }
        for priority_peer in priority_peers.clone() {
            disconnect_and_remove_peer(&mut mock_network, &mut priority_peers, priority_peer);
        }

        // Verify no peers can service the request
        verify_request_is_unserviceable(&client, &storage_request);
    }
}

#[tokio::test]
async fn prioritized_peer_subscription_requests() {
    // Create a data client with a max lag of 10
    let max_subscription_lag_secs = 10;
    let data_client_config = AptosDataClientConfig {
        max_subscription_lag_secs,
        ..Default::default()
    };

    // Create the mock network, time service and client
    let (mut mock_network, time_service, client, _) =
        MockNetwork::new(None, Some(data_client_config), None);

    // Create test data
    let known_version = 1000;
    let known_epoch = 5;

    // Ensure the properties hold for all subscription requests
    for data_request in enumerate_subscription_requests(known_version, known_epoch) {
        let storage_request = StorageServiceRequest::new(data_request, true);

        // Ensure no peers can service the request (we have no connections)
        verify_request_is_unserviceable(&client, &storage_request);

        // Add two priority peers and a regular peer
        let priority_peer_1 = mock_network.add_peer(true);
        let priority_peer_2 = mock_network.add_peer(true);
        let regular_peer_1 = mock_network.add_peer(false);

        // Verify no peers can service the request (no peers are advertising data)
        verify_request_is_unserviceable(&client, &storage_request);

        // Advertise the data for all peers
        let timestamp_usecs = time_service.now_unix_time().as_micros() as u64;
        for peer in [priority_peer_1, priority_peer_2, regular_peer_1] {
            client.update_peer_storage_summary(
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
                client.update_peer_storage_summary(
                    peer,
                    utils::create_storage_summary_with_timestamp(known_version, timestamp_usecs),
                );
            }
        }

        // Verify no peers can service the request (because the
        // previously selected peer is still too far behind).
        verify_request_is_unserviceable(&client, &storage_request);

        // Verify the other priority peer is now select (as the
        // previous request will terminate the subscription).
        let next_selected_peer = client.choose_peer_for_request(&storage_request).unwrap();
        assert_ne!(selected_peer, next_selected_peer);
        assert!(selected_peer == priority_peer_1 || selected_peer == priority_peer_2);

        // Update the request's subscription ID and verify the other priority peer is selected
        let storage_request = update_subscription_request_id(&storage_request);
        let next_selected_peer = client.choose_peer_for_request(&storage_request).unwrap();
        assert_ne!(selected_peer, next_selected_peer);
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
        verify_request_is_unserviceable(&client, &storage_request);
    }
}

#[tokio::test]
async fn prioritized_peer_subscription_latency_selection() {
    // Create a data client with a max lag of 500
    let max_subscription_lag_secs = 500;
    let data_client_config = AptosDataClientConfig {
        max_subscription_lag_secs,
        ..Default::default()
    };

    // Create the mock network, time service and client
    let (mut mock_network, time_service, client, _) =
        MockNetwork::new(None, Some(data_client_config), None);

    // Create test data
    let known_version = 1;
    let known_epoch = 1;

    // Ensure the properties hold for all subscription requests
    for data_request in enumerate_subscription_requests(known_version, known_epoch) {
        let storage_request = StorageServiceRequest::new(data_request, true);

        // Add several regular peers and verify the peers cannot service the request
        let mut regular_peers = vec![];
        for _ in 0..5 {
            // Add a regular peer
            let regular_peer = mock_network.add_peer(false);
            regular_peers.push(regular_peer);

            // Verify the peer cannot service the request
            verify_request_is_unserviceable(&client, &storage_request);
        }

        // Advertise the data for the regular peers
        let timestamp_usecs = time_service.now_unix_time().as_micros() as u64;
        update_storage_summaries_for_peers(&client, &regular_peers, known_version, timestamp_usecs);

        // Verify the lowest latency regular peer is selected for the request
        let lowest_latency_peer = verify_lowest_latency_peer_selected(
            &mut mock_network,
            &client,
            &storage_request,
            &mut regular_peers,
        );

        // Add several priority peers and verify the regular peer is still selected
        let mut priority_peers = vec![];
        for _ in 0..3 {
            // Add a priority peer
            let priority_peer = mock_network.add_peer(true);
            priority_peers.push(priority_peer);

            // Verify the regular peer is still selected
            verify_peer_selected_for_request(&client, lowest_latency_peer, &storage_request);
        }

        // Advertise the data for the priority peers
        update_storage_summaries_for_peers(
            &client,
            &priority_peers,
            known_version,
            timestamp_usecs,
        );

        // Verify the request is unserviceable (the last request went to the regular peer)
        verify_request_is_unserviceable(&client, &storage_request);

        // Update the request's subscription ID and verify the
        // lowest latency priority peer is selected.
        let storage_request = update_subscription_request_id(&storage_request);
        verify_lowest_latency_peer_selected(
            &mut mock_network,
            &client,
            &storage_request,
            &mut priority_peers,
        );

        // Disconnect all but one priority peer and remove them from the list of priority peers
        let last_priority_peer = priority_peers[0];
        for priority_peer in priority_peers.clone() {
            if priority_peer != last_priority_peer {
                mock_network.disconnect_peer(priority_peer);
            }
        }
        priority_peers.retain(|peer| *peer == last_priority_peer);

        // Update the request's subscription ID and verify the
        // lowest latency priority peer is selected.
        let storage_request = update_subscription_request_id(&storage_request);
        verify_lowest_latency_peer_selected(
            &mut mock_network,
            &client,
            &storage_request,
            &mut priority_peers,
        );

        // Disconnect the final priority peer and remove it from the list of priority peers
        disconnect_and_remove_peer(&mut mock_network, &mut priority_peers, last_priority_peer);

        // Verify the request is unserviceable (the last request went to the priority peer)
        verify_request_is_unserviceable(&client, &storage_request);

        // Update the request's subscription ID and verify the
        // lowest latency regular peer is selected.
        let storage_request = update_subscription_request_id(&storage_request);
        verify_lowest_latency_peer_selected(
            &mut mock_network,
            &client,
            &storage_request,
            &mut regular_peers,
        );

        // Disconnect all regular peers and verify no peers can service the request
        for regular_peer in regular_peers {
            mock_network.disconnect_peer(regular_peer);
        }
        verify_request_is_unserviceable(&client, &storage_request);
    }
}

#[tokio::test]
async fn prioritized_peer_subscription_missing_latencies() {
    // Create a data client with a max lag of 900
    let max_subscription_lag_secs = 900;
    let data_client_config = AptosDataClientConfig {
        max_subscription_lag_secs,
        ..Default::default()
    };

    // Create the mock network, time service and client
    let (mut mock_network, time_service, client, _) =
        MockNetwork::new(None, Some(data_client_config), None);

    // Create test data
    let known_version = 1;
    let known_epoch = 1;

    // Ensure the properties hold for all subscription requests
    for data_request in enumerate_subscription_requests(known_version, known_epoch) {
        let storage_request = StorageServiceRequest::new(data_request, true);

        // Add several priority peers and remove their latency metadata
        let mut priority_peers = vec![];
        for _ in 0..3 {
            // Add a priority peer
            let priority_peer = mock_network.add_peer(true);
            priority_peers.push(priority_peer);

            // Remove the latency metadata for the peer
            utils::remove_latency_metadata(&client, priority_peer);
        }

        // Advertise the data for the priority peers
        let timestamp_usecs = time_service.now_unix_time().as_micros() as u64;
        update_storage_summaries_for_peers(
            &client,
            &priority_peers,
            known_version,
            timestamp_usecs,
        );

        // Verify that a random priority peer is selected for the request
        let selected_peer = client.choose_peer_for_request(&storage_request).unwrap();
        assert!(priority_peers.contains(&selected_peer));

        // Disconnect the selected peer and update the request's subscription ID
        disconnect_and_remove_peer(&mut mock_network, &mut priority_peers, selected_peer);
        let storage_request = update_subscription_request_id(&storage_request);

        // Verify that another priority peer is selected for the request
        let another_selected_peer = client.choose_peer_for_request(&storage_request).unwrap();
        assert_ne!(selected_peer, another_selected_peer);
        assert!(priority_peers.contains(&another_selected_peer));

        // Add several regular peers and remove their latency metadata
        let mut regular_peers = vec![];
        for _ in 0..10 {
            // Add a regular peer
            let regular_peer = mock_network.add_peer(false);
            regular_peers.push(regular_peer);

            // Remove the latency metadata for the peer
            utils::remove_latency_metadata(&client, regular_peer);
        }

        // Verify that a priority peer is still selected for the request
        let selected_peer = client.choose_peer_for_request(&storage_request).unwrap();
        assert!(priority_peers.contains(&selected_peer));

        // Advertise the data for the regular peers and update the request's subscription ID
        update_storage_summaries_for_peers(&client, &regular_peers, known_version, timestamp_usecs);
        let storage_request = update_subscription_request_id(&storage_request);

        // Verify that a random priority peer is still selected for the request
        let selected_peer = client.choose_peer_for_request(&storage_request).unwrap();
        assert!(priority_peers.contains(&selected_peer));

        // Disconnect and remove all priority peers
        for priority_peer in priority_peers.clone() {
            disconnect_and_remove_peer(&mut mock_network, &mut priority_peers, priority_peer);
        }

        // Update the request's subscription ID and verify that a random regular peer is selected
        let storage_request = update_subscription_request_id(&storage_request);
        let selected_peer = client.choose_peer_for_request(&storage_request).unwrap();
        assert!(regular_peers.contains(&selected_peer));

        // Disconnect the selected peer and update the request's subscription ID
        disconnect_and_remove_peer(&mut mock_network, &mut regular_peers, selected_peer);
        let storage_request = update_subscription_request_id(&storage_request);

        // Verify that another regular peer is selected for the request
        let another_selected_peer = client.choose_peer_for_request(&storage_request).unwrap();
        assert_ne!(selected_peer, another_selected_peer);
        assert!(regular_peers.contains(&another_selected_peer));

        // Disconnect and remove all regular peers
        for regular_peer in regular_peers.clone() {
            disconnect_and_remove_peer(&mut mock_network, &mut regular_peers, regular_peer);
        }

        // Verify no peers can service the request
        for _ in 0..10 {
            verify_request_is_unserviceable(&client, &storage_request);
        }
    }
}

#[tokio::test]
async fn prioritized_peer_subscription_sticky_selection() {
    // Create a data client with a max lag of 100
    let max_subscription_lag_secs = 100;
    let data_client_config = AptosDataClientConfig {
        max_subscription_lag_secs,
        ..Default::default()
    };

    // Create the mock network, time service and client
    let (mut mock_network, time_service, client, _) =
        MockNetwork::new(None, Some(data_client_config), None);

    // Create test data
    let known_version = 10000000;
    let known_epoch = 10;

    // Ensure the properties hold for all subscription requests
    for data_request in enumerate_subscription_requests(known_version, known_epoch) {
        let storage_request = StorageServiceRequest::new(data_request, true);

        // Ensure no peers can service the request (we have no connections)
        verify_request_is_unserviceable(&client, &storage_request);

        // Add a regular peer and verify the peer cannot service the request
        let regular_peer_1 = mock_network.add_peer(false);
        verify_request_is_unserviceable(&client, &storage_request);

        // Advertise the data for the regular peer and verify it is now selected
        let timestamp_usecs = time_service.now_unix_time().as_micros() as u64;
        client.update_peer_storage_summary(
            regular_peer_1,
            utils::create_storage_summary_with_timestamp(known_version, timestamp_usecs),
        );
        verify_peer_selected_for_request(&client, regular_peer_1, &storage_request);

        // Add a priority peer and verify the regular peer is still selected
        let priority_peer_1 = mock_network.add_peer(true);
        verify_peer_selected_for_request(&client, regular_peer_1, &storage_request);

        // Advertise the data for the priority peer and verify it is not selected
        // (the previous subscription request went to the regular peer).
        client.update_peer_storage_summary(
            priority_peer_1,
            utils::create_storage_summary_with_timestamp(known_version, timestamp_usecs),
        );
        verify_request_is_unserviceable(&client, &storage_request);

        // Update the request's subscription ID and verify it now goes to the priority peer
        let storage_request = update_subscription_request_id(&storage_request);
        verify_peer_selected_for_request(&client, priority_peer_1, &storage_request);

        // Elapse enough time for both peers to be too far behind
        time_service
            .clone()
            .advance_secs(max_subscription_lag_secs + 1);

        // Verify neither peer is now selected
        verify_request_is_unserviceable(&client, &storage_request);

        // Update the request's subscription ID
        let storage_request = update_subscription_request_id(&storage_request);

        // Update the regular peer to be up-to-date and verify it is now chosen
        let timestamp_usecs = time_service.now_unix_time().as_micros() as u64;
        let regular_peer_timestamp_usecs =
            timestamp_usecs - ((max_subscription_lag_secs / 2) * NUM_MICROSECONDS_IN_SECOND);
        client.update_peer_storage_summary(
            regular_peer_1,
            utils::create_storage_summary_with_timestamp(
                known_version,
                regular_peer_timestamp_usecs,
            ),
        );
        verify_peer_selected_for_request(&client, regular_peer_1, &storage_request);

        // Update the request's subscription ID
        let storage_request = update_subscription_request_id(&storage_request);

        // Update the priority peer to be up-to-date and verify it is now chosen
        let priority_peer_timestamp_usecs =
            timestamp_usecs - ((max_subscription_lag_secs / 2) * NUM_MICROSECONDS_IN_SECOND);
        client.update_peer_storage_summary(
            priority_peer_1,
            utils::create_storage_summary_with_timestamp(
                known_version,
                priority_peer_timestamp_usecs,
            ),
        );
        verify_peer_selected_for_request(&client, priority_peer_1, &storage_request);

        // Update the request's subscription ID
        let storage_request = update_subscription_request_id(&storage_request);

        // Disconnect the priority peer and verify the regular peer is selected
        mock_network.disconnect_peer(priority_peer_1);
        verify_peer_selected_for_request(&client, regular_peer_1, &storage_request);

        // Elapse enough time for the regular peer to be too far behind
        time_service.clone().advance_secs(max_subscription_lag_secs);

        // Verify neither peer is now select
        verify_request_is_unserviceable(&client, &storage_request);

        // Disconnect the regular peer so that we no longer have any connections
        mock_network.disconnect_peer(regular_peer_1);
    }
}

#[tokio::test]
async fn validator_peer_prioritization() {
    // Create a validator node
    let base_config = BaseConfig {
        role: RoleType::Validator,
        ..Default::default()
    };

    // Create the mock network and client
    let (mut mock_network, _, client, _) = MockNetwork::new(Some(base_config), None, None);

    // Add a validator peer and ensure it's prioritized
    let validator_peer = mock_network.add_peer_with_network_id(NetworkId::Validator, false);
    let (priority_peers, regular_peers) = client.get_priority_and_regular_peers().unwrap();
    assert_eq!(priority_peers, hashset![validator_peer]);
    assert_eq!(regular_peers, hashset![]);

    // Add a vfn peer and ensure it's not prioritized
    let vfn_peer = mock_network.add_peer_with_network_id(NetworkId::Vfn, true);
    let (priority_peers, regular_peers) = client.get_priority_and_regular_peers().unwrap();
    assert_eq!(priority_peers, hashset![validator_peer]);
    assert_eq!(regular_peers, hashset![vfn_peer]);
}

#[tokio::test]
async fn vfn_peer_prioritization() {
    // Create a validator fullnode
    let base_config = BaseConfig {
        role: RoleType::FullNode,
        ..Default::default()
    };

    // Create the mock network and client
    let (mut mock_network, _, client, _) = MockNetwork::new(Some(base_config), None, None);

    // Add a validator peer and ensure it's prioritized
    let validator_peer = mock_network.add_peer_with_network_id(NetworkId::Vfn, false);
    let (priority_peers, regular_peers) = client.get_priority_and_regular_peers().unwrap();
    assert_eq!(priority_peers, hashset![validator_peer]);
    assert_eq!(regular_peers, hashset![]);

    // Add a pfn peer and ensure it's not prioritized
    let pfn_peer = mock_network.add_peer_with_network_id(NetworkId::Public, true);
    let (priority_peers, regular_peers) = client.get_priority_and_regular_peers().unwrap();
    assert_eq!(priority_peers, hashset![validator_peer]);
    assert_eq!(regular_peers, hashset![pfn_peer]);
}

#[tokio::test]
async fn pfn_peer_prioritization() {
    // Create a public fullnode
    let base_config = BaseConfig {
        role: RoleType::FullNode,
        ..Default::default()
    };

    // Create the mock network and client
    let (mut mock_network, _, client, _) =
        MockNetwork::new(Some(base_config), None, Some(vec![NetworkId::Public]));

    // Add an inbound pfn peer and ensure it's not prioritized
    let inbound_peer = mock_network.add_peer_with_network_id(NetworkId::Public, false);
    let (priority_peers, regular_peers) = client.get_priority_and_regular_peers().unwrap();
    assert_eq!(priority_peers, hashset![]);
    assert_eq!(regular_peers, hashset![inbound_peer]);

    // Add an outbound pfn peer and ensure it's prioritized
    let outbound_peer = mock_network.add_peer_with_network_id(NetworkId::Public, true);
    let (priority_peers, regular_peers) = client.get_priority_and_regular_peers().unwrap();
    assert_eq!(priority_peers, hashset![outbound_peer]);
    assert_eq!(regular_peers, hashset![inbound_peer]);
}

/// Disconnects the given peer and removes it from the list of specified peers
fn disconnect_and_remove_peer(
    mock_network: &mut MockNetwork,
    peers: &mut Vec<PeerNetworkId>,
    peer_to_disconnect: PeerNetworkId,
) {
    // Disconnect the peer
    mock_network.disconnect_peer(peer_to_disconnect);

    // Remove the peer from the list of given peers
    peers.retain(|peer| *peer != peer_to_disconnect);
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

/// Returns the peer with the lowest latency from the given list of peers
fn get_lowest_latency_peer(
    peers: &[PeerNetworkId],
    mock_network: &mut MockNetwork,
) -> PeerNetworkId {
    let mut lowest_latency_peer = peers[0];
    let mut lowest_latency = f64::MAX;
    for peer in peers {
        // Get the peer's latency
        let ping_latency = utils::get_peer_ping_latency(mock_network, *peer);

        // Update the lowest latency peer
        if ping_latency < lowest_latency {
            lowest_latency = ping_latency;
            lowest_latency_peer = *peer;
        }
    }

    lowest_latency_peer
}

/// Updates the storage summaries for the given peers using the specified
/// version and timestamp.
fn update_storage_summaries_for_peers(
    client: &AptosDataClient,
    peers: &[PeerNetworkId],
    known_version: u64,
    timestamp_usecs: u64,
) {
    for peer in peers.iter() {
        client.update_peer_storage_summary(
            *peer,
            utils::create_storage_summary_with_timestamp(known_version, timestamp_usecs),
        );
    }
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

/// Verifies that the lowest latency peer is selected for the given request
/// and returns the lowest calculated latency peer.
fn verify_lowest_latency_peer_selected(
    mock_network: &mut MockNetwork,
    client: &AptosDataClient,
    storage_request: &StorageServiceRequest,
    regular_peers: &mut [PeerNetworkId],
) -> PeerNetworkId {
    // Calculate the lowest latency peer
    let lowest_latency_peer = get_lowest_latency_peer(regular_peers, mock_network);

    // Verify the lowest latency peer is selected for the given request
    verify_peer_selected_for_request(client, lowest_latency_peer, storage_request);

    lowest_latency_peer
}

/// Verifies that the peer is selected to service the given request
fn verify_peer_selected_for_request(
    client: &AptosDataClient,
    peer: PeerNetworkId,
    request: &StorageServiceRequest,
) {
    assert_eq!(client.choose_peer_for_request(request), Ok(peer));
}

/// Verifies that the given request is unserviceable
fn verify_request_is_unserviceable(client: &AptosDataClient, request: &StorageServiceRequest) {
    assert_matches!(
        client.choose_peer_for_request(request),
        Err(Error::DataIsUnavailable(_))
    );
}

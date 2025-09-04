// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    client::VelorDataClient,
    priority::PeerPriority,
    tests::{mock::MockNetwork, utils},
};
use velor_config::{
    config::{VelorDataClientConfig, VelorDataMultiFetchConfig},
    network_id::{NetworkId, PeerNetworkId},
};
use velor_storage_service_types::{
    requests::{DataRequest, StorageServiceRequest},
    responses::NUM_MICROSECONDS_IN_SECOND,
};
use velor_time_service::TimeServiceTrait;
use maplit::hashset;
use std::{cmp::Ordering, collections::HashSet};

#[tokio::test]
async fn prioritized_peer_request_selection() {
    // Create a data client with multi-fetch disabled
    let data_client_config = VelorDataClientConfig {
        data_multi_fetch_config: VelorDataMultiFetchConfig {
            enable_multi_fetch: false,
            ..Default::default()
        },
        ..Default::default()
    };

    // Create the mock network and client
    let (mut mock_network, _, client, _) = MockNetwork::new(None, Some(data_client_config), None);

    // Ensure the properties hold for storage summary and version requests
    let storage_summary_request = DataRequest::GetStorageServerSummary;
    let get_version_request = DataRequest::GetServerProtocolVersion;
    for data_request in [storage_summary_request, get_version_request] {
        let storage_request = StorageServiceRequest::new(data_request, true);

        // Ensure no peers can service the request (we have no connections)
        utils::verify_request_is_unserviceable(&client, &storage_request, true);

        // Add a medium priority peer and verify the peer is selected as the recipient
        let medium_priority_peer = mock_network.add_peer(PeerPriority::MediumPriority);
        utils::verify_selected_peers_match(
            &client,
            hashset![medium_priority_peer],
            &storage_request,
        );

        // Add a high priority peer and verify the peer is selected as the recipient
        let high_priority_peer_1 = mock_network.add_peer(PeerPriority::HighPriority);
        utils::verify_selected_peers_match(
            &client,
            hashset![high_priority_peer_1],
            &storage_request,
        );

        // Disconnect the high priority peer and verify the medium priority peer is now chosen
        mock_network.disconnect_peer(high_priority_peer_1);
        utils::verify_selected_peers_match(
            &client,
            hashset![medium_priority_peer],
            &storage_request,
        );

        // Connect a new high priority peer and verify it is now selected
        let high_priority_peer_2 = mock_network.add_peer(PeerPriority::HighPriority);
        utils::verify_selected_peers_match(
            &client,
            hashset![high_priority_peer_2],
            &storage_request,
        );

        // Disconnect the high priority peer and verify the medium priority peer is again chosen
        mock_network.disconnect_peer(high_priority_peer_2);
        utils::verify_selected_peers_match(
            &client,
            hashset![medium_priority_peer],
            &storage_request,
        );

        // Disconnect the medium priority peer so that we no longer have any connections
        mock_network.disconnect_peer(medium_priority_peer);
    }
}

#[tokio::test]
async fn prioritized_peer_optimistic_fetch_selection() {
    // Create a data client with a max lag of 100 and multi-fetch disabled
    let max_optimistic_fetch_lag_secs = 100;
    let data_client_config = VelorDataClientConfig {
        data_multi_fetch_config: VelorDataMultiFetchConfig {
            enable_multi_fetch: false,
            ..Default::default()
        },
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
    for data_request in utils::enumerate_optimistic_fetch_requests(known_version, known_epoch) {
        let storage_request = StorageServiceRequest::new(data_request, true);

        // Ensure no peers can service the request (we have no connections)
        utils::verify_request_is_unserviceable(&client, &storage_request, true);

        // Add a medium priority peer and verify the peer cannot service the request
        let medium_priority_peer_1 = mock_network.add_peer(PeerPriority::MediumPriority);
        utils::verify_request_is_unserviceable(&client, &storage_request, false);

        // Advertise the data for the medium priority peer and verify it is now selected
        let timestamp_usecs = time_service.now_unix_time().as_micros() as u64;
        client.update_peer_storage_summary(
            medium_priority_peer_1,
            utils::create_storage_summary_with_timestamp(known_version, timestamp_usecs),
        );
        utils::verify_selected_peers_match(
            &client,
            hashset![medium_priority_peer_1],
            &storage_request,
        );

        // Add a high priority peer and verify the medium priority peer is still selected
        let high_priority_peer_1 = mock_network.add_peer(PeerPriority::HighPriority);
        utils::verify_selected_peers_match(
            &client,
            hashset![medium_priority_peer_1],
            &storage_request,
        );

        // Advertise the data for the high priority peer and verify it is now selected
        client.update_peer_storage_summary(
            high_priority_peer_1,
            utils::create_storage_summary_with_timestamp(known_version, timestamp_usecs),
        );
        utils::verify_selected_peers_match(
            &client,
            hashset![high_priority_peer_1],
            &storage_request,
        );

        // Elapse enough time for both peers to be too far behind
        time_service
            .clone()
            .advance_secs(max_optimistic_fetch_lag_secs + 1);

        // Verify neither peer is now selected
        utils::verify_request_is_unserviceable(&client, &storage_request, false);

        // Update the medium priority peer to be up-to-date and verify it is now chosen
        let timestamp_usecs = time_service.now_unix_time().as_micros() as u64;
        let peer_timestamp_usecs =
            timestamp_usecs - ((max_optimistic_fetch_lag_secs / 2) * NUM_MICROSECONDS_IN_SECOND);
        client.update_peer_storage_summary(
            medium_priority_peer_1,
            utils::create_storage_summary_with_timestamp(known_version, peer_timestamp_usecs),
        );
        utils::verify_selected_peers_match(
            &client,
            hashset![medium_priority_peer_1],
            &storage_request,
        );

        // Update the high priority peer to be up-to-date and verify it is now chosen
        let peer_timestamp_usecs =
            timestamp_usecs - ((max_optimistic_fetch_lag_secs / 2) * NUM_MICROSECONDS_IN_SECOND);
        client.update_peer_storage_summary(
            high_priority_peer_1,
            utils::create_storage_summary_with_timestamp(known_version, peer_timestamp_usecs),
        );
        utils::verify_selected_peers_match(
            &client,
            hashset![high_priority_peer_1],
            &storage_request,
        );

        // Disconnect the high priority peer and verify the medium priority peer is selected
        mock_network.disconnect_peer(high_priority_peer_1);
        utils::verify_selected_peers_match(
            &client,
            hashset![medium_priority_peer_1],
            &storage_request,
        );

        // Elapse enough time for the medium priority peer to be too far behind
        time_service
            .clone()
            .advance_secs(max_optimistic_fetch_lag_secs);

        // Verify neither peer is now select
        utils::verify_request_is_unserviceable(&client, &storage_request, false);

        // Disconnect the medium priority peer so that we no longer have any connections
        mock_network.disconnect_peer(medium_priority_peer_1);
    }
}

#[tokio::test]
async fn prioritized_peer_optimistic_fetch_distance_latency_selection() {
    // Create a data client with a max lag of 100 and multi-fetch disabled
    let max_optimistic_fetch_lag_secs = 100;
    let data_client_config = VelorDataClientConfig {
        data_multi_fetch_config: VelorDataMultiFetchConfig {
            enable_multi_fetch: false,
            ..Default::default()
        },
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
    for data_request in utils::enumerate_optimistic_fetch_requests(known_version, known_epoch) {
        let storage_request = StorageServiceRequest::new(data_request, true);

        // Add several medium priority peers and verify the peers cannot service the request
        let mut medium_priority_peers = hashset![];
        for _ in 0..5 {
            // Add a medium priority peer
            let peer = mock_network.add_peer(PeerPriority::MediumPriority);
            medium_priority_peers.insert(peer);

            // Verify the peer cannot service the request
            utils::verify_request_is_unserviceable(&client, &storage_request, false);
        }

        // Advertise the data for the medium priority peers
        utils::update_storage_summaries_for_peers(
            &client,
            &medium_priority_peers,
            known_version,
            time_service.now_unix_time().as_micros(),
        );

        // Verify the peers are selected by distance and latency
        let selected_peers = verify_peers_selected_by_distance_and_latency(
            &mut mock_network,
            &client,
            &storage_request,
            &mut medium_priority_peers,
        );

        // Disconnect the selected peers and remove them from the list of medium priority peers
        disconnect_and_remove_peers(
            &mut mock_network,
            &mut medium_priority_peers,
            &selected_peers,
        );

        // Verify the next set of peers are selected by distance and latency
        let selected_peers = verify_peers_selected_by_distance_and_latency(
            &mut mock_network,
            &client,
            &storage_request,
            &mut medium_priority_peers,
        );

        // Add several high priority peers and verify the medium priority peers are still selected
        let mut high_priority_peers = hashset![];
        for _ in 0..3 {
            // Add a high priority peer
            let peer = mock_network.add_peer(PeerPriority::HighPriority);
            high_priority_peers.insert(peer);

            // Verify the medium priority peers are still selected
            utils::verify_selected_peers_match(&client, selected_peers.clone(), &storage_request);
        }

        // Advertise the data for the high priority peers
        utils::update_storage_summaries_for_peers(
            &client,
            &high_priority_peers,
            known_version,
            time_service.now_unix_time().as_micros(),
        );

        // Verify the high priority peers are selected by distance and latency
        verify_peers_selected_by_distance_and_latency(
            &mut mock_network,
            &client,
            &storage_request,
            &mut high_priority_peers,
        );

        // Disconnect all but one high priority peer and remove them from the list of peers
        let last_priority_peer = *high_priority_peers.iter().next().unwrap();
        for peer in high_priority_peers.clone() {
            if peer != last_priority_peer {
                mock_network.disconnect_peer(peer);
            }
        }
        high_priority_peers.retain(|peer| *peer == last_priority_peer);

        // Verify the last high priority peer is selected for the request
        utils::verify_selected_peers_match(&client, hashset![last_priority_peer], &storage_request);

        // Disconnect the final high priority peer and remove it from the list of peers
        disconnect_and_remove_peers(&mut mock_network, &mut high_priority_peers, &hashset![
            last_priority_peer
        ]);

        // Verify a medium priority peer is selected by distance and latency
        verify_peers_selected_by_distance_and_latency(
            &mut mock_network,
            &client,
            &storage_request,
            &mut medium_priority_peers,
        );

        // Disconnect all medium priority peers and verify no peers can service the request
        utils::disconnect_all_peers(&mut mock_network, &medium_priority_peers);
        utils::verify_request_is_unserviceable(&client, &storage_request, true);
    }
}

#[tokio::test]
async fn prioritized_peer_optimistic_fetch_missing_distances() {
    // Create a data client with a max lag of 1000
    let max_optimistic_fetch_lag_secs = 1000;
    let data_client_config = VelorDataClientConfig {
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
    for data_request in utils::enumerate_optimistic_fetch_requests(known_version, known_epoch) {
        let storage_request = StorageServiceRequest::new(data_request, true);

        // Add several medium priority peers and remove their distance metadata
        let mut medium_priority_peers = hashset![];
        for _ in 0..5 {
            // Add a medium priority peer
            let peer = mock_network.add_peer(PeerPriority::LowPriority);
            medium_priority_peers.insert(peer);

            // Remove the distance metadata for the peer
            utils::remove_distance_metadata(&client, peer);
        }

        // Advertise the data for the medium priority peers
        utils::update_storage_summaries_for_peers(
            &client,
            &medium_priority_peers,
            known_version,
            time_service.now_unix_time().as_micros(),
        );

        // Verify that random medium priority peers are selected for the request
        let selected_peers = client.choose_peers_for_request(&storage_request).unwrap();
        assert!(medium_priority_peers.is_superset(&selected_peers));

        // Disconnect the selected peers and verify other peers are selected
        disconnect_and_remove_peers(
            &mut mock_network,
            &mut medium_priority_peers,
            &selected_peers,
        );
        let other_selected_peers = client.choose_peers_for_request(&storage_request).unwrap();
        assert_ne!(selected_peers, other_selected_peers);
        assert!(medium_priority_peers.is_superset(&other_selected_peers));

        // Add several high priority peers and remove their distance metadata
        let mut high_priority_peers = hashset![];
        for _ in 0..3 {
            // Add a high priority peer
            let peer = mock_network.add_peer(PeerPriority::HighPriority);
            high_priority_peers.insert(peer);

            // Remove the distance metadata for the peer
            utils::remove_distance_metadata(&client, peer);
        }

        // Verify that medium priority peers are selected for the request
        let selected_peers = client.choose_peers_for_request(&storage_request).unwrap();
        assert!(medium_priority_peers.is_superset(&selected_peers));

        // Advertise the data for the high priority peers
        utils::update_storage_summaries_for_peers(
            &client,
            &high_priority_peers,
            known_version,
            time_service.now_unix_time().as_micros(),
        );

        // Verify that high priority peers are now selected for the request
        let selected_peers = client.choose_peers_for_request(&storage_request).unwrap();
        assert!(high_priority_peers.is_superset(&selected_peers));

        // Disconnect the high priority peers and verify more high priority peers are selected
        disconnect_and_remove_peers(&mut mock_network, &mut high_priority_peers, &selected_peers);
        let other_selected_peers = client.choose_peers_for_request(&storage_request).unwrap();
        assert_ne!(selected_peers, other_selected_peers);
        assert!(high_priority_peers.is_superset(&other_selected_peers));

        // Disconnect and remove all medium priority and high priority peers
        disconnect_and_remove_all_peers(&mut mock_network, &mut medium_priority_peers);
        disconnect_and_remove_all_peers(&mut mock_network, &mut high_priority_peers);

        // Verify no peers can service the request
        utils::verify_request_is_unserviceable(&client, &storage_request, true);
    }
}

#[tokio::test]
async fn prioritized_peer_optimistic_fetch_missing_latencies() {
    // Create a data client with a max lag of 1000
    let max_optimistic_fetch_lag_secs = 1000;
    let data_client_config = VelorDataClientConfig {
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
    for data_request in utils::enumerate_optimistic_fetch_requests(known_version, known_epoch) {
        let storage_request = StorageServiceRequest::new(data_request, true);

        // Add several medium priority peers and remove their latency metadata
        let mut medium_priority_peers = hashset![];
        for _ in 0..5 {
            // Add a medium priority peer
            let peer = mock_network.add_peer(PeerPriority::LowPriority);
            medium_priority_peers.insert(peer);

            // Remove the latency metadata for the peer
            utils::remove_latency_metadata(&client, peer);
        }

        // Advertise the data for the medium priority peers
        utils::update_storage_summaries_for_peers(
            &client,
            &medium_priority_peers,
            known_version,
            time_service.now_unix_time().as_micros(),
        );

        // Verify that random medium priority peers are selected for the request
        let selected_peers = client.choose_peers_for_request(&storage_request).unwrap();
        assert!(medium_priority_peers.is_superset(&selected_peers));

        // Disconnect the selected peers and verify other peers are selected
        disconnect_and_remove_peers(
            &mut mock_network,
            &mut medium_priority_peers,
            &selected_peers,
        );
        let other_selected_peers = client.choose_peers_for_request(&storage_request).unwrap();
        assert_ne!(selected_peers, other_selected_peers);
        assert!(medium_priority_peers.is_superset(&other_selected_peers));

        // Add several high priority peers and remove their latency metadata
        let mut high_priority_peers = hashset![];
        for _ in 0..3 {
            // Add a high priority peer
            let peer = mock_network.add_peer(PeerPriority::HighPriority);
            high_priority_peers.insert(peer);

            // Remove the latency metadata for the peer
            utils::remove_latency_metadata(&client, peer);
        }

        // Verify that random medium priority peers are selected for the request
        let selected_peers = client.choose_peers_for_request(&storage_request).unwrap();
        assert!(medium_priority_peers.is_superset(&selected_peers));

        // Advertise the data for the high priority peers
        utils::update_storage_summaries_for_peers(
            &client,
            &high_priority_peers,
            known_version,
            time_service.now_unix_time().as_micros(),
        );

        // Verify that random high priority peers are now selected for the request
        let selected_peers = client.choose_peers_for_request(&storage_request).unwrap();
        assert!(high_priority_peers.is_superset(&selected_peers));

        // Disconnect the high priority peers and verify more priority peers are selected
        disconnect_and_remove_peers(&mut mock_network, &mut high_priority_peers, &selected_peers);
        let other_selected_peers = client.choose_peers_for_request(&storage_request).unwrap();
        assert_ne!(selected_peers, other_selected_peers);
        assert!(high_priority_peers.is_superset(&other_selected_peers));

        // Disconnect and remove all medium priority and high priority peers
        disconnect_and_remove_all_peers(&mut mock_network, &mut medium_priority_peers);
        disconnect_and_remove_all_peers(&mut mock_network, &mut high_priority_peers);

        // Verify no peers can service the request
        utils::verify_request_is_unserviceable(&client, &storage_request, true);
    }
}

#[tokio::test]
async fn prioritized_peer_subscription_requests() {
    // Create a data client with a max lag of 10
    let max_subscription_lag_secs = 10;
    let data_client_config = VelorDataClientConfig {
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
    for data_request in utils::enumerate_subscription_requests(known_version, known_epoch) {
        let storage_request = StorageServiceRequest::new(data_request, true);

        // Ensure no peers can service the request (we have no connections)
        utils::verify_request_is_unserviceable(&client, &storage_request, true);

        // Add two high priority peers and a medium priority peer
        let high_priority_peer_1 = mock_network.add_peer(PeerPriority::HighPriority);
        let high_priority_peer_2 = mock_network.add_peer(PeerPriority::HighPriority);
        let medium_priority_peer = mock_network.add_peer(PeerPriority::LowPriority);

        // Verify no peers can service the request (no peers are advertising data)
        utils::verify_request_is_unserviceable(&client, &storage_request, false);

        // Advertise the data for all peers
        let timestamp_usecs = time_service.now_unix_time().as_micros() as u64;
        for peer in [
            high_priority_peer_1,
            high_priority_peer_2,
            medium_priority_peer,
        ] {
            client.update_peer_storage_summary(
                peer,
                utils::create_storage_summary_with_timestamp(known_version, timestamp_usecs),
            );
        }

        // Verify a high priority peer is selected
        let selected_peer = client.choose_peers_for_request(&storage_request).unwrap();
        assert!(
            selected_peer == hashset![high_priority_peer_1]
                || selected_peer == hashset![high_priority_peer_2]
        );

        // Make several more requests and verify the same priority peer is selected
        for _ in 0..10 {
            utils::verify_selected_peers_match(&client, selected_peer.clone(), &storage_request);
        }

        // Elapse enough time for all peers to be too far behind
        time_service
            .clone()
            .advance_secs(max_subscription_lag_secs + 1);

        // Advertise new data for all peers (except the selected peer)
        let timestamp_usecs = time_service.now_unix_time().as_micros() as u64;
        for peer in [
            high_priority_peer_1,
            high_priority_peer_2,
            medium_priority_peer,
        ] {
            if hashset![peer] != selected_peer {
                client.update_peer_storage_summary(
                    peer,
                    utils::create_storage_summary_with_timestamp(known_version, timestamp_usecs),
                );
            }
        }

        // Verify no peers can service the request (because the
        // previously selected peer is still too far behind).
        utils::verify_request_is_unserviceable(&client, &storage_request, false);

        // Verify the other high priority peer is now select (as the
        // previous request will terminate the subscription).
        let next_selected_peer = client.choose_peers_for_request(&storage_request).unwrap();
        assert_ne!(selected_peer, next_selected_peer);
        assert!(
            selected_peer == hashset![high_priority_peer_1]
                || selected_peer == hashset![high_priority_peer_2]
        );

        // Update the request's subscription ID and verify the other high priority peer is selected
        let storage_request = utils::update_subscription_request_id(&storage_request);
        let next_selected_peer = client.choose_peers_for_request(&storage_request).unwrap();
        assert_ne!(selected_peer, next_selected_peer);
        assert!(
            next_selected_peer == hashset![high_priority_peer_1]
                || next_selected_peer == hashset![high_priority_peer_2]
        );

        // Make several more requests and verify the same high priority peer is selected
        for _ in 0..10 {
            utils::verify_selected_peers_match(
                &client,
                next_selected_peer.clone(),
                &storage_request,
            );
        }

        // Disconnect all peers and verify no peers can service the request
        utils::disconnect_all_peers(&mut mock_network, &hashset![
            high_priority_peer_1,
            high_priority_peer_2,
            medium_priority_peer
        ]);
        utils::verify_request_is_unserviceable(&client, &storage_request, true);
    }
}

#[tokio::test]
async fn prioritized_peer_subscription_distance_latency_selection() {
    // Create a data client with a max lag of 500
    let max_subscription_lag_secs = 500;
    let data_client_config = VelorDataClientConfig {
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
    for data_request in utils::enumerate_subscription_requests(known_version, known_epoch) {
        let storage_request = StorageServiceRequest::new(data_request, true);

        // Add several low priority peers and verify the peers cannot service the request
        let mut low_priority_peers = hashset![];
        for _ in 0..5 {
            // Add a low priority peer
            let peer = mock_network.add_peer(PeerPriority::LowPriority);
            low_priority_peers.insert(peer);

            // Verify the peer cannot service the request
            utils::verify_request_is_unserviceable(&client, &storage_request, false);
        }

        // Advertise the data for the low priority peers
        utils::update_storage_summaries_for_peers(
            &client,
            &low_priority_peers,
            known_version,
            time_service.now_unix_time().as_micros(),
        );

        // Verify the low priority peer is selected by distance and latency
        let selected_peers = verify_peers_selected_by_distance_and_latency(
            &mut mock_network,
            &client,
            &storage_request,
            &mut low_priority_peers,
        );
        assert_eq!(selected_peers.len(), 1);
        let low_priority_peer = selected_peers.iter().next().unwrap();

        // Add several high priority peers and verify the low priority peer is still selected
        let mut high_priority_peers = hashset![];
        for _ in 0..3 {
            // Add a high priority peer
            let peer = mock_network.add_peer(PeerPriority::HighPriority);
            high_priority_peers.insert(peer);

            // Verify the low priority peer is still selected
            utils::verify_selected_peers_match(
                &client,
                hashset![*low_priority_peer],
                &storage_request,
            );
        }

        // Advertise the data for the high priority peers
        utils::update_storage_summaries_for_peers(
            &client,
            &high_priority_peers,
            known_version,
            time_service.now_unix_time().as_micros(),
        );

        // Verify the request is unserviceable (the last request went to the low priority peer)
        utils::verify_request_is_unserviceable(&client, &storage_request, false);

        // Update the request's subscription ID and verify
        // the high priority peer is selected by distance and latency.
        let storage_request = utils::update_subscription_request_id(&storage_request);
        verify_peers_selected_by_distance_and_latency(
            &mut mock_network,
            &client,
            &storage_request,
            &mut high_priority_peers,
        );

        // Disconnect all but one high priority peer and remove them from the list of peers
        let last_priority_peer = *high_priority_peers.iter().next().unwrap();
        for peer in high_priority_peers.clone() {
            if peer != last_priority_peer {
                mock_network.disconnect_peer(peer);
            }
        }
        high_priority_peers.retain(|peer| *peer == last_priority_peer);

        // Update the request's subscription ID and verify the
        // high priority peer is selected by distance and latency.
        let storage_request = utils::update_subscription_request_id(&storage_request);
        verify_peers_selected_by_distance_and_latency(
            &mut mock_network,
            &client,
            &storage_request,
            &mut high_priority_peers,
        );

        // Disconnect the final high priority peer and remove it from the list of peers
        disconnect_and_remove_peers(&mut mock_network, &mut high_priority_peers, &hashset![
            last_priority_peer
        ]);

        // Verify the request is unserviceable (the last request went to the high priority peer)
        utils::verify_request_is_unserviceable(&client, &storage_request, false);

        // Update the request's subscription ID and verify the
        // low priority peer is selected by distance and latency.
        let storage_request = utils::update_subscription_request_id(&storage_request);
        verify_peers_selected_by_distance_and_latency(
            &mut mock_network,
            &client,
            &storage_request,
            &mut low_priority_peers,
        );

        // Disconnect all low priority peers and verify no peers can service the request
        utils::disconnect_all_peers(&mut mock_network, &low_priority_peers);
        utils::verify_request_is_unserviceable(&client, &storage_request, true);
    }
}

#[tokio::test]
async fn prioritized_peer_subscription_missing_distances() {
    // Create a data client with a max lag of 900
    let max_subscription_lag_secs = 900;
    let data_client_config = VelorDataClientConfig {
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
    for data_request in utils::enumerate_subscription_requests(known_version, known_epoch) {
        let storage_request = StorageServiceRequest::new(data_request, true);

        // Add several high priority peers and remove their distance metadata
        let mut high_priority_peers = hashset![];
        for _ in 0..3 {
            // Add a high priority peer
            let peer = mock_network.add_peer(PeerPriority::HighPriority);
            high_priority_peers.insert(peer);

            // Remove the distance metadata for the peer
            utils::remove_distance_metadata(&client, peer);
        }

        // Advertise the data for the high priority peers
        utils::update_storage_summaries_for_peers(
            &client,
            &high_priority_peers,
            known_version,
            time_service.now_unix_time().as_micros(),
        );

        // Verify that a random high priority peer is selected for the request
        let selected_peer = client.choose_peers_for_request(&storage_request).unwrap();
        verify_peer_in_set(&selected_peer, &high_priority_peers);

        // Disconnect the selected peer and update the request's subscription ID
        disconnect_and_remove_peers(&mut mock_network, &mut high_priority_peers, &selected_peer);
        let storage_request = utils::update_subscription_request_id(&storage_request);

        // Verify that another high priority peer is selected for the request
        let another_selected_peer = client.choose_peers_for_request(&storage_request).unwrap();
        assert_ne!(selected_peer, another_selected_peer);
        verify_peer_in_set(&another_selected_peer, &high_priority_peers);

        // Add several low priority peers and remove their distance metadata
        let mut low_priority_peers = hashset![];
        for _ in 0..10 {
            // Add a low priority peer
            let peer = mock_network.add_peer(PeerPriority::LowPriority);
            low_priority_peers.insert(peer);

            // Remove the distance metadata for the peer
            utils::remove_distance_metadata(&client, peer);
        }

        // Verify that a high priority peer is still selected for the request
        let selected_peer = client.choose_peers_for_request(&storage_request).unwrap();
        verify_peer_in_set(&selected_peer, &high_priority_peers);

        // Advertise the data for the low priority peers and update the request's subscription ID
        utils::update_storage_summaries_for_peers(
            &client,
            &low_priority_peers,
            known_version,
            time_service.now_unix_time().as_micros(),
        );
        let storage_request = utils::update_subscription_request_id(&storage_request);

        // Verify that a random high priority peer is still selected for the request
        let selected_peer = client.choose_peers_for_request(&storage_request).unwrap();
        verify_peer_in_set(&selected_peer, &high_priority_peers);

        // Disconnect and remove all high priority peers
        disconnect_and_remove_all_peers(&mut mock_network, &mut high_priority_peers);

        // Update the request's subscription ID and verify that a random low priority peer is selected
        let storage_request = utils::update_subscription_request_id(&storage_request);
        let selected_peer = client.choose_peers_for_request(&storage_request).unwrap();
        verify_peer_in_set(&selected_peer, &low_priority_peers);

        // Disconnect the selected peer and update the request's subscription ID
        disconnect_and_remove_peers(&mut mock_network, &mut low_priority_peers, &selected_peer);
        let storage_request = utils::update_subscription_request_id(&storage_request);

        // Verify that another low priority peer is selected for the request
        let another_selected_peer = client.choose_peers_for_request(&storage_request).unwrap();
        assert_ne!(selected_peer, another_selected_peer);
        verify_peer_in_set(&another_selected_peer, &low_priority_peers);

        // Disconnect and remove all low priority peers
        disconnect_and_remove_all_peers(&mut mock_network, &mut low_priority_peers);

        // Verify no peers can service the request
        for _ in 0..10 {
            utils::verify_request_is_unserviceable(&client, &storage_request, true);
        }
    }
}

#[tokio::test]
async fn prioritized_peer_subscription_missing_latencies() {
    // Create a data client with a max lag of 900
    let max_subscription_lag_secs = 900;
    let data_client_config = VelorDataClientConfig {
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
    for data_request in utils::enumerate_subscription_requests(known_version, known_epoch) {
        let storage_request = StorageServiceRequest::new(data_request, true);

        // Add several high priority peers and remove their latency metadata
        let mut high_priority_peers = hashset![];
        for _ in 0..3 {
            // Add a high priority peer
            let peer = mock_network.add_peer(PeerPriority::HighPriority);
            high_priority_peers.insert(peer);

            // Remove the latency metadata for the peer
            utils::remove_latency_metadata(&client, peer);
        }

        // Advertise the data for the high priority peers
        utils::update_storage_summaries_for_peers(
            &client,
            &high_priority_peers,
            known_version,
            time_service.now_unix_time().as_micros(),
        );

        // Verify that a random high priority peer is selected for the request
        let selected_peer = client.choose_peers_for_request(&storage_request).unwrap();
        verify_peer_in_set(&selected_peer, &high_priority_peers);

        // Disconnect the selected peer and update the request's subscription ID
        disconnect_and_remove_peers(&mut mock_network, &mut high_priority_peers, &selected_peer);
        let storage_request = utils::update_subscription_request_id(&storage_request);

        // Verify that another high priority peer is selected for the request
        let another_selected_peer = client.choose_peers_for_request(&storage_request).unwrap();
        assert_ne!(selected_peer, another_selected_peer);
        verify_peer_in_set(&another_selected_peer, &high_priority_peers);

        // Add several medium priority peers and remove their latency metadata
        let mut medium_priority_peers = hashset![];
        for _ in 0..10 {
            // Add a medium priority peer
            let peer = mock_network.add_peer(PeerPriority::MediumPriority);
            medium_priority_peers.insert(peer);

            // Remove the latency metadata for the peer
            utils::remove_latency_metadata(&client, peer);
        }

        // Verify that a high priority peer is still selected for the request
        let selected_peer = client.choose_peers_for_request(&storage_request).unwrap();
        verify_peer_in_set(&selected_peer, &high_priority_peers);

        // Advertise the data for the medium priority peers and update the request's subscription ID
        utils::update_storage_summaries_for_peers(
            &client,
            &medium_priority_peers,
            known_version,
            time_service.now_unix_time().as_micros(),
        );
        let storage_request = utils::update_subscription_request_id(&storage_request);

        // Verify that a random high priority peer is still selected for the request
        let selected_peer = client.choose_peers_for_request(&storage_request).unwrap();
        verify_peer_in_set(&selected_peer, &high_priority_peers);

        // Disconnect and remove all high priority peers
        disconnect_and_remove_all_peers(&mut mock_network, &mut high_priority_peers);

        // Update the request's subscription ID and verify that a random medium priority peer is selected
        let storage_request = utils::update_subscription_request_id(&storage_request);
        let selected_peer = client.choose_peers_for_request(&storage_request).unwrap();
        verify_peer_in_set(&selected_peer, &medium_priority_peers);

        // Disconnect the selected peer and update the request's subscription ID
        disconnect_and_remove_peers(
            &mut mock_network,
            &mut medium_priority_peers,
            &selected_peer,
        );
        let storage_request = utils::update_subscription_request_id(&storage_request);

        // Verify that another medium priority peer is selected for the request
        let another_selected_peer = client.choose_peers_for_request(&storage_request).unwrap();
        assert_ne!(selected_peer, another_selected_peer);
        verify_peer_in_set(&another_selected_peer, &medium_priority_peers);

        // Disconnect and remove all medium priority peers
        disconnect_and_remove_all_peers(&mut mock_network, &mut medium_priority_peers);

        // Verify no peers can service the request
        for _ in 0..10 {
            utils::verify_request_is_unserviceable(&client, &storage_request, true);
        }
    }
}

#[tokio::test]
async fn prioritized_peer_subscription_sticky_selection() {
    // Create a data client with a max lag of 100
    let max_subscription_lag_secs = 100;
    let data_client_config = VelorDataClientConfig {
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
    for data_request in utils::enumerate_subscription_requests(known_version, known_epoch) {
        let storage_request = StorageServiceRequest::new(data_request, true);

        // Ensure no peers can service the request (we have no connections)
        utils::verify_request_is_unserviceable(&client, &storage_request, true);

        // Add a low priority peer and verify the peer cannot service the request
        let low_priority_peer_1 = mock_network.add_peer(PeerPriority::LowPriority);
        utils::verify_request_is_unserviceable(&client, &storage_request, false);

        // Advertise the data for the low priority peer and verify it is now selected
        let timestamp_usecs = time_service.now_unix_time().as_micros() as u64;
        client.update_peer_storage_summary(
            low_priority_peer_1,
            utils::create_storage_summary_with_timestamp(known_version, timestamp_usecs),
        );
        utils::verify_selected_peers_match(
            &client,
            hashset![low_priority_peer_1],
            &storage_request,
        );

        // Add a high priority peer and verify the low priority peer is still selected
        let high_priority_peer_1 = mock_network.add_peer(PeerPriority::HighPriority);
        utils::verify_selected_peers_match(
            &client,
            hashset![low_priority_peer_1],
            &storage_request,
        );

        // Advertise the data for the high priority peer and verify it is not selected
        // (the previous subscription request went to the low priority peer).
        client.update_peer_storage_summary(
            high_priority_peer_1,
            utils::create_storage_summary_with_timestamp(known_version, timestamp_usecs),
        );
        utils::verify_request_is_unserviceable(&client, &storage_request, false);

        // Update the request's subscription ID and verify it now goes to the high priority peer
        let storage_request = utils::update_subscription_request_id(&storage_request);
        utils::verify_selected_peers_match(
            &client,
            hashset![high_priority_peer_1],
            &storage_request,
        );

        // Elapse enough time for both peers to be too far behind
        time_service
            .clone()
            .advance_secs(max_subscription_lag_secs + 1);

        // Verify neither peer is now selected
        utils::verify_request_is_unserviceable(&client, &storage_request, false);

        // Update the request's subscription ID
        let storage_request = utils::update_subscription_request_id(&storage_request);

        // Update the low priority peer to be up-to-date and verify it is now chosen
        let timestamp_usecs = time_service.now_unix_time().as_micros() as u64;
        let peer_timestamp_usecs =
            timestamp_usecs - ((max_subscription_lag_secs / 2) * NUM_MICROSECONDS_IN_SECOND);
        client.update_peer_storage_summary(
            low_priority_peer_1,
            utils::create_storage_summary_with_timestamp(known_version, peer_timestamp_usecs),
        );
        utils::verify_selected_peers_match(
            &client,
            hashset![low_priority_peer_1],
            &storage_request,
        );

        // Update the request's subscription ID
        let storage_request = utils::update_subscription_request_id(&storage_request);

        // Update the high priority peer to be up-to-date and verify it is now chosen
        let peer_timestamp_usecs =
            timestamp_usecs - ((max_subscription_lag_secs / 2) * NUM_MICROSECONDS_IN_SECOND);
        client.update_peer_storage_summary(
            high_priority_peer_1,
            utils::create_storage_summary_with_timestamp(known_version, peer_timestamp_usecs),
        );
        utils::verify_selected_peers_match(
            &client,
            hashset![high_priority_peer_1],
            &storage_request,
        );

        // Update the request's subscription ID
        let storage_request = utils::update_subscription_request_id(&storage_request);

        // Disconnect the high priority peer and verify the low priority peer is selected
        mock_network.disconnect_peer(high_priority_peer_1);
        utils::verify_selected_peers_match(
            &client,
            hashset![low_priority_peer_1],
            &storage_request,
        );

        // Elapse enough time for the low priority peer to be too far behind
        time_service.clone().advance_secs(max_subscription_lag_secs);

        // Verify neither peer is now select
        utils::verify_request_is_unserviceable(&client, &storage_request, false);

        // Disconnect the low priority peer so that we no longer have any connections
        mock_network.disconnect_peer(low_priority_peer_1);
    }
}

#[tokio::test]
async fn validator_peer_prioritization() {
    // Create a base config for a validator
    let base_config = utils::create_validator_base_config();

    // Create the mock network and client
    let (mut mock_network, _, client, _) = MockNetwork::new(Some(base_config), None, None);

    // Add a validator peer and ensure it's prioritized
    let validator_peer = mock_network.add_peer_with_network_id(NetworkId::Validator, false);
    let (priority_peers, regular_peers) = client.get_priority_and_regular_peers().unwrap();
    assert_eq!(priority_peers, hashset![validator_peer]);
    assert!(regular_peers.is_empty());

    // Add a vfn peer and ensure it's not prioritized
    let vfn_peer = mock_network.add_peer_with_network_id(NetworkId::Vfn, true);
    let (priority_peers, regular_peers) = client.get_priority_and_regular_peers().unwrap();
    assert_eq!(priority_peers, hashset![validator_peer]);
    assert_eq!(regular_peers, hashset![vfn_peer]);
}

#[tokio::test]
async fn vfn_peer_prioritization() {
    // Create a base config for a VFN
    let base_config = utils::create_fullnode_base_config();

    // Create the mock network and client
    let (mut mock_network, _, client, _) = MockNetwork::new(Some(base_config), None, None);

    // Add a validator peer and ensure it's prioritized
    let validator_peer = mock_network.add_peer_with_network_id(NetworkId::Vfn, false);
    let (priority_peers, regular_peers) = client.get_priority_and_regular_peers().unwrap();
    assert_eq!(priority_peers, hashset![validator_peer]);
    assert!(regular_peers.is_empty());

    // Add a pfn peer and ensure it's not prioritized
    let pfn_peer = mock_network.add_peer_with_network_id(NetworkId::Public, true);
    let (priority_peers, regular_peers) = client.get_priority_and_regular_peers().unwrap();
    assert_eq!(priority_peers, hashset![validator_peer]);
    assert_eq!(regular_peers, hashset![pfn_peer]);
}

#[tokio::test]
async fn pfn_peer_prioritization() {
    // Create a base config for a PFN
    let base_config = utils::create_fullnode_base_config();

    // Create the mock network and client
    let (mut mock_network, _, client, _) =
        MockNetwork::new(Some(base_config), None, Some(vec![NetworkId::Public]));

    // Add an inbound pfn peer and ensure it's not prioritized
    let inbound_peer = mock_network.add_peer_with_network_id(NetworkId::Public, false);
    let (priority_peers, regular_peers) = client.get_priority_and_regular_peers().unwrap();
    assert!(priority_peers.is_empty());
    assert_eq!(regular_peers, hashset![inbound_peer]);

    // Add an outbound pfn peer and ensure it's prioritized
    let outbound_peer = mock_network.add_peer_with_network_id(NetworkId::Public, true);
    let (priority_peers, regular_peers) = client.get_priority_and_regular_peers().unwrap();
    assert_eq!(priority_peers, hashset![outbound_peer]);
    assert_eq!(regular_peers, hashset![inbound_peer]);
}

/// Disconnects all the given peers and removes them from the list of specified peers
fn disconnect_and_remove_all_peers(
    mock_network: &mut MockNetwork,
    peers: &mut HashSet<PeerNetworkId>,
) {
    utils::disconnect_all_peers(mock_network, peers);
    peers.clear();
}

/// Disconnects the given peers and removes them from the list of specified peers
fn disconnect_and_remove_peers(
    mock_network: &mut MockNetwork,
    peers: &mut HashSet<PeerNetworkId>,
    peers_to_disconnect: &HashSet<PeerNetworkId>,
) {
    for peer_to_disconnect in peers_to_disconnect {
        mock_network.disconnect_peer(*peer_to_disconnect);
        peers.retain(|peer| *peer != *peer_to_disconnect);
    }
}

/// Returns the peers with the lowest validator distance from the given list of peers
fn get_lowest_distance_peers(
    peers: &HashSet<PeerNetworkId>,
    mock_network: &mut MockNetwork,
) -> HashSet<PeerNetworkId> {
    let mut lowest_distance_peers = hashset![];
    let mut lowest_distance = u64::MAX;

    // Identify the peers with the lowest distance
    for peer in peers {
        // Get the peer's distance
        let distance = utils::get_peer_distance_from_validators(mock_network, *peer);

        // Update the lowest distance peers
        match distance.cmp(&lowest_distance) {
            Ordering::Equal => {
                // Add the peer to the list of lowest distance peers
                lowest_distance_peers.insert(*peer);
            },
            Ordering::Less => {
                // We found a new lowest distance!
                lowest_distance = distance;
                lowest_distance_peers = hashset![*peer];
            },
            Ordering::Greater => {
                // The peer is not a lowest distance peer
            },
        }
    }

    lowest_distance_peers
}

/// Verifies that the given peer set contains a single entry
/// and that the single peer is in the superset.
fn verify_peer_in_set(single_peer: &HashSet<PeerNetworkId>, peers: &HashSet<PeerNetworkId>) {
    assert_eq!(single_peer.len(), 1);
    assert!(peers.is_superset(single_peer));
}

/// Verifies that low distance and latency peers are selected for
/// the given request (from the specified list of potential peers)
/// and returns the selected peers.
fn verify_peers_selected_by_distance_and_latency(
    mock_network: &mut MockNetwork,
    client: &VelorDataClient,
    storage_request: &StorageServiceRequest,
    potential_peers: &mut HashSet<PeerNetworkId>,
) -> HashSet<PeerNetworkId> {
    // Select peers for the given request
    let selected_peers = client.choose_peers_for_request(storage_request).unwrap();
    for selected_peer in &selected_peers {
        // Verify the selected peer is in the list of potential peers
        assert!(potential_peers.contains(selected_peer));

        // Verify the selected peer has the lowest distance
        let lowest_distance_peers = get_lowest_distance_peers(potential_peers, mock_network);
        assert!(lowest_distance_peers.contains(selected_peer));
    }

    selected_peers
}

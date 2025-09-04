// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    client::VelorDataClient,
    priority::PeerPriority,
    tests::{mock::MockNetwork, utils, utils::NUM_SELECTION_ITERATIONS},
};
use velor_config::{
    config::{VelorDataClientConfig, VelorDataMultiFetchConfig},
    network_id::NetworkId,
};
use velor_storage_service_types::requests::{
    DataRequest, StorageServiceRequest, TransactionOutputsWithProofRequest,
};
use velor_time_service::TimeServiceTrait;
use maplit::hashset;
use std::collections::{HashMap, HashSet};

#[tokio::test]
async fn multi_fetch_disabled_trivial_request() {
    // Create a base config for a VFN
    let base_config = utils::create_fullnode_base_config();
    let networks = vec![NetworkId::Vfn, NetworkId::Public];

    // Create a data client config with multi-fetch disabled
    let data_client_config = VelorDataClientConfig {
        data_multi_fetch_config: VelorDataMultiFetchConfig {
            enable_multi_fetch: false,
            ..Default::default()
        },
        ..Default::default()
    };

    // Create the mock network and client
    let (mut mock_network, _, client, _) =
        MockNetwork::new(Some(base_config), Some(data_client_config), Some(networks));

    // Create a server version request that is trivially serviceable
    let server_version_request =
        StorageServiceRequest::new(DataRequest::GetServerProtocolVersion, true);

    // Ensure no peers can service the request (we have no connections)
    utils::verify_request_is_unserviceable(&client, &server_version_request, true);

    // Ensure the properties hold for all peer priorities (with increasing priority)
    for peer_priority in PeerPriority::get_all_ordered_priorities().iter().rev() {
        // Add many peers for the current priority
        let peers = utils::add_several_peers(&mut mock_network, 100, *peer_priority);

        // Verify only a single peer is selected from the current priority set
        utils::verify_selected_peer_from_set(&client, &server_version_request, &peers);
    }
}

#[tokio::test]
async fn multi_fetch_disabled_optimistic_fetch_request() {
    // Create a base config for a VFN
    let base_config = utils::create_fullnode_base_config();
    let networks = vec![NetworkId::Vfn, NetworkId::Public];

    // Create a data client config with multi-fetch disabled
    let data_client_config = VelorDataClientConfig {
        data_multi_fetch_config: VelorDataMultiFetchConfig {
            enable_multi_fetch: false,
            ..Default::default()
        },
        ..Default::default()
    };

    // Create test data
    let known_version = 10000000;
    let known_epoch = 10;

    // Ensure the properties hold for all optimistic fetch requests
    for data_request in utils::enumerate_optimistic_fetch_requests(known_version, known_epoch) {
        // Ensure the properties hold for all peer priorities
        for peer_priority in PeerPriority::get_all_ordered_priorities() {
            // Create the storage request
            let storage_request = StorageServiceRequest::new(data_request.clone(), true);

            // Create the mock network, time service and client
            let (mut mock_network, time_service, client, _) = MockNetwork::new(
                Some(base_config.clone()),
                Some(data_client_config),
                Some(networks.clone()),
            );

            // Ensure no peers can service the request (we have no connections)
            utils::verify_request_is_unserviceable(&client, &storage_request, true);

            // Add several peers and verify the request is still unserviceable
            let peers = utils::add_several_peers(&mut mock_network, 100, peer_priority);
            utils::verify_request_is_unserviceable(&client, &storage_request, false);

            // Advertise the data for the peers and verify a single peer is selected
            let timestamp_usecs = time_service.now_unix_time().as_micros() as u64;
            for peer in peers.iter() {
                client.update_peer_storage_summary(
                    *peer,
                    utils::create_storage_summary_with_timestamp(known_version, timestamp_usecs),
                );
            }
            utils::verify_selected_peer_from_set(&client, &storage_request, &peers);
        }
    }
}

#[tokio::test]
async fn multi_fetch_optimistic_fetch_extend_with_random_peers() {
    // Create a base config for a validator
    let base_config = utils::create_validator_base_config();

    // Create the data client config with a max lag of 5 and multi-fetch enabled
    let max_optimistic_fetch_lag_secs = 5;
    let num_peers_for_multi_fetch = 5;
    let data_client_config = VelorDataClientConfig {
        data_multi_fetch_config: VelorDataMultiFetchConfig {
            enable_multi_fetch: true,
            min_peers_for_multi_fetch: num_peers_for_multi_fetch,
            max_peers_for_multi_fetch: num_peers_for_multi_fetch,
            ..Default::default()
        },
        max_optimistic_fetch_lag_secs,
        ..Default::default()
    };

    // Create test data
    let known_version = 10000000;
    let known_epoch = 10;
    let min_validator_distance = 0;
    let max_validator_distance = 3;

    // Ensure the properties hold for all peer priorities
    for peer_priority in PeerPriority::get_all_ordered_priorities() {
        // Ensure the properties hold for all optimistic fetch requests
        for data_request in utils::enumerate_optimistic_fetch_requests(known_version, known_epoch) {
            // Ensure the properties hold for both distance and latency metadata
            for remove_distance_metadata in [true, false] {
                // Create the mock network and client
                let (mut mock_network, time_service, client, _) =
                    MockNetwork::new(Some(base_config.clone()), Some(data_client_config), None);

                // Create the storage request
                let storage_request = StorageServiceRequest::new(data_request.clone(), true);

                // Add several peers
                let num_peers = 50;
                let peers = utils::add_several_peers_with_metadata(
                    &mut mock_network,
                    &client,
                    num_peers as u64,
                    min_validator_distance,
                    max_validator_distance,
                    peer_priority,
                );

                // Remove the distance or latency metadata for all except some peers
                let num_peers_with_latency_metadata = 2;
                let peers: Vec<_> = peers.into_iter().collect();
                for peer in peers[num_peers_with_latency_metadata..].iter() {
                    if remove_distance_metadata {
                        utils::remove_distance_metadata(&client, *peer)
                    } else {
                        utils::remove_latency_metadata(&client, *peer);
                    }
                }

                // Advertise the data for the peers
                utils::update_storage_summaries_for_peers(
                    &client,
                    &HashSet::from_iter(peers.clone()),
                    known_version,
                    time_service.now_unix_time().as_micros(),
                );

                // Select peers to service the request multiple times
                let peers_and_selection_counts = utils::select_peers_multiple_times(
                    &client,
                    num_peers_for_multi_fetch,
                    &storage_request,
                );

                // Build a max-heap of all peers by their selection counts
                let mut max_heap_selection_counts =
                    utils::build_selection_count_max_heap(&peers_and_selection_counts);

                // Verify the top peers in the max-heap are the peers with latency metadata
                for _ in 0..num_peers_with_latency_metadata {
                    // Get the peer monitoring metadata
                    let peer_monitoring_metadata = utils::get_peer_monitoring_metadata(
                        &mut mock_network,
                        max_heap_selection_counts.pop().unwrap().1,
                    );

                    // Verify the appropriate metadata is present
                    if remove_distance_metadata {
                        assert!(peer_monitoring_metadata
                            .latest_network_info_response
                            .is_some());
                    } else {
                        assert!(peer_monitoring_metadata.average_ping_latency_secs.is_some());
                    }
                }

                // Verify the rest of the peers in the max-heap are the peers without latency metadata
                for _ in num_peers_with_latency_metadata..num_peers {
                    // Get the peer monitoring metadata
                    let peer_monitoring_metadata = utils::get_peer_monitoring_metadata(
                        &mut mock_network,
                        max_heap_selection_counts.pop().unwrap().1,
                    );

                    // Verify the appropriate metadata has been removed
                    if remove_distance_metadata {
                        assert!(peer_monitoring_metadata
                            .latest_network_info_response
                            .is_none());
                    } else {
                        assert!(peer_monitoring_metadata.average_ping_latency_secs.is_none());
                    }
                }
            }
        }
    }
}

#[tokio::test]
async fn multi_fetch_optimistic_fetch_priority() {
    // Create a base config for a VFN
    let base_config = utils::create_fullnode_base_config();
    let networks = vec![NetworkId::Vfn, NetworkId::Public];

    // Create a data client with a max lag of 10 and multi-fetch enabled (3 peers per request)
    let peers_for_multi_fetch = 3;
    let max_optimistic_fetch_lag_secs = 10;
    let data_client_config = VelorDataClientConfig {
        data_multi_fetch_config: VelorDataMultiFetchConfig {
            enable_multi_fetch: true,
            min_peers_for_multi_fetch: peers_for_multi_fetch,
            max_peers_for_multi_fetch: peers_for_multi_fetch,
            ..Default::default()
        },
        max_optimistic_fetch_lag_secs,
        ..Default::default()
    };

    // Create the mock network, time service and client
    let (mut mock_network, time_service, client, _) =
        MockNetwork::new(Some(base_config), Some(data_client_config), Some(networks));

    // Create test data
    let known_version = 1000;
    let known_epoch = 5;

    // Ensure the properties hold for all peer priorities (in increasing priority order)
    for peer_priority in PeerPriority::get_all_ordered_priorities().iter().rev() {
        // Ensure the properties hold for all optimistic fetch requests
        for data_request in utils::enumerate_optimistic_fetch_requests(known_version, known_epoch) {
            // Create the storage request
            let storage_request = StorageServiceRequest::new(data_request, true);

            // Add several peers and verify the request is unserviceable
            let peers = utils::add_several_peers(&mut mock_network, 100, *peer_priority);
            utils::verify_request_is_unserviceable(&client, &storage_request, false);

            // Advertise the data for the peers and verify multiple peers are selected
            let timestamp_usecs = time_service.now_unix_time().as_micros() as u64;
            for peer in peers.iter() {
                client.update_peer_storage_summary(
                    *peer,
                    utils::create_storage_summary_with_timestamp(known_version, timestamp_usecs),
                );
            }
            verify_num_selected_peers(&client, &storage_request, 3);

            // Disconnect all peers
            utils::disconnect_all_peers(&mut mock_network, &HashSet::from_iter(peers));

            // Add a single peer and advertise the data for the peer
            let peer = mock_network.add_peer(*peer_priority);
            client.update_peer_storage_summary(
                peer,
                utils::create_storage_summary_with_timestamp(known_version, timestamp_usecs),
            );

            // Verify the peer is selected
            utils::verify_selected_peers_match(&client, hashset![peer], &storage_request);

            // Disconnect the peer and verify the request is unserviceable
            mock_network.disconnect_peer(peer);
            utils::verify_request_is_unserviceable(&client, &storage_request, true);
        }
    }
}

#[tokio::test]
async fn multi_fetch_optimistic_fetch_priority_mix() {
    // Create a base config for a validator
    let base_config = utils::create_validator_base_config();

    // Create a data client with a max lag of 1 and multi-fetch enabled (4 peers per request)
    let peers_for_multi_fetch = 4;
    let max_optimistic_fetch_lag_secs = 1;
    let data_client_config = VelorDataClientConfig {
        data_multi_fetch_config: VelorDataMultiFetchConfig {
            enable_multi_fetch: true,
            min_peers_for_multi_fetch: 1,
            max_peers_for_multi_fetch: peers_for_multi_fetch,
            multi_fetch_peer_bucket_size: 2,
            ..Default::default()
        },
        max_optimistic_fetch_lag_secs,
        ..Default::default()
    };

    // Create the mock network, time service and client
    let (mut mock_network, time_service, client, _) =
        MockNetwork::new(Some(base_config), Some(data_client_config), None);

    // Create test data
    let known_version = 1000;
    let known_epoch = 5;

    // Ensure the properties hold for all optimistic fetch requests
    for data_request in utils::enumerate_optimistic_fetch_requests(known_version, known_epoch) {
        // Create the storage request
        let storage_request = StorageServiceRequest::new(data_request, true);

        // Add several low priority peers and verify the request is unserviceable
        let low_priority_peers =
            utils::add_several_peers(&mut mock_network, 100, PeerPriority::LowPriority);
        utils::verify_request_is_unserviceable(&client, &storage_request, false);

        // Advertise the data for the low priority peers and verify multiple peers are selected
        let timestamp_usecs = time_service.now_unix_time().as_micros() as u64;
        for peer in low_priority_peers.iter() {
            client.update_peer_storage_summary(
                *peer,
                utils::create_storage_summary_with_timestamp(known_version, timestamp_usecs),
            );
        }
        utils::verify_selected_peers_from_set(
            &client,
            &storage_request,
            peers_for_multi_fetch,
            &low_priority_peers,
        );

        // Add several medium priority peers
        let medium_priority_peers =
            utils::add_several_peers(&mut mock_network, 100, PeerPriority::MediumPriority);

        // Verify the request is still serviced by low priority peers
        utils::verify_selected_peers_from_set(
            &client,
            &storage_request,
            peers_for_multi_fetch,
            &low_priority_peers,
        );

        // Advertise the data for the medium priority peers
        for peer in medium_priority_peers.iter() {
            client.update_peer_storage_summary(
                *peer,
                utils::create_storage_summary_with_timestamp(known_version, timestamp_usecs),
            );
        }

        // Make several requests and verify medium priority peers are selected
        for _ in 0..100 {
            utils::verify_selected_peers_from_set(
                &client,
                &storage_request,
                peers_for_multi_fetch,
                &medium_priority_peers,
            );
        }

        // Add several high priority peers
        let high_priority_peers =
            utils::add_several_peers(&mut mock_network, 100, PeerPriority::HighPriority);

        // Verify the request is still serviced by medium priority peers
        utils::verify_selected_peers_from_set(
            &client,
            &storage_request,
            peers_for_multi_fetch,
            &medium_priority_peers,
        );

        // Advertise the data for the high priority peers
        for peer in high_priority_peers.iter() {
            client.update_peer_storage_summary(
                *peer,
                utils::create_storage_summary_with_timestamp(known_version, timestamp_usecs),
            );
        }

        // Make several requests and verify high priority peers are selected
        for _ in 0..100 {
            utils::verify_selected_peers_from_set(
                &client,
                &storage_request,
                peers_for_multi_fetch,
                &high_priority_peers,
            );
        }

        // Disconnect all high priority peers (except a single one)
        for (index, peer) in high_priority_peers.iter().enumerate() {
            if index != 0 {
                mock_network.disconnect_peer(*peer);
            }
        }
        let high_priority_peer = *high_priority_peers.iter().next().unwrap();

        // Make several requests and verify medium priority peers are selected (alongside the high priority peer)
        for _ in 0..100 {
            let selected_peers = client.choose_peers_for_request(&storage_request).unwrap();
            assert_eq!(selected_peers.len(), peers_for_multi_fetch);
            assert!(selected_peers.contains(&high_priority_peer));
            let selected_medium_priority_peers: HashSet<_> = selected_peers
                .difference(&hashset![high_priority_peer])
                .cloned()
                .collect();
            medium_priority_peers.is_superset(&selected_medium_priority_peers);
        }

        // Disconnect all medium priority peers
        utils::disconnect_all_peers(&mut mock_network, &medium_priority_peers);

        // Make several requests and verify only the high priority peer is selected
        for _ in 0..100 {
            utils::verify_selected_peers_match(
                &client,
                hashset![high_priority_peer],
                &storage_request,
            );
        }

        // Disconnect the high priority peer
        mock_network.disconnect_peer(high_priority_peer);

        // Make several requests and verify low priority peers are selected
        for _ in 0..100 {
            utils::verify_selected_peers_from_set(
                &client,
                &storage_request,
                peers_for_multi_fetch,
                &low_priority_peers,
            );
        }

        // Add two medium priority peers
        let medium_priority_peers = hashset![
            mock_network.add_peer(PeerPriority::MediumPriority),
            mock_network.add_peer(PeerPriority::MediumPriority)
        ];

        // Advertise the data for the medium priority peers
        for peer in medium_priority_peers.iter() {
            client.update_peer_storage_summary(
                *peer,
                utils::create_storage_summary_with_timestamp(known_version, timestamp_usecs),
            );
        }

        // Make several requests and verify the medium priority peers are selected
        for _ in 0..100 {
            utils::verify_selected_peers_match(
                &client,
                medium_priority_peers.clone(),
                &storage_request,
            );
        }

        // Disconnect all medium priority peers
        utils::disconnect_all_peers(&mut mock_network, &medium_priority_peers);

        // Make several requests and verify the low priority peers are selected
        for _ in 0..100 {
            utils::verify_selected_peers_from_set(
                &client,
                &storage_request,
                peers_for_multi_fetch,
                &low_priority_peers,
            );
        }

        // Disconnect all low priority peers
        utils::disconnect_all_peers(&mut mock_network, &low_priority_peers);

        // Verify the request is unserviceable
        utils::verify_request_is_unserviceable(&client, &storage_request, true);
    }
}

#[tokio::test]
async fn multi_fetch_peer_bucket_sizes() {
    // Create a base config for a validator
    let base_config = utils::create_validator_base_config();

    // Create a data client config with multi-fetch enabled
    let additional_requests_per_peer_bucket = 2;
    let min_peers_for_multi_fetch = 2;
    let max_peers_for_multi_fetch = 1000;
    let multi_fetch_peer_bucket_size = 4;
    let data_client_config = VelorDataClientConfig {
        data_multi_fetch_config: VelorDataMultiFetchConfig {
            enable_multi_fetch: true,
            additional_requests_per_peer_bucket,
            min_peers_for_multi_fetch,
            max_peers_for_multi_fetch,
            multi_fetch_peer_bucket_size,
        },
        ..Default::default()
    };

    // Create the mock network and client
    let (mut mock_network, _, client, _) =
        MockNetwork::new(Some(base_config), Some(data_client_config), None);

    // Create a server version request that is trivially serviceable
    let server_version_request =
        StorageServiceRequest::new(DataRequest::GetServerProtocolVersion, true);

    // Add the minimum number of low priority peers and verify the correct number of peers are selected
    let low_priority_peers = utils::add_several_peers(
        &mut mock_network,
        min_peers_for_multi_fetch as u64,
        PeerPriority::LowPriority,
    );
    utils::verify_selected_peers_match(
        &client,
        low_priority_peers.clone(),
        &server_version_request,
    );

    // Add a large number of low priority peers and verify the correct number of peers are selected
    let _ = utils::add_several_peers(
        &mut mock_network,
        (max_peers_for_multi_fetch * 2) as u64,
        PeerPriority::LowPriority,
    );
    verify_num_selected_peers(&client, &server_version_request, max_peers_for_multi_fetch);

    // Add less than the minimum number of medium priority peers and verify the correct number of peers are selected
    let medium_priority_peers = utils::add_several_peers(
        &mut mock_network,
        (min_peers_for_multi_fetch - 1) as u64,
        PeerPriority::MediumPriority,
    );
    verify_num_selected_peers(
        &client,
        &server_version_request,
        min_peers_for_multi_fetch - 1,
    );

    // Disconnect all medium priority peers and verify the correct number of peers are selected
    utils::disconnect_all_peers(&mut mock_network, &medium_priority_peers);
    verify_num_selected_peers(&client, &server_version_request, max_peers_for_multi_fetch);

    // Add exactly a single bucket of high priority peers and verify the correct number of peers are selected
    let mut high_priority_peers = utils::add_several_peers(
        &mut mock_network,
        multi_fetch_peer_bucket_size as u64,
        PeerPriority::HighPriority,
    );
    verify_num_selected_peers(
        &client,
        &server_version_request,
        min_peers_for_multi_fetch + additional_requests_per_peer_bucket,
    );

    // Continue to add buckets of high priority peers and verify the correct number of peers are selected
    for index in 0..10 {
        for _ in 0..multi_fetch_peer_bucket_size {
            let high_priority_peer = mock_network.add_peer(PeerPriority::HighPriority);
            high_priority_peers.insert(high_priority_peer);
        }
        verify_num_selected_peers(
            &client,
            &server_version_request,
            min_peers_for_multi_fetch + additional_requests_per_peer_bucket * (index + 2),
        );
    }

    // Disconnect all high priority peers and verify the correct number of peers are selected
    utils::disconnect_all_peers(&mut mock_network, &high_priority_peers);
    verify_num_selected_peers(&client, &server_version_request, max_peers_for_multi_fetch);
}

#[tokio::test]
async fn multi_fetch_peer_bucket_sizes_across_buckets() {
    // Create a base config for a validator
    let base_config = utils::create_validator_base_config();

    // Create a data client config with multi-fetch enabled
    let additional_requests_per_peer_bucket = 10;
    let min_peers_for_multi_fetch = 5;
    let max_peers_for_multi_fetch = 1000;
    let multi_fetch_peer_bucket_size = 1;
    let data_client_config = VelorDataClientConfig {
        data_multi_fetch_config: VelorDataMultiFetchConfig {
            enable_multi_fetch: true,
            additional_requests_per_peer_bucket,
            min_peers_for_multi_fetch,
            max_peers_for_multi_fetch,
            multi_fetch_peer_bucket_size,
        },
        ..Default::default()
    };

    // Create the mock network and client
    let (mut mock_network, _, client, _) =
        MockNetwork::new(Some(base_config), Some(data_client_config), None);

    // Create a server version request that is trivially serviceable
    let server_version_request =
        StorageServiceRequest::new(DataRequest::GetServerProtocolVersion, true);

    // Add a large number of low priority peers and verify the correct number of peers are selected
    let low_priority_peers = utils::add_several_peers(
        &mut mock_network,
        (max_peers_for_multi_fetch * 2) as u64,
        PeerPriority::LowPriority,
    );
    verify_num_selected_peers(&client, &server_version_request, max_peers_for_multi_fetch);

    // Add less than the minimum number of medium priority peers and verify the correct number of peers are selected
    let medium_priority_peers = utils::add_several_peers(
        &mut mock_network,
        (min_peers_for_multi_fetch - 1) as u64,
        PeerPriority::MediumPriority,
    );
    verify_num_selected_peers(
        &client,
        &server_version_request,
        min_peers_for_multi_fetch - 1,
    );

    // Add a single high priority peer and verify the correct peers are selected
    let high_priority_peer_1 = mock_network.add_peer(PeerPriority::HighPriority);
    let expected_peers = hashset![high_priority_peer_1]
        .union(&medium_priority_peers)
        .cloned()
        .collect();
    utils::verify_selected_peers_match(&client, expected_peers, &server_version_request);

    // Remove all medium priority peers and verify the correct peers are selected
    utils::disconnect_all_peers(&mut mock_network, &medium_priority_peers);
    utils::verify_selected_peers_match(
        &client,
        hashset![high_priority_peer_1],
        &server_version_request,
    );

    // Add another high priority peer and verify the correct peers are selected
    let high_priority_peer_2 = mock_network.add_peer(PeerPriority::HighPriority);
    utils::verify_selected_peers_match(
        &client,
        hashset![high_priority_peer_1, high_priority_peer_2],
        &server_version_request,
    );

    // Remove all high priority peers and verify the correct peers are selected
    utils::disconnect_all_peers(&mut mock_network, &hashset![
        high_priority_peer_1,
        high_priority_peer_2
    ]);
    verify_num_selected_peers(&client, &server_version_request, max_peers_for_multi_fetch);

    // Disconnect all low priority peers
    utils::disconnect_all_peers(&mut mock_network, &low_priority_peers);

    // Connect a single high priority peer and medium priority peer
    let high_priority_peer_1 = mock_network.add_peer(PeerPriority::HighPriority);
    let medium_priority_peer_1 = mock_network.add_peer(PeerPriority::MediumPriority);

    // Verify the correct peers are selected
    utils::verify_selected_peers_match(
        &client,
        hashset![high_priority_peer_1, medium_priority_peer_1],
        &server_version_request,
    );

    // Disconnect all peers
    utils::disconnect_all_peers(&mut mock_network, &hashset![
        high_priority_peer_1,
        medium_priority_peer_1
    ]);

    // Verify the request is unserviceable
    utils::verify_request_is_unserviceable(&client, &server_version_request, true);
}

#[tokio::test]
async fn multi_fetch_request_selection_extend_with_random_peers() {
    // Create a base config for a validator
    let base_config = utils::create_validator_base_config();

    // Create a data client with multi-fetch enabled (4 peers per request)
    let num_peers_for_multi_fetch = 4;
    let data_client_config = VelorDataClientConfig {
        data_multi_fetch_config: VelorDataMultiFetchConfig {
            enable_multi_fetch: true,
            min_peers_for_multi_fetch: num_peers_for_multi_fetch,
            max_peers_for_multi_fetch: num_peers_for_multi_fetch,
            ..Default::default()
        },
        ..Default::default()
    };

    // Ensure the properties hold for all peer priorities
    for peer_priority in PeerPriority::get_all_ordered_priorities() {
        // Create the storage request
        let server_version_request =
            StorageServiceRequest::new(DataRequest::GetServerProtocolVersion, true);

        // Create the mock network and client
        let (mut mock_network, _, client, _) =
            MockNetwork::new(Some(base_config.clone()), Some(data_client_config), None);

        // Add several peers
        let num_peers = 50;
        let peers = utils::add_several_peers_with_metadata(
            &mut mock_network,
            &client,
            num_peers as u64,
            0,
            3,
            peer_priority,
        );

        // Remove the latency metadata for all except some peers
        let num_peers_with_latency_metadata = 2;
        let peers: Vec<_> = peers.into_iter().collect();
        for peer in peers[num_peers_with_latency_metadata..].iter() {
            utils::remove_latency_metadata(&client, *peer);
        }

        // Select peers to service the request multiple times
        let mut peers_and_selection_counts = HashMap::new();
        for _ in 0..NUM_SELECTION_ITERATIONS {
            // Select peers to service the request
            let selected_peers = client
                .choose_peers_for_request(&server_version_request)
                .unwrap();
            assert_eq!(selected_peers.len(), num_peers_for_multi_fetch);

            // Update the peer selection counts
            for selected_peer in selected_peers {
                *peers_and_selection_counts.entry(selected_peer).or_insert(0) += 1;
            }
        }

        // Build a max-heap of all peers by their selection counts
        let mut max_heap_selection_counts =
            utils::build_selection_count_max_heap(&peers_and_selection_counts);

        // Verify the top peers in the max-heap are the peers with latency metadata
        for _ in 0..num_peers_with_latency_metadata {
            let peer_monitoring_metadata = utils::get_peer_monitoring_metadata(
                &mut mock_network,
                max_heap_selection_counts.pop().unwrap().1,
            );
            assert!(peer_monitoring_metadata.average_ping_latency_secs.is_some())
        }

        // Verify the rest of the peers in the max-heap are the peers without latency metadata
        for _ in num_peers_with_latency_metadata..num_peers {
            // Get the peer monitoring metadata
            let peer_monitoring_metadata = utils::get_peer_monitoring_metadata(
                &mut mock_network,
                max_heap_selection_counts.pop().unwrap().1,
            );
            assert!(peer_monitoring_metadata.average_ping_latency_secs.is_none())
        }
    }
}

#[tokio::test]
async fn multi_fetch_request_selection_priority() {
    // Create a base config for a validator
    let base_config = utils::create_validator_base_config();

    // Create a data client with multi-fetch enabled (4 peers per request)
    let peers_for_multi_fetch = 4;
    let data_client_config = VelorDataClientConfig {
        data_multi_fetch_config: VelorDataMultiFetchConfig {
            enable_multi_fetch: true,
            min_peers_for_multi_fetch: peers_for_multi_fetch,
            max_peers_for_multi_fetch: peers_for_multi_fetch,
            ..Default::default()
        },
        ..Default::default()
    };

    // Create the mock network, time service and client
    let (mut mock_network, _, client, _) =
        MockNetwork::new(Some(base_config), Some(data_client_config), None);

    // Ensure the properties hold for all peer priorities (in increasing priority order)
    for peer_priority in PeerPriority::get_all_ordered_priorities().iter().rev() {
        // Create the storage request
        let server_version_request =
            StorageServiceRequest::new(DataRequest::GetServerProtocolVersion, true);

        // Add several peers
        let peers = utils::add_several_peers(&mut mock_network, 100, *peer_priority);

        // Verify multiple peers are selected
        utils::verify_selected_peers_from_set(
            &client,
            &server_version_request,
            peers_for_multi_fetch,
            &peers,
        );

        // Disconnect all peers
        utils::disconnect_all_peers(&mut mock_network, &peers);

        // Add a single peer
        let peer = mock_network.add_peer(*peer_priority);

        // Verify the peer is selected
        utils::verify_selected_peers_match(&client, hashset![peer], &server_version_request);

        // Disconnect the peer and verify the request is unserviceable
        mock_network.disconnect_peer(peer);
        utils::verify_request_is_unserviceable(&client, &server_version_request, true);
    }
}

#[tokio::test]
async fn multi_fetch_request_selection_priority_mix() {
    // Create a base config for a validator
    let base_config = utils::create_validator_base_config();

    // Create a data client with multi-fetch enabled (3 peers per request)
    let peers_for_multi_fetch = 3;
    let data_client_config = VelorDataClientConfig {
        data_multi_fetch_config: VelorDataMultiFetchConfig {
            enable_multi_fetch: true,
            min_peers_for_multi_fetch: 2,
            max_peers_for_multi_fetch: peers_for_multi_fetch,
            multi_fetch_peer_bucket_size: 3,
            ..Default::default()
        },
        ..Default::default()
    };

    // Create the mock network, time service and client
    let (mut mock_network, _, client, _) =
        MockNetwork::new(Some(base_config), Some(data_client_config), None);

    // Create the storage request
    let server_version_request =
        StorageServiceRequest::new(DataRequest::GetServerProtocolVersion, true);

    // Add several low priority peers
    let low_priority_peers =
        utils::add_several_peers(&mut mock_network, 100, PeerPriority::LowPriority);

    // Verify the low priority peers are selected
    utils::verify_selected_peers_from_set(
        &client,
        &server_version_request,
        peers_for_multi_fetch,
        &low_priority_peers,
    );

    // Add several medium priority peers
    let medium_priority_peers =
        utils::add_several_peers(&mut mock_network, 100, PeerPriority::MediumPriority);

    // Verify the medium priority peers are selected
    utils::verify_selected_peers_from_set(
        &client,
        &server_version_request,
        peers_for_multi_fetch,
        &medium_priority_peers,
    );

    // Add several high priority peers
    let high_priority_peers =
        utils::add_several_peers(&mut mock_network, 100, PeerPriority::HighPriority);

    // Verify the high priority peers are selected
    utils::verify_selected_peers_from_set(
        &client,
        &server_version_request,
        peers_for_multi_fetch,
        &high_priority_peers,
    );

    // Disconnect all high priority peers
    utils::disconnect_all_peers(&mut mock_network, &high_priority_peers);

    // Add a single high priority peer
    let high_priority_peer = mock_network.add_peer(PeerPriority::HighPriority);

    // Verify the high priority peer is selected (along side several medium priority peers)
    let selected_peers = client
        .choose_peers_for_request(&server_version_request)
        .unwrap();
    assert_eq!(selected_peers.len(), peers_for_multi_fetch);
    let selected_medium_priority_peers: HashSet<_> = selected_peers
        .difference(&hashset![high_priority_peer])
        .cloned()
        .collect();
    medium_priority_peers.is_superset(&selected_medium_priority_peers);

    // Disconnect all medium priority peers
    utils::disconnect_all_peers(&mut mock_network, &medium_priority_peers);

    // Make several requests and verify only the high priority peer is selected
    for _ in 0..100 {
        utils::verify_selected_peers_match(
            &client,
            hashset![high_priority_peer],
            &server_version_request,
        );
    }

    // Disconnect the high priority peer
    mock_network.disconnect_peer(high_priority_peer);

    // Make several requests and verify the low priority peers are selected
    for _ in 0..100 {
        utils::verify_selected_peers_from_set(
            &client,
            &server_version_request,
            peers_for_multi_fetch,
            &low_priority_peers,
        );
    }

    // Add two medium priority peers
    let medium_priority_peers = hashset![
        mock_network.add_peer(PeerPriority::MediumPriority),
        mock_network.add_peer(PeerPriority::MediumPriority)
    ];

    // Make several requests and verify the medium priority peers are selected
    for _ in 0..100 {
        utils::verify_selected_peers_match(
            &client,
            medium_priority_peers.clone(),
            &server_version_request,
        );
    }
}

#[tokio::test]
async fn multi_fetch_simple_peer_selection() {
    // Create a base config for a validator
    let base_config = utils::create_validator_base_config();

    // Create a data client config with multi-fetch enabled (2 -> 3 peers per request)
    let data_client_config = VelorDataClientConfig {
        data_multi_fetch_config: VelorDataMultiFetchConfig {
            enable_multi_fetch: true,
            min_peers_for_multi_fetch: 2,
            max_peers_for_multi_fetch: 3,
            ..Default::default()
        },
        ..Default::default()
    };

    // Create the mock network and client
    let (mut mock_network, _, client, _) =
        MockNetwork::new(Some(base_config), Some(data_client_config), None);

    // Create a storage request for transaction outputs
    let output_data_request =
        DataRequest::GetTransactionOutputsWithProof(TransactionOutputsWithProofRequest {
            proof_version: 100,
            start_version: 0,
            end_version: 100,
        });
    let storage_request = StorageServiceRequest::new(output_data_request, false);

    // Ensure no peers can service the request (we have no connections)
    utils::verify_request_is_unserviceable(&client, &storage_request, true);

    // Add a low priority peer and verify it cannot service the request (no advertised data)
    let low_priority_peer = mock_network.add_peer(PeerPriority::LowPriority);
    utils::verify_request_is_unserviceable(&client, &storage_request, false);

    // Add a medium priority peer and verify it cannot service the request (no advertised data)
    let medium_priority_peer = mock_network.add_peer(PeerPriority::MediumPriority);
    utils::verify_request_is_unserviceable(&client, &storage_request, false);

    // Add two high priority peers and verify they cannot service the request (no advertised data)
    let high_priority_peer_1 = mock_network.add_peer(PeerPriority::HighPriority);
    let high_priority_peer_2 = mock_network.add_peer(PeerPriority::HighPriority);
    utils::verify_request_is_unserviceable(&client, &storage_request, false);

    // Request data that is not being advertised (by anyone) and verify we get an error
    let output_data_request =
        DataRequest::GetTransactionOutputsWithProof(TransactionOutputsWithProofRequest {
            proof_version: 100,
            start_version: 0,
            end_version: 100,
        });
    let storage_request = StorageServiceRequest::new(output_data_request, false);
    utils::verify_request_is_unserviceable(&client, &storage_request, false);

    // Advertise the data for the low priority peer and verify it is now selected
    client.update_peer_storage_summary(low_priority_peer, utils::create_storage_summary(100));
    utils::verify_selected_peers_match(&client, hashset![low_priority_peer], &storage_request);

    // Advertise the data for high priority peer 2 and verify the peer is selected
    client.update_peer_storage_summary(high_priority_peer_2, utils::create_storage_summary(100));
    utils::verify_selected_peers_match(&client, hashset![high_priority_peer_2], &storage_request);

    // Reconnect high priority peer 1 and remove the advertised data for high priority peer 2
    mock_network.reconnect_peer(high_priority_peer_1);
    client.update_peer_storage_summary(high_priority_peer_2, utils::create_storage_summary(0));

    // Request the data again and verify the low priority peer is chosen
    utils::verify_selected_peers_match(&client, hashset![low_priority_peer], &storage_request);

    // Advertise the data for high priority peer 1 and verify the peer is selected
    client.update_peer_storage_summary(high_priority_peer_1, utils::create_storage_summary(100));
    utils::verify_selected_peers_match(&client, hashset![high_priority_peer_1], &storage_request);

    // Advertise the data for high priority peer 2 and verify both high priority peers are selected
    client.update_peer_storage_summary(high_priority_peer_2, utils::create_storage_summary(100));
    utils::verify_selected_peers_match(
        &client,
        hashset![high_priority_peer_1, high_priority_peer_2],
        &storage_request,
    );

    // Disconnect both high priority peers and verify the low priority peer is selected
    utils::disconnect_all_peers(&mut mock_network, &hashset![
        high_priority_peer_1,
        high_priority_peer_2
    ]);
    utils::verify_selected_peers_match(&client, hashset![low_priority_peer], &storage_request);

    // Advertise the data for the medium priority peer and verify the peer is selected
    client.update_peer_storage_summary(medium_priority_peer, utils::create_storage_summary(100));
    utils::verify_selected_peers_match(&client, hashset![medium_priority_peer], &storage_request);
}

#[tokio::test]
async fn multi_fetch_subscription_selection_priority() {
    // Create a base config for a VFN
    let base_config = utils::create_fullnode_base_config();
    let networks = vec![NetworkId::Vfn, NetworkId::Public];

    // Create a data client with a max lag of 10 and multi-fetch enabled (2 peers per request)
    let max_subscription_lag_secs = 10;
    let data_client_config = VelorDataClientConfig {
        data_multi_fetch_config: VelorDataMultiFetchConfig {
            enable_multi_fetch: true,
            min_peers_for_multi_fetch: 2,
            max_peers_for_multi_fetch: 2,
            ..Default::default()
        },
        max_subscription_lag_secs,
        ..Default::default()
    };

    // Create the mock network, time service and client
    let (mut mock_network, time_service, client, _) =
        MockNetwork::new(Some(base_config), Some(data_client_config), Some(networks));

    // Create test data
    let known_version = 1000;
    let known_epoch = 5;

    // Ensure the properties hold for all peer priorities (in increasing priority order)
    for peer_priority in PeerPriority::get_all_ordered_priorities().iter().rev() {
        // Ensure the properties hold for all subscription requests
        for data_request in utils::enumerate_subscription_requests(known_version, known_epoch) {
            // Create the storage request
            let storage_request = StorageServiceRequest::new(data_request, true);

            // Add several peers and verify the request is unserviceable
            let peers = utils::add_several_peers(&mut mock_network, 100, *peer_priority);
            utils::verify_request_is_unserviceable(&client, &storage_request, false);

            // Advertise the data for the peers and verify a single peer is selected
            let timestamp_usecs = time_service.now_unix_time().as_micros() as u64;
            for peer in peers.iter() {
                client.update_peer_storage_summary(
                    *peer,
                    utils::create_storage_summary_with_timestamp(known_version, timestamp_usecs),
                );
            }
            utils::verify_selected_peer_from_set(&client, &storage_request, &peers);

            // Disconnect all peers and verify the request is unserviceable
            utils::disconnect_all_peers(&mut mock_network, &HashSet::from_iter(peers));
            utils::verify_request_is_unserviceable(&client, &storage_request, true);

            // Add a single peer and advertise the data for the peer
            let peer = mock_network.add_peer(*peer_priority);
            client.update_peer_storage_summary(
                peer,
                utils::create_storage_summary_with_timestamp(known_version, timestamp_usecs),
            );

            // Verify the peer is not selected (the previous request went to a different peer)
            utils::verify_request_is_unserviceable(&client, &storage_request, false);

            // Update the stream ID and verify the peer is selected
            let storage_request = utils::update_subscription_request_id(&storage_request);
            utils::verify_selected_peers_match(&client, hashset![peer], &storage_request);

            // Disconnect the peer and verify the request is unserviceable
            mock_network.disconnect_peer(peer);
            utils::verify_request_is_unserviceable(&client, &storage_request, true);
        }
    }
}

#[tokio::test]
async fn multi_fetch_subscription_selection_priority_mix() {
    // Create a base config for a VFN
    let base_config = utils::create_fullnode_base_config();
    let networks = vec![NetworkId::Vfn, NetworkId::Public];

    // Create a data client with a max lag of 100 and multi-fetch enabled (2 peers per request)
    let max_subscription_lag_secs = 100;
    let data_client_config = VelorDataClientConfig {
        data_multi_fetch_config: VelorDataMultiFetchConfig {
            enable_multi_fetch: true,
            min_peers_for_multi_fetch: 2,
            max_peers_for_multi_fetch: 2,
            ..Default::default()
        },
        max_subscription_lag_secs,
        ..Default::default()
    };

    // Create the mock network, time service and client
    let (mut mock_network, time_service, client, _) =
        MockNetwork::new(Some(base_config), Some(data_client_config), Some(networks));

    // Create test data
    let known_version = 1000;
    let known_epoch = 5;

    // Ensure the properties hold for all subscription requests
    for data_request in utils::enumerate_subscription_requests(known_version, known_epoch) {
        // Create the storage request
        let mut storage_request = StorageServiceRequest::new(data_request, true);

        // Add several low priority peers and verify the request is unserviceable
        let low_priority_peers =
            utils::add_several_peers(&mut mock_network, 100, PeerPriority::LowPriority);
        utils::verify_request_is_unserviceable(&client, &storage_request, false);

        // Advertise the data for the low priority peers and verify a single peer is selected
        let timestamp_usecs = time_service.now_unix_time().as_micros() as u64;
        for peer in low_priority_peers.iter() {
            client.update_peer_storage_summary(
                *peer,
                utils::create_storage_summary_with_timestamp(known_version, timestamp_usecs),
            );
        }
        let selected_peers = client.choose_peers_for_request(&storage_request).unwrap();
        assert_eq!(selected_peers.len(), 1);
        assert!(low_priority_peers.contains(selected_peers.iter().next().unwrap()));

        // Add several medium priority peers and verify the request is still serviced by the same low priority peer
        let medium_priority_peers =
            utils::add_several_peers(&mut mock_network, 100, PeerPriority::MediumPriority);
        utils::verify_selected_peers_match(&client, selected_peers.clone(), &storage_request);

        // Advertise the data for the medium priority peers
        for peer in medium_priority_peers.iter() {
            client.update_peer_storage_summary(
                *peer,
                utils::create_storage_summary_with_timestamp(known_version, timestamp_usecs),
            );
        }

        // Update the subscription stream ID and verify a medium priority peer is selected
        storage_request = utils::update_subscription_request_id(&storage_request);
        let selected_peers = client.choose_peers_for_request(&storage_request).unwrap();
        assert!(medium_priority_peers.contains(selected_peers.iter().next().unwrap()));

        // Make several more requests and verify the same medium priority peer is selected
        for _ in 0..100 {
            utils::verify_selected_peers_match(&client, selected_peers.clone(), &storage_request);
        }

        // Update the stream request ID and verify medium priority peers are selected
        for _ in 0..100 {
            storage_request = utils::update_subscription_request_id(&storage_request);
            utils::verify_selected_peer_from_set(&client, &storage_request, &medium_priority_peers);
        }

        // Add several high priority peers and verify the request is still serviced by the same medium priority peer
        let high_priority_peers =
            utils::add_several_peers(&mut mock_network, 100, PeerPriority::HighPriority);
        utils::verify_selected_peers_match(&client, selected_peers, &storage_request);

        // Advertise the data for the high priority peers
        for peer in high_priority_peers.iter() {
            client.update_peer_storage_summary(
                *peer,
                utils::create_storage_summary_with_timestamp(known_version, timestamp_usecs),
            );
        }

        // Verify the next request is unserviceable (the previous request went to a medium priority peer)
        utils::verify_request_is_unserviceable(&client, &storage_request, false);

        // Make several requests and verify the high priority peers are selected
        for _ in 0..100 {
            storage_request = utils::update_subscription_request_id(&storage_request);
            utils::verify_selected_peer_from_set(&client, &storage_request, &high_priority_peers);
        }

        // Disconnect all high priority peers
        utils::disconnect_all_peers(&mut mock_network, &HashSet::from_iter(high_priority_peers));

        // Make several requests and verify the medium priority peers are selected
        for _ in 0..100 {
            storage_request = utils::update_subscription_request_id(&storage_request);
            utils::verify_selected_peer_from_set(&client, &storage_request, &medium_priority_peers);
        }

        // Disconnect all medium priority peers
        utils::disconnect_all_peers(
            &mut mock_network,
            &HashSet::from_iter(medium_priority_peers),
        );

        // Make several requests and verify the low priority peers are selected
        for _ in 0..100 {
            storage_request = utils::update_subscription_request_id(&storage_request);
            utils::verify_selected_peer_from_set(&client, &storage_request, &low_priority_peers);
        }

        // Disconnect all low priority peers and verify the request is unserviceable
        utils::disconnect_all_peers(&mut mock_network, &HashSet::from_iter(low_priority_peers));
        utils::verify_request_is_unserviceable(&client, &storage_request, true);
    }
}

#[tokio::test]
async fn multi_fetch_trivial_serviceability() {
    // Create a base config for a VFN
    let base_config = utils::create_fullnode_base_config();
    let networks = vec![NetworkId::Vfn, NetworkId::Public];

    // Create a data client config with multi-fetch enabled (2 peers per request)
    let data_client_config = VelorDataClientConfig {
        data_multi_fetch_config: VelorDataMultiFetchConfig {
            enable_multi_fetch: true,
            min_peers_for_multi_fetch: 2,
            max_peers_for_multi_fetch: 2,
            ..Default::default()
        },
        ..Default::default()
    };

    // Create the mock network and client
    let (mut mock_network, _, client, _) =
        MockNetwork::new(Some(base_config), Some(data_client_config), Some(networks));

    // Create a server version request that is trivially serviceable
    let server_version_request =
        StorageServiceRequest::new(DataRequest::GetServerProtocolVersion, true);

    // Ensure no peers can service the request (we have no connections)
    utils::verify_request_is_unserviceable(&client, &server_version_request, true);

    // Add a low priority peer and verify the peer is selected as the request recipient
    let low_priority_peer_1 = mock_network.add_peer(PeerPriority::LowPriority);
    utils::verify_selected_peers_match(
        &client,
        hashset![low_priority_peer_1],
        &server_version_request,
    );

    // Add a medium priority peer and verify the peer is selected as the recipient.
    // (Low priority peers are ignored if there are higher priority peers available).
    let medium_priority_peer_1 = mock_network.add_peer(PeerPriority::MediumPriority);
    utils::verify_selected_peers_match(
        &client,
        hashset![medium_priority_peer_1],
        &server_version_request,
    );

    // Add another medium priority peer and verify both medium priority peers are selected
    let medium_priority_peer_2 = mock_network.add_peer(PeerPriority::MediumPriority);
    utils::verify_selected_peers_match(
        &client,
        hashset![medium_priority_peer_1, medium_priority_peer_2],
        &server_version_request,
    );

    // Add two high priority peers
    let high_priority_peer_1 = mock_network.add_peer(PeerPriority::HighPriority);
    let high_priority_peer_2 = mock_network.add_peer(PeerPriority::HighPriority);

    // Verify both high priority peers are selected
    utils::verify_selected_peers_match(
        &client,
        hashset![high_priority_peer_1, high_priority_peer_2],
        &server_version_request,
    );

    // Disconnect the first high priority peer
    mock_network.disconnect_peer(high_priority_peer_1);

    // Verify the other high priority peer is selected (alongside one medium priority peer)
    let selected_peers = client
        .choose_peers_for_request(&server_version_request)
        .unwrap();
    assert_eq!(selected_peers.len(), 2);
    assert!(selected_peers.contains(&high_priority_peer_2));
    assert!(
        selected_peers.contains(&medium_priority_peer_1)
            || selected_peers.contains(&medium_priority_peer_2)
    );

    // Disconnect the second high priority peer
    mock_network.disconnect_peer(high_priority_peer_2);

    // Verify both medium priority peers are selected
    utils::verify_selected_peers_match(
        &client,
        hashset![medium_priority_peer_1, medium_priority_peer_2],
        &server_version_request,
    );

    // Disconnect the first medium priority peer and reconnect the first high priority peer
    mock_network.disconnect_peer(medium_priority_peer_1);
    mock_network.reconnect_peer(high_priority_peer_1);

    // Verify the first high priority peer is selected (alongside the second medium priority peer)
    utils::verify_selected_peers_match(
        &client,
        hashset![high_priority_peer_1, medium_priority_peer_2],
        &server_version_request,
    );

    // Disconnect the second medium priority peer and verify the first high priority peer is selected
    mock_network.disconnect_peer(medium_priority_peer_2);
    utils::verify_selected_peers_match(
        &client,
        hashset![high_priority_peer_1],
        &server_version_request,
    );

    // Disconnect the first high priority peer and verify the low priority peer is selected
    mock_network.disconnect_peer(high_priority_peer_1);
    utils::verify_selected_peers_match(
        &client,
        hashset![low_priority_peer_1],
        &server_version_request,
    );

    // Add another low priority peer and verify both low priority peers are selected
    let low_priority_peer_2 = mock_network.add_peer(PeerPriority::LowPriority);
    utils::verify_selected_peers_match(
        &client,
        hashset![low_priority_peer_1, low_priority_peer_2],
        &server_version_request,
    );

    // Add another low priority peer and verify only two low priority peers are selected
    let low_priority_peer_3 = mock_network.add_peer(PeerPriority::LowPriority);
    let selected_peers = client
        .choose_peers_for_request(&server_version_request)
        .unwrap();
    assert_eq!(selected_peers.len(), 2);

    // Verify that two low priority peers are selected
    let all_low_priority_peers = hashset![
        low_priority_peer_1,
        low_priority_peer_2,
        low_priority_peer_3
    ];
    let difference: HashSet<_> = all_low_priority_peers.difference(&selected_peers).collect();
    assert_eq!(difference.len(), 1);
}

#[tokio::test]
async fn multi_fetch_trivial_serviceability_pfn() {
    // Create a base config for a PFN
    let base_config = utils::create_fullnode_base_config();
    let networks = vec![NetworkId::Public];

    // Create a data client config with multi-fetch enabled (2 peers per request)
    let data_client_config = VelorDataClientConfig {
        data_multi_fetch_config: VelorDataMultiFetchConfig {
            enable_multi_fetch: true,
            min_peers_for_multi_fetch: 2,
            max_peers_for_multi_fetch: 2,
            ..Default::default()
        },
        ..Default::default()
    };

    // Create the mock network and client
    let (mut mock_network, _, client, _) =
        MockNetwork::new(Some(base_config), Some(data_client_config), Some(networks));

    // Create a server version request that is trivially serviceable
    let server_version_request =
        StorageServiceRequest::new(DataRequest::GetServerProtocolVersion, true);

    // Add a low priority peer and verify the peer is selected as the request recipient
    let low_priority_peer_1 = mock_network.add_peer(PeerPriority::LowPriority);
    utils::verify_selected_peers_match(
        &client,
        hashset![low_priority_peer_1],
        &server_version_request,
    );

    // Add a high priority peer and verify the peer is selected as the request recipient
    let high_priority_peer_1 = mock_network.add_peer(PeerPriority::HighPriority);
    utils::verify_selected_peers_match(
        &client,
        hashset![high_priority_peer_1],
        &server_version_request,
    );

    // Add another high priority peer and verify both high priority peers are selected
    let high_priority_peer_2 = mock_network.add_peer(PeerPriority::HighPriority);
    utils::verify_selected_peers_match(
        &client,
        hashset![high_priority_peer_1, high_priority_peer_2],
        &server_version_request,
    );

    // Disconnect the first high priority peer
    mock_network.disconnect_peer(high_priority_peer_1);

    // Verify the other high priority peer is selected
    utils::verify_selected_peers_match(
        &client,
        hashset![high_priority_peer_2],
        &server_version_request,
    );

    // Disconnect the second high priority peer
    mock_network.disconnect_peer(high_priority_peer_2);

    // Verify the low priority peer is selected
    utils::verify_selected_peers_match(
        &client,
        hashset![low_priority_peer_1],
        &server_version_request,
    );

    // Add another low priority peer and verify both low priority peers are selected
    let low_priority_peer_2 = mock_network.add_peer(PeerPriority::LowPriority);
    utils::verify_selected_peers_match(
        &client,
        hashset![low_priority_peer_1, low_priority_peer_2],
        &server_version_request,
    );

    // Add another low priority peer and verify only two low priority peers are selected
    let low_priority_peer_3 = mock_network.add_peer(PeerPriority::LowPriority);
    let selected_peers = client
        .choose_peers_for_request(&server_version_request)
        .unwrap();
    assert_eq!(selected_peers.len(), 2);

    // Verify that two low priority peers are selected
    let all_low_priority_peers = hashset![
        low_priority_peer_1,
        low_priority_peer_2,
        low_priority_peer_3
    ];
    let difference: HashSet<_> = all_low_priority_peers.difference(&selected_peers).collect();
    assert_eq!(difference.len(), 1);
}

/// Verifies that the number of selected peers for the
/// given request matches expectations.
fn verify_num_selected_peers(
    client: &VelorDataClient,
    storage_request: &StorageServiceRequest,
    num_expected_peers: usize,
) {
    let selected_peers = client.choose_peers_for_request(storage_request).unwrap();
    assert_eq!(selected_peers.len(), num_expected_peers);
}

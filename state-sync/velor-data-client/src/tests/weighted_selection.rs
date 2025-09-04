// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    client::VelorDataClient,
    priority::PeerPriority,
    tests::{mock::MockNetwork, utils, utils::NUM_SELECTION_ITERATIONS},
};
use velor_config::{
    config::{VelorDataClientConfig, VelorDataMultiFetchConfig, VelorLatencyFilteringConfig},
    network_id::{NetworkId, PeerNetworkId},
};
use velor_storage_service_types::requests::{DataRequest, StorageServiceRequest};
use velor_time_service::TimeServiceTrait;
use maplit::hashset;
use ordered_float::OrderedFloat;
use rand::Rng;
use std::collections::{HashMap, HashSet};

// Useful test constants
const NUM_PEERS_TO_ADD: u64 = 50;

#[tokio::test]
async fn optimistic_fetch_distance_latency_weights() {
    // Create the data client config with a max lag of 5 and multi-fetch enabled
    let max_optimistic_fetch_lag_secs = 5;
    let num_peers_for_multi_fetch = 3;
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
    let max_validator_distance = 2;

    // Ensure the properties hold for all peer priorities
    for peer_priority in PeerPriority::get_all_ordered_priorities() {
        // Ensure the properties hold for all optimistic fetch requests
        for data_request in utils::enumerate_optimistic_fetch_requests(known_version, known_epoch) {
            // Create the storage request
            let storage_request = StorageServiceRequest::new(data_request, true);

            // Create the mock network, time service and client
            let (mut mock_network, time_service, client, _) =
                MockNetwork::new(None, Some(data_client_config), None);

            // Add several peers with metadata
            let peers = utils::add_several_peers_with_metadata(
                &mut mock_network,
                &client,
                NUM_PEERS_TO_ADD,
                min_validator_distance,
                max_validator_distance,
                peer_priority,
            );

            // Verify none of the peers can service the request
            utils::verify_request_is_unserviceable(&client, &storage_request, false);

            // Advertise the data for the peers
            utils::update_storage_summaries_for_peers(
                &client,
                &peers,
                known_version,
                time_service.now_unix_time().as_micros(),
            );

            // Select peers to service the request multiple times
            let mut peers_and_selection_counts = utils::select_peers_multiple_times(
                &client,
                num_peers_for_multi_fetch,
                &storage_request,
            );

            // Verify all of the selected peers have the lowest distance
            verify_lowest_distance_from_validators(
                &mut mock_network,
                &mut peers_and_selection_counts,
                min_validator_distance,
            );

            // Verify the highest selected peers are the lowest latency peers
            utils::verify_highest_peer_selection_latencies(
                &mut mock_network,
                &mut peers_and_selection_counts,
            );
        }
    }
}

#[tokio::test]
async fn optimistic_fetch_missing_distances() {
    // Create a base config for a validator
    let base_config = utils::create_validator_base_config();

    // Create the data client config with a max lag of 5 and multi-fetch enabled
    let max_optimistic_fetch_lag_secs = 5;
    let num_peers_for_multi_fetch = 4;
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
    let validator_distance = 2;

    // Ensure the properties hold for all peer priorities
    for peer_priority in PeerPriority::get_all_ordered_priorities() {
        // Ensure the properties hold for all optimistic fetch requests
        for data_request in utils::enumerate_optimistic_fetch_requests(known_version, known_epoch) {
            // Create the storage request
            let storage_request = StorageServiceRequest::new(data_request, true);

            // Create the mock network and client
            let (mut mock_network, time_service, client, _) =
                MockNetwork::new(Some(base_config.clone()), Some(data_client_config), None);

            // Add several peers with metadata
            let peers = utils::add_several_peers_with_metadata(
                &mut mock_network,
                &client,
                NUM_PEERS_TO_ADD,
                validator_distance,
                validator_distance,
                peer_priority,
            );

            // Remove the distance metadata for some peers
            let peers_with_missing_distances =
                remove_metadata_for_several_peers(&client, &peers, false);

            // Advertise the data for the peers
            utils::update_storage_summaries_for_peers(
                &client,
                &HashSet::from_iter(peers),
                known_version,
                time_service.now_unix_time().as_micros(),
            );

            // Select peers to service the request multiple times
            let mut peers_and_selection_counts = utils::select_peers_multiple_times(
                &client,
                num_peers_for_multi_fetch,
                &storage_request,
            );

            // Verify all of the selected peers have the lowest distance
            verify_lowest_distance_from_validators(
                &mut mock_network,
                &mut peers_and_selection_counts,
                validator_distance,
            );

            // Verify the highest selected peers are the lowest latency peers
            utils::verify_highest_peer_selection_latencies(
                &mut mock_network,
                &mut peers_and_selection_counts,
            );

            // Verify that the peers with missing distances are not selected
            verify_zero_selection_counts(
                &peers_with_missing_distances,
                &peers_and_selection_counts,
            );
        }
    }
}

#[tokio::test]
async fn optimistic_fetch_no_distances() {
    // Create a base config for a validator
    let base_config = utils::create_validator_base_config();

    // Create the data client config with a max lag of 5 and multi-fetch enabled
    let max_optimistic_fetch_lag_secs = 5;
    let num_peers_for_multi_fetch = 2;
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
            // Create the storage request
            let storage_request = StorageServiceRequest::new(data_request, true);

            // Create the mock network and client
            let (mut mock_network, time_service, client, _) =
                MockNetwork::new(Some(base_config.clone()), Some(data_client_config), None);

            // Add several peers with metadata
            let peers = utils::add_several_peers_with_metadata(
                &mut mock_network,
                &client,
                NUM_PEERS_TO_ADD,
                min_validator_distance,
                max_validator_distance,
                peer_priority,
            );

            // Remove the distance metadata for all peers
            for peer in peers.iter() {
                utils::remove_distance_metadata(&client, *peer);
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

            // Verify that peers are still selected even though there are no recorded distances
            verify_all_peers_are_selected(peers, peers_and_selection_counts);
        }
    }
}

#[tokio::test]
async fn optimistic_fetch_missing_latencies() {
    // Create a base config for a validator
    let base_config = utils::create_validator_base_config();

    // Create the data client config with a max lag of 5 and multi-fetch enabled
    let max_optimistic_fetch_lag_secs = 5;
    let num_peers_for_multi_fetch = 3;
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
    let validator_distance = 1;

    // Ensure the properties hold for all peer priorities
    for peer_priority in PeerPriority::get_all_ordered_priorities() {
        // Ensure the properties hold for all optimistic fetch requests
        for data_request in utils::enumerate_optimistic_fetch_requests(known_version, known_epoch) {
            // Create the storage request
            let storage_request = StorageServiceRequest::new(data_request, true);

            // Create the mock network and client
            let (mut mock_network, time_service, client, _) =
                MockNetwork::new(Some(base_config.clone()), Some(data_client_config), None);

            // Add several peers with metadata
            let peers = utils::add_several_peers_with_metadata(
                &mut mock_network,
                &client,
                NUM_PEERS_TO_ADD,
                validator_distance,
                validator_distance,
                peer_priority,
            );

            // Remove the latency metadata for some peers
            let peers_with_missing_latencies =
                remove_metadata_for_several_peers(&client, &peers, true);

            // Advertise the data for the peers
            utils::update_storage_summaries_for_peers(
                &client,
                &HashSet::from_iter(peers),
                known_version,
                time_service.now_unix_time().as_micros(),
            );

            // Select peers to service the request multiple times
            let mut peers_and_selection_counts = utils::select_peers_multiple_times(
                &client,
                num_peers_for_multi_fetch,
                &storage_request,
            );

            // Verify all of the selected peers have the lowest distance
            verify_lowest_distance_from_validators(
                &mut mock_network,
                &mut peers_and_selection_counts,
                validator_distance,
            );

            // Verify the highest selected peers are the lowest latency peers
            utils::verify_highest_peer_selection_latencies(
                &mut mock_network,
                &mut peers_and_selection_counts,
            );

            // Verify that the peers with missing latencies are not selected
            verify_zero_selection_counts(
                &peers_with_missing_latencies,
                &peers_and_selection_counts,
            );
        }
    }
}

#[tokio::test]
async fn optimistic_fetch_no_latencies() {
    // Create a base config for a validator
    let base_config = utils::create_validator_base_config();

    // Create the data client config with a max lag of 5 and multi-fetch enabled
    let max_optimistic_fetch_lag_secs = 5;
    let num_peers_for_multi_fetch = 3;
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
            // Create the storage request
            let storage_request = StorageServiceRequest::new(data_request, true);

            // Create the mock network and client
            let (mut mock_network, time_service, client, _) =
                MockNetwork::new(Some(base_config.clone()), Some(data_client_config), None);

            // Add several peers and remove their latency metadata
            let mut peers = vec![];
            for _ in 0..NUM_PEERS_TO_ADD {
                // Add a peer
                let peer = mock_network.add_peer(peer_priority);
                peers.push(peer);

                // Generate a random distance for the peer and update the peer's distance metadata
                let distance_from_validator =
                    rand::thread_rng().gen_range(min_validator_distance..=max_validator_distance);
                utils::update_distance_metadata(&client, peer, distance_from_validator as u64);

                // Remove the latency metadata for the peer
                utils::remove_latency_metadata(&client, peer);
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

            // Verify that peers are still selected even though there are no recorded latencies
            verify_all_peers_are_selected(HashSet::from_iter(peers), peers_and_selection_counts);
        }
    }
}

#[tokio::test]
async fn request_latency_filtering() {
    // Create a base config for a validator
    let base_config = utils::create_validator_base_config();

    // Create the data client config with latency filtering and multi-fetch enabled
    let num_peers_for_multi_fetch = 3;
    let min_peers_for_latency_filtering = 100;
    let latency_filtering_reduction_factor = 2;
    let data_client_config = VelorDataClientConfig {
        data_multi_fetch_config: VelorDataMultiFetchConfig {
            enable_multi_fetch: true,
            min_peers_for_multi_fetch: 1,
            max_peers_for_multi_fetch: num_peers_for_multi_fetch,
            ..Default::default()
        },
        latency_filtering_config: VelorLatencyFilteringConfig {
            min_peers_for_latency_filtering,
            latency_filtering_reduction_factor,
            ..Default::default()
        },
        ..Default::default()
    };

    // Create the mock network and client
    let (mut mock_network, _, client, _) =
        MockNetwork::new(Some(base_config.clone()), Some(data_client_config), None);

    // Ensure the properties hold for all peer priorities (in ascending order)
    for peer_priority in PeerPriority::get_all_ordered_priorities().iter().rev() {
        // Create the data request
        let data_request = DataRequest::GetStorageServerSummary;
        let storage_request = StorageServiceRequest::new(data_request, true);

        // Add several peers (enough to trigger latency filtering)
        let peers = utils::add_several_peers(
            &mut mock_network,
            min_peers_for_latency_filtering + 10,
            *peer_priority,
        );

        // Select peers to service the request multiple times
        let mut peers_and_selection_counts = utils::select_peers_multiple_times(
            &client,
            num_peers_for_multi_fetch,
            &storage_request,
        );

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
        let peers_to_verify = peers.len() / (latency_filtering_reduction_factor as usize);
        for (peer, _) in peers_and_latencies[0..peers_to_verify].iter() {
            match peers_and_selection_counts.get(peer) {
                Some(selection_count) => assert!(*selection_count > 0),
                None => panic!("Peer {:?} was not found in the selection counts!", peer),
            }
        }

        // Verify that the bottom subset of peers do not have
        // selection counts (as they were filtered out).
        let peers_with_zero_selection_counts = peers_and_latencies[peers_to_verify..]
            .iter()
            .map(|(peer, _)| *peer)
            .collect();
        verify_zero_selection_counts(
            &peers_with_zero_selection_counts,
            &peers_and_selection_counts,
        );
    }
}

#[tokio::test]
async fn request_latency_filtering_ratio() {
    // Create a base config for a validatorpeers_with_zero_selection_counts
    let base_config = utils::create_validator_base_config();

    // Create the data client config with latency filtering and multi-fetch enabled
    let num_peers_for_multi_fetch = 5;
    let min_peers_for_latency_filtering = 50;
    let min_peer_ratio_for_latency_filtering = 10_000; // Set to a very high value
    let latency_filtering_reduction_factor = 2;
    let data_client_config = VelorDataClientConfig {
        data_multi_fetch_config: VelorDataMultiFetchConfig {
            enable_multi_fetch: true,
            min_peers_for_multi_fetch: num_peers_for_multi_fetch,
            max_peers_for_multi_fetch: num_peers_for_multi_fetch,
            ..Default::default()
        },
        latency_filtering_config: VelorLatencyFilteringConfig {
            min_peers_for_latency_filtering,
            min_peer_ratio_for_latency_filtering,
            latency_filtering_reduction_factor,
        },
        ..Default::default()
    };

    // Create the mock network and client
    let (mut mock_network, _, client, _) =
        MockNetwork::new(Some(base_config.clone()), Some(data_client_config), None);

    // Ensure the properties hold for all peer priorities (in ascending order)
    for peer_priority in PeerPriority::get_all_ordered_priorities().iter().rev() {
        // Create the data request
        let data_request = DataRequest::GetStorageServerSummary;
        let storage_request = StorageServiceRequest::new(data_request, true);

        // Add several peers (enough to satisfy the minimum number of peers)
        let peers = utils::add_several_peers(
            &mut mock_network,
            min_peers_for_latency_filtering * 2,
            *peer_priority,
        );

        // Select peers to service the request multiple times
        let mut peers_and_selection_counts = utils::select_peers_multiple_times(
            &client,
            num_peers_for_multi_fetch,
            &storage_request,
        );

        // Verify the highest selected peers are the lowest latency peers
        utils::verify_highest_peer_selection_latencies(
            &mut mock_network,
            &mut peers_and_selection_counts,
        );

        // Verify that the number of selected peers is more than
        // half the total peers (as filtering was disabled).
        let num_filtered_peers = peers.len() / latency_filtering_reduction_factor as usize;
        assert!(peers_and_selection_counts.len() > num_filtered_peers);
    }
}

#[tokio::test]
async fn request_latency_selection() {
    // Create a base config for a validator
    let base_config = utils::create_validator_base_config();

    // Create the data client config with latency filtering and multi-fetch enabled
    let num_peers_for_multi_fetch = 1;
    let min_peers_for_latency_filtering = 50;
    let latency_filtering_reduction_factor = 2;
    let data_client_config = VelorDataClientConfig {
        data_multi_fetch_config: VelorDataMultiFetchConfig {
            enable_multi_fetch: true,
            min_peers_for_multi_fetch: num_peers_for_multi_fetch,
            max_peers_for_multi_fetch: num_peers_for_multi_fetch,
            ..Default::default()
        },
        latency_filtering_config: VelorLatencyFilteringConfig {
            min_peers_for_latency_filtering,
            latency_filtering_reduction_factor,
            ..Default::default()
        },
        ..Default::default()
    };

    // Ensure the properties hold for all peer priorities
    for peer_priority in PeerPriority::get_all_ordered_priorities() {
        // Create the mock network and client
        let (mut mock_network, _, client, _) =
            MockNetwork::new(Some(base_config.clone()), Some(data_client_config), None);

        // Create the data request
        let data_request = DataRequest::GetStorageServerSummary;
        let storage_request = StorageServiceRequest::new(data_request, true);

        // Add several peers (but not enough to trigger latency filtering)
        let peers = utils::add_several_peers(
            &mut mock_network,
            min_peers_for_latency_filtering - 1,
            peer_priority,
        );

        // Select peers to service the request multiple times
        let mut peers_and_selection_counts = utils::select_peers_multiple_times(
            &client,
            num_peers_for_multi_fetch,
            &storage_request,
        );

        // Verify the highest selected peers are the lowest latency peers
        utils::verify_highest_peer_selection_latencies(
            &mut mock_network,
            &mut peers_and_selection_counts,
        );

        // Verify that the number of selected peers is more than
        // half the total peers (as filtering was disabled).
        let num_filtered_peers = peers.len() / (latency_filtering_reduction_factor as usize);
        assert!(peers_and_selection_counts.len() > num_filtered_peers);
    }
}

#[tokio::test]
async fn request_missing_latencies() {
    // Create a base config for a VFN
    let base_config = utils::create_fullnode_base_config();
    let networks = vec![NetworkId::Vfn, NetworkId::Public];

    // Create the data client config with latency filtering and multi-fetch enabled
    let num_peers_for_multi_fetch = 2;
    let min_peers_for_latency_filtering = 50;
    let data_client_config = VelorDataClientConfig {
        data_multi_fetch_config: VelorDataMultiFetchConfig {
            enable_multi_fetch: true,
            min_peers_for_multi_fetch: num_peers_for_multi_fetch,
            max_peers_for_multi_fetch: num_peers_for_multi_fetch,
            ..Default::default()
        },
        latency_filtering_config: VelorLatencyFilteringConfig {
            min_peers_for_latency_filtering,
            ..Default::default()
        },
        ..Default::default()
    };

    // Create the mock network and client
    let (mut mock_network, _, client, _) = MockNetwork::new(
        Some(base_config.clone()),
        Some(data_client_config),
        Some(networks.clone()),
    );

    // Ensure the properties hold for all peer priorities (in ascending order)
    for peer_priority in PeerPriority::get_all_ordered_priorities().iter().rev() {
        // Create the data request
        let data_request = DataRequest::GetStorageServerSummary;
        let storage_request = StorageServiceRequest::new(data_request, true);

        // Add several peers
        let peers = utils::add_several_peers(
            &mut mock_network,
            min_peers_for_latency_filtering + 10,
            *peer_priority,
        );

        // Remove the latency metadata for some peers
        let peers_with_missing_latencies = remove_metadata_for_several_peers(&client, &peers, true);

        // Select peers to service the request multiple times
        let mut peers_and_selection_counts = utils::select_peers_multiple_times(
            &client,
            num_peers_for_multi_fetch,
            &storage_request,
        );

        // Verify the highest selected peers are the lowest latency peers
        utils::verify_highest_peer_selection_latencies(
            &mut mock_network,
            &mut peers_and_selection_counts,
        );

        // Verify that the peers with missing latencies are not selected
        verify_zero_selection_counts(&peers_with_missing_latencies, &peers_and_selection_counts);
    }
}

#[tokio::test]
async fn request_no_latencies() {
    // Create a base config for a VFN
    let base_config = utils::create_fullnode_base_config();
    let networks = vec![NetworkId::Vfn, NetworkId::Public];

    // Create the data client config with latency filtering and multi-fetch enabled
    let num_peers_for_multi_fetch = 2;
    let min_peers_for_latency_filtering = 50;
    let data_client_config = VelorDataClientConfig {
        data_multi_fetch_config: VelorDataMultiFetchConfig {
            enable_multi_fetch: true,
            min_peers_for_multi_fetch: 1,
            max_peers_for_multi_fetch: num_peers_for_multi_fetch,
            multi_fetch_peer_bucket_size: 1,
            ..Default::default()
        },
        latency_filtering_config: VelorLatencyFilteringConfig {
            min_peers_for_latency_filtering,
            ..Default::default()
        },
        ..Default::default()
    };

    // Create the mock network and client
    let (mut mock_network, _, client, _) = MockNetwork::new(
        Some(base_config.clone()),
        Some(data_client_config),
        Some(networks.clone()),
    );

    // Ensure the properties hold for all peer priorities (in ascending order)
    for peer_priority in PeerPriority::get_all_ordered_priorities().iter().rev() {
        // Create the data request
        let data_request = DataRequest::GetStorageServerSummary;
        let storage_request = StorageServiceRequest::new(data_request, true);

        // Add several peers and remove their latency metadata
        let mut peers = hashset![];
        for _ in 0..min_peers_for_latency_filtering + 10 {
            // Add a peer
            let peer = mock_network.add_peer(*peer_priority);
            peers.insert(peer);

            // Remove the latency metadata for the peer
            utils::remove_latency_metadata(&client, peer)
        }

        // Select peers to service the request multiple times
        let peers_and_selection_counts = utils::select_peers_multiple_times(
            &client,
            num_peers_for_multi_fetch,
            &storage_request,
        );

        // Verify that peers are still selected even though there are no recorded latencies
        verify_all_peers_are_selected(peers, peers_and_selection_counts);
    }
}

#[tokio::test]
async fn subscription_distance_latency_weights() {
    // Create a data client with a max lag of 500
    let max_subscription_lag_secs = 500;
    let data_client_config = VelorDataClientConfig {
        max_subscription_lag_secs,
        ..Default::default()
    };

    // Create test data
    let known_version = 1;
    let known_epoch = 1;
    let min_validator_distance = 1;
    let max_validator_distance = 3;

    // Ensure the properties hold for all peer priorities
    for peer_priority in PeerPriority::get_all_ordered_priorities() {
        // Ensure the properties hold for all subscription requests
        for data_request in utils::enumerate_subscription_requests(known_version, known_epoch) {
            // Create the storage request
            let mut storage_request = StorageServiceRequest::new(data_request, true);

            // Create the mock network, time service and client
            let (mut mock_network, time_service, client, _) =
                MockNetwork::new(None, Some(data_client_config), None);

            // Add several peers with metadata
            let peers = utils::add_several_peers_with_metadata(
                &mut mock_network,
                &client,
                NUM_PEERS_TO_ADD,
                min_validator_distance,
                max_validator_distance,
                peer_priority,
            );

            // Verify none of the peers can service the request
            utils::verify_request_is_unserviceable(&client, &storage_request, false);

            // Advertise the data for the peers
            utils::update_storage_summaries_for_peers(
                &client,
                &peers,
                known_version,
                time_service.now_unix_time().as_micros(),
            );

            // Select a peer to service the request multiple times
            let mut peers_and_selection_counts = HashMap::new();
            for _ in 0..NUM_SELECTION_ITERATIONS {
                // Select a peer to service the request
                let selected_peers = client.choose_peers_for_request(&storage_request).unwrap();
                assert_eq!(selected_peers.len(), 1);

                // Update the peer selection counts
                for selected_peer in selected_peers {
                    *peers_and_selection_counts.entry(selected_peer).or_insert(0) += 1;
                }

                // Update the request's subscription ID
                storage_request = utils::update_subscription_request_id(&storage_request);
            }

            // Verify all of the selected peers have the lowest distance
            verify_lowest_distance_from_validators(
                &mut mock_network,
                &mut peers_and_selection_counts,
                min_validator_distance,
            );

            // Verify the highest selected peers are the lowest latency peers
            utils::verify_highest_peer_selection_latencies(
                &mut mock_network,
                &mut peers_and_selection_counts,
            );
        }
    }
}

/// Removes metadata for several peers and returns the set of peers with missing metadata.
/// If `remove_latency_metadata` is true, then the latency metadata is removed. Otherwise,
/// distance metadata is removed.
fn remove_metadata_for_several_peers(
    client: &VelorDataClient,
    peers: &HashSet<PeerNetworkId>,
    remove_latency_metadata: bool,
) -> Vec<PeerNetworkId> {
    // Remove 1/3 of the peers' appropriate metadata
    let num_peers_with_missing_metadata = peers.len() / 3;
    let mut peers_with_missing_metadata = vec![];

    // Remove the metadata for some peers
    let peers: Vec<_> = peers.iter().cloned().collect();
    for peer in peers[0..num_peers_with_missing_metadata].iter() {
        // Remove the appropriate metadata for the peer
        if remove_latency_metadata {
            utils::remove_latency_metadata(client, *peer);
        } else {
            utils::remove_distance_metadata(client, *peer);
        }

        // Add the peer to the set of peers with missing metadata
        peers_with_missing_metadata.push(*peer);
    }

    peers_with_missing_metadata
}

/// Verifies all peers are selected at least once
fn verify_all_peers_are_selected(
    peers: HashSet<PeerNetworkId>,
    peers_and_selection_counts: HashMap<PeerNetworkId, i32>,
) {
    for peer in peers {
        match peers_and_selection_counts.get(&peer) {
            Some(selection_count) => assert!(*selection_count > 0),
            None => panic!("Peer {:?} was not found in the selection counts!", peer),
        }
    }
}

/// Verifies that all of the selected peers have the lowest distance from the validators
fn verify_lowest_distance_from_validators(
    mock_network: &mut MockNetwork,
    peers_and_selection_counts: &mut HashMap<PeerNetworkId, i32>,
    min_validator_distance: u64,
) {
    for peer in peers_and_selection_counts.keys() {
        let distance_from_validator = utils::get_peer_distance_from_validators(mock_network, *peer);
        assert_eq!(distance_from_validator, min_validator_distance);
    }
}

/// Verifies that all of the specified peers have zero selection counts
fn verify_zero_selection_counts(
    peers_with_zero_counts: &Vec<PeerNetworkId>,
    peers_and_selection_counts: &HashMap<PeerNetworkId, i32>,
) {
    for peer in peers_with_zero_counts {
        if let Some(selection_count) = peers_and_selection_counts.get(peer) {
            assert_eq!(*selection_count, 0);
        }
    }
}

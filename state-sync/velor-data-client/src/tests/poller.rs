// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::Error,
    poller,
    poller::DataSummaryPoller,
    priority::PeerPriority,
    tests::{mock::MockNetwork, utils, utils::NUM_SELECTION_ITERATIONS},
};
use velor_config::{
    config::{VelorDataClientConfig, VelorDataPollerConfig},
    network_id::PeerNetworkId,
};
use velor_storage_service_types::StorageServiceError;
use claims::assert_matches;
use maplit::hashset;
use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashMap, HashSet},
    thread::sleep,
};

#[tokio::test]
async fn identify_peers_to_poll_rounds() {
    // Create a mock network with a poller
    let (mut mock_network, _, _, poller) = MockNetwork::new(None, None, None);

    // Add several priority peers
    let priority_peers =
        utils::add_several_peers(&mut mock_network, 10, PeerPriority::HighPriority);

    // Add several regular peers
    let regular_peers =
        utils::add_several_peers(&mut mock_network, 20, PeerPriority::MediumPriority);

    // Fetch the priority peers to poll multiple times and verify no regular peers are returned
    let num_polling_rounds = 100;
    for _ in 0..num_polling_rounds {
        let peers_to_poll = poller.identify_peers_to_poll(true).unwrap();
        for peer in peers_to_poll {
            assert!(priority_peers.contains(&peer));
            assert!(!regular_peers.contains(&peer));
        }
    }

    // Fetch the regular peers to poll multiple times and verify no priority peers are returned
    for _ in 0..num_polling_rounds {
        let peers_to_poll = poller.identify_peers_to_poll(false).unwrap();
        for peer in peers_to_poll {
            assert!(!priority_peers.contains(&peer));
            assert!(regular_peers.contains(&peer));
        }
    }

    // Fetch the peers to poll in alternating loops and verify
    // that we receive the expected peers.
    for i in 0..num_polling_rounds {
        // Alternate between polling priority and regular peers
        let poll_priority_peers = i % 2 == 0;

        let peers_to_poll = poller.identify_peers_to_poll(poll_priority_peers).unwrap();
        for peer in peers_to_poll {
            if poll_priority_peers {
                // Verify the peer is a priority peer
                assert!(priority_peers.contains(&peer));
                assert!(!regular_peers.contains(&peer));
            } else {
                // Verify the peer is a regular peer
                assert!(!priority_peers.contains(&peer));
                assert!(regular_peers.contains(&peer));
            }
        }
    }
}

#[tokio::test]
async fn identify_peers_to_poll_frequencies() {
    // Create the data client config
    let data_client_config = VelorDataClientConfig {
        data_poller_config: VelorDataPollerConfig {
            additional_polls_per_peer_bucket: 1,
            min_polls_per_second: 5,
            max_polls_per_second: 20,
            peer_bucket_size: 10,
            poll_loop_interval_ms: 100,
            ..Default::default()
        },
        ..Default::default()
    };

    // Test priority and regular peers
    for poll_priority_peers in [false, true] {
        // Create a list of peer counts and expected peer polling frequencies.
        // Format is: (peer_count, expected_polls_per_second).
        let peer_counts_and_polls_per_second = vec![
            (1, 5.0),
            (9, 5.0),
            (10, 6.0),
            (19, 6.0),
            (25, 7.0),
            (39, 8.0),
            (40, 9.0),
            (51, 10.0),
            (69, 11.0),
            (79, 12.0),
            (80, 13.0),
            (99, 14.0),
            (100, 15.0),
            (110, 16.0),
            (121, 17.0),
            (139, 18.0),
            (149, 19.0),
            (150, 20.0),
            (160, 20.0),
            (200, 20.0),
        ];

        // Test various peer counts and expected peer polling frequencies
        verify_peer_counts_and_polls(
            data_client_config,
            poll_priority_peers,
            peer_counts_and_polls_per_second,
        );
    }
}

#[tokio::test]
async fn identify_peers_to_poll_config_changes() {
    // Create the data client config with non-default config values
    let data_client_config = VelorDataClientConfig {
        data_poller_config: VelorDataPollerConfig {
            additional_polls_per_peer_bucket: 2,
            min_polls_per_second: 10,
            max_polls_per_second: 25,
            peer_bucket_size: 20,
            poll_loop_interval_ms: 50,
            ..Default::default()
        },
        ..Default::default()
    };

    // Test priority and regular peers
    for poll_priority_peers in [false, true] {
        // Create a list of peer counts and expected peer polling
        // frequencies using the poller config above.
        // Format is: (peer_count, expected_polls_per_second).
        let peer_counts_and_polls_per_second = vec![
            (1, 10.0),
            (10, 10.0),
            (20, 12.0),
            (30, 12.0),
            (50, 14.0),
            (60, 16.0),
            (80, 18.0),
            (90, 18.0),
            (110, 20.0),
            (120, 22.0),
            (130, 22.0),
            (150, 24.0),
            (160, 25.0),
            (170, 25.0),
            (180, 25.0),
            (190, 25.0),
            (200, 25.0),
            (250, 25.0),
        ];

        // Test various peer counts and expected peer polling frequencies
        verify_peer_counts_and_polls(
            data_client_config,
            poll_priority_peers,
            peer_counts_and_polls_per_second,
        );
    }
}

#[tokio::test]
async fn identify_peers_to_poll_latency_weights() {
    // Create the data client config
    let data_client_config = VelorDataClientConfig {
        data_poller_config: VelorDataPollerConfig {
            additional_polls_per_peer_bucket: 2,
            min_polls_per_second: 6,
            max_polls_per_second: 40,
            peer_bucket_size: 10,
            poll_loop_interval_ms: 250,
            ..Default::default()
        },
        ..Default::default()
    };

    // Test priority and regular peers
    for poll_priority_peers in [false, true] {
        // Test various peer counts
        for peer_count in &[20, 30, 50, 80, 100, 120, 150] {
            // Create a mock network with a poller
            let (mut mock_network, _, _, poller) =
                MockNetwork::new(None, Some(data_client_config), None);

            // Determine the peer priority
            let peer_priority = utils::get_peer_priority_for_polling(poll_priority_peers);

            // Add the expected number of peers
            let _ = utils::add_several_peers(&mut mock_network, *peer_count, peer_priority);

            // Gather the peers to poll over many rounds
            let mut peers_and_poll_counts = HashMap::new();
            for _ in 0..5_000 {
                // Identify the peers to poll this round
                let peers_to_poll = poller.identify_peers_to_poll(poll_priority_peers).unwrap();

                // Update the peer poll counts
                for peer in peers_to_poll {
                    *peers_and_poll_counts.entry(peer).or_insert(0) += 1;
                }
            }

            // Verify the highest selected peers are the lowest latency peers
            utils::verify_highest_peer_selection_latencies(
                &mut mock_network,
                &mut peers_and_poll_counts,
            );
        }
    }
}

#[tokio::test]
async fn identify_peers_to_poll_missing_latencies() {
    // Create the data client config
    let data_client_config = VelorDataClientConfig {
        data_poller_config: VelorDataPollerConfig {
            poll_loop_interval_ms: 1000,
            ..Default::default()
        },
        ..Default::default()
    };

    // Create a mock network, data client and poller
    let (mut mock_network, _, client, poller) =
        MockNetwork::new(None, Some(data_client_config), None);

    // Ensure the properties hold for both priority and non-priority peers
    for poll_priority_peers in [true, false] {
        // Determine the peer priority
        let peer_priority = utils::get_peer_priority_for_polling(poll_priority_peers);

        // Add several peers
        let peers = utils::add_several_peers(&mut mock_network, 100, peer_priority);

        // Set some peers to have missing latencies in the peer metadata
        let num_peers_with_missing_latencies = 10;
        let mut peers_with_missing_latencies = hashset![];
        for peer in peers.iter().take(num_peers_with_missing_latencies) {
            // Remove the latency metadata for the peer
            utils::remove_latency_metadata(&client, *peer);

            // Add the peer to the list of peers with missing latencies
            peers_with_missing_latencies.insert(*peer);
        }

        // Gather the peers to poll over many rounds
        let mut peers_and_poll_counts = HashMap::new();
        for _ in 0..NUM_SELECTION_ITERATIONS {
            // Identify the peers to poll this round
            let peers_to_poll = poller.identify_peers_to_poll(poll_priority_peers).unwrap();

            // Update the peer poll counts
            for peer in peers_to_poll {
                *peers_and_poll_counts.entry(peer).or_insert(0) += 1;
            }
        }

        // Build a min-heap of all peers by their polling counts
        let mut min_heap_poll_counts = BinaryHeap::new();
        for (peer, poll_count) in peers_and_poll_counts {
            min_heap_poll_counts.push((Reverse(poll_count), peer));
        }

        // Verify the fewest polled peers are the peers without latencies
        for _ in 0..num_peers_with_missing_latencies {
            // Get the peer and poll count
            let (_, peer) = min_heap_poll_counts.pop().unwrap();

            // Verify the peer is in the set of peers with missing latencies
            assert!(peers_with_missing_latencies.contains(&peer));
        }
    }
}

#[tokio::test]
async fn identify_peers_to_poll_disconnected() {
    // Create a mock network with a poller
    let (mut mock_network, _, _, poller) = MockNetwork::new(None, None, None);

    // Ensure the properties hold for both priority and non-priority peers
    for poll_priority_peers in [true, false] {
        // Request the next set of peers to poll and verify we have no peers
        assert_matches!(
            poller.identify_peers_to_poll(poll_priority_peers),
            Err(Error::NoConnectedPeers(_))
        );

        // Determine the peer priority
        let peer_priority = utils::get_peer_priority_for_polling(poll_priority_peers);

        // Add peer 1
        let peer_1 = mock_network.add_peer(peer_priority);

        // Request the next set of peers to poll and verify it's peer 1
        let peers_to_poll = poller.identify_peers_to_poll(poll_priority_peers).unwrap();
        assert_eq!(peers_to_poll, hashset![peer_1]);

        // Add peer 2 and disconnect peer 1
        let peer_2 = mock_network.add_peer(peer_priority);
        mock_network.disconnect_peer(peer_1);

        // Request the next set of peers to poll and verify it's peer 2
        let peers_to_poll = poller.identify_peers_to_poll(poll_priority_peers).unwrap();
        assert_eq!(peers_to_poll, hashset![peer_2]);

        // Disconnect peer 2
        mock_network.disconnect_peer(peer_2);

        // Request the next set of peers to poll and verify we have no peers
        assert_matches!(
            poller.identify_peers_to_poll(poll_priority_peers),
            Err(Error::NoConnectedPeers(_))
        );

        // Add peer 3
        let peer_3 = mock_network.add_peer(peer_priority);

        // Request the next set of peers to poll and verify it's peer 3
        let peers_to_poll = poller.identify_peers_to_poll(poll_priority_peers).unwrap();
        assert_eq!(peers_to_poll, hashset![peer_3]);

        // Disconnect peer 3
        mock_network.disconnect_peer(peer_3);

        // Request the next set of peers to poll and verify we have no peers
        assert_matches!(
            poller.identify_peers_to_poll(poll_priority_peers),
            Err(Error::NoConnectedPeers(_))
        );
    }
}

#[tokio::test]
async fn identify_peers_to_poll_ordering() {
    // Create a mock network with a poller
    let (mut mock_network, _, _, poller) = MockNetwork::new(None, None, None);

    // Ensure the properties hold for both priority and non-priority peers
    for poll_priority_peers in [true, false] {
        // Determine the peer priority
        let peer_priority = utils::get_peer_priority_for_polling(poll_priority_peers);

        // Add peer 1
        let peer_1 = mock_network.add_peer(peer_priority);

        // Request the next set of peers to poll and verify it's peer 1
        for _ in 0..3 {
            let peers_to_poll = poller.identify_peers_to_poll(poll_priority_peers).unwrap();
            assert_eq!(peers_to_poll, hashset![peer_1]);
            poller.in_flight_request_started(poll_priority_peers, &peer_1);
            poller.in_flight_request_complete(&peer_1);
        }

        // Add peer 2
        let peer_2 = mock_network.add_peer(peer_priority);

        // Request the next set of peers to poll and verify it's either peer
        let peers_to_poll = poller.identify_peers_to_poll(poll_priority_peers).unwrap();
        assert!(peers_to_poll == hashset![peer_1] || peers_to_poll == hashset![peer_2]);
        poller.in_flight_request_started(
            poll_priority_peers,
            &get_single_peer_from_set(&peers_to_poll),
        );
        poller.in_flight_request_complete(&get_single_peer_from_set(&peers_to_poll));

        // Request the next set of peers to poll and don't mark the request as complete
        let peers_to_poll_1 = poller.identify_peers_to_poll(poll_priority_peers).unwrap();
        poller.in_flight_request_started(
            poll_priority_peers,
            &get_single_peer_from_set(&peers_to_poll_1),
        );

        // Request another set of peers to poll and verify it's the other peer
        let peers_to_poll_2 = poller.identify_peers_to_poll(poll_priority_peers).unwrap();
        poller.in_flight_request_started(
            poll_priority_peers,
            &get_single_peer_from_set(&peers_to_poll_2),
        );
        assert_ne!(peers_to_poll_1, peers_to_poll_2);

        // Neither poll has completed (they're both in-flight), so make another request
        // and verify we get no peers.
        assert!(poller
            .identify_peers_to_poll(poll_priority_peers)
            .unwrap()
            .is_empty());

        // Add peer 3
        let peer_3 = mock_network.add_peer(peer_priority);

        // Request another peer again and verify it's peer_3
        let peers_to_poll_3 = poller.identify_peers_to_poll(poll_priority_peers).unwrap();
        assert_eq!(peers_to_poll_3, hashset![peer_3]);
        poller.in_flight_request_started(
            poll_priority_peers,
            &get_single_peer_from_set(&peers_to_poll_3),
        );

        // Mark the second poll as completed
        poller.in_flight_request_complete(&get_single_peer_from_set(&peers_to_poll_2));

        // Make another request and verify we get peer 2 now (as it was ready)
        let peers_to_poll = poller.identify_peers_to_poll(poll_priority_peers).unwrap();
        assert_eq!(peers_to_poll, peers_to_poll_2);
        poller.in_flight_request_started(
            poll_priority_peers,
            &get_single_peer_from_set(&peers_to_poll_2),
        );

        // Mark the first poll as completed
        poller.in_flight_request_complete(&get_single_peer_from_set(&peers_to_poll_1));

        // Make another request and verify we get peer 1 now
        let peers_to_poll = poller.identify_peers_to_poll(poll_priority_peers).unwrap();
        assert_eq!(peers_to_poll, peers_to_poll_1);
        poller.in_flight_request_started(
            poll_priority_peers,
            &get_single_peer_from_set(&peers_to_poll_1),
        );

        // Mark the third poll as completed
        poller.in_flight_request_complete(&get_single_peer_from_set(&peers_to_poll_3));

        // Make another request and verify we get peer 3 now
        let peers_to_poll = poller.identify_peers_to_poll(poll_priority_peers).unwrap();
        assert_eq!(peers_to_poll, peers_to_poll_3);
        poller.in_flight_request_complete(&get_single_peer_from_set(&peers_to_poll_3));
    }
}

#[tokio::test]
async fn identify_peers_to_poll_reconnected() {
    // Create a mock network with a poller
    let (mut mock_network, _, _, poller) = MockNetwork::new(None, None, None);

    // Ensure the properties hold for both priority and non-priority peers
    for poll_priority_peers in [true, false] {
        // Request the next set of peers to poll and verify we have no peers
        assert_matches!(
            poller.identify_peers_to_poll(poll_priority_peers),
            Err(Error::NoConnectedPeers(_))
        );

        // Determine the peer priority
        let peer_priority = utils::get_peer_priority_for_polling(poll_priority_peers);

        // Add peer 1
        let peer_1 = mock_network.add_peer(peer_priority);

        // Request the next set of peers to poll and verify it's peer 1
        let peers_to_poll = poller.identify_peers_to_poll(poll_priority_peers).unwrap();
        assert_eq!(peers_to_poll, hashset![peer_1]);

        // Add peer 2 and disconnect peer 1
        let peer_2 = mock_network.add_peer(peer_priority);
        mock_network.disconnect_peer(peer_1);

        // Request the next set of peers to poll and verify it's peer 2
        let peers_to_poll = poller.identify_peers_to_poll(poll_priority_peers).unwrap();
        assert_eq!(peers_to_poll, hashset![peer_2]);

        // Disconnect peer 2 and reconnect peer 1
        mock_network.disconnect_peer(peer_2);
        mock_network.reconnect_peer(peer_1);

        // Request the next set of peers to poll and verify it's peer 1
        let peers_to_poll = poller.identify_peers_to_poll(poll_priority_peers).unwrap();
        assert_eq!(peers_to_poll, hashset![peer_1]);

        // Disconnect peer 1
        mock_network.disconnect_peer(peer_1);

        // Request the next set of peers to poll and verify we have no peers
        assert_matches!(
            poller.identify_peers_to_poll(poll_priority_peers),
            Err(Error::NoConnectedPeers(_))
        );
    }
}

#[tokio::test]
async fn identify_peers_to_poll_reconnected_in_flight() {
    // Create a mock network with a poller
    let (mut mock_network, _, _, poller) = MockNetwork::new(None, None, None);

    // Ensure the properties hold for both priority and non-priority peers
    for poll_priority_peers in [true, false] {
        // Request the next set of peers to poll and verify we have no peers
        assert_matches!(
            poller.identify_peers_to_poll(poll_priority_peers),
            Err(Error::NoConnectedPeers(_))
        );

        // Determine the peer priority
        let peer_priority = utils::get_peer_priority_for_polling(poll_priority_peers);

        // Add peer 1
        let peer_1 = mock_network.add_peer(peer_priority);

        // Request the next set of peers to poll and verify it's peer 1.
        // Mark the request as in-flight but not completed.
        let peers_to_poll = poller.identify_peers_to_poll(poll_priority_peers).unwrap();
        assert_eq!(peers_to_poll, hashset![peer_1]);
        poller.in_flight_request_started(poll_priority_peers, &peer_1);

        // Add peer 2 and disconnect peer 1
        let peer_2 = mock_network.add_peer(peer_priority);
        mock_network.disconnect_peer(peer_1);

        // Request the next set of peers to poll and verify it's peer 2.
        // Mark the request as in-flight but not completed.
        let peers_to_poll = poller.identify_peers_to_poll(poll_priority_peers).unwrap();
        assert_eq!(peers_to_poll, hashset![peer_2]);
        poller.in_flight_request_started(poll_priority_peers, &peer_2);

        // Request the next set of peers to poll and verify no peers are returned
        // (peer 2's request is still in-flight).
        for _ in 0..10 {
            assert_eq!(
                poller.identify_peers_to_poll(poll_priority_peers),
                Ok(hashset![])
            );
        }

        // Reconnect peer 1
        poller.in_flight_request_complete(&peer_1);
        mock_network.reconnect_peer(peer_1);

        // Request the next set of peers to poll and verify it's peer 1
        // (peer 2's request is still in-flight).
        let peers_to_poll = poller.identify_peers_to_poll(poll_priority_peers).unwrap();
        assert_eq!(peers_to_poll, hashset![peer_1]);
        poller.in_flight_request_started(poll_priority_peers, &peer_1);

        // Mark peer 2's request as complete
        poller.in_flight_request_complete(&peer_2);

        // Request the next set of peers to poll and verify it's peer 2
        // (peer 1's request is still in-flight).
        let peers_to_poll = poller.identify_peers_to_poll(poll_priority_peers).unwrap();
        assert_eq!(peers_to_poll, hashset![peer_2]);

        // Disconnect peer 2
        mock_network.disconnect_peer(peer_2);

        // Request the next set of peers to poll and verify no peers are returned
        // (peer s's request is still in-flight).
        assert_eq!(
            poller.identify_peers_to_poll(poll_priority_peers),
            Ok(hashset![])
        );

        // Mark peer 1's request as complete
        poller.in_flight_request_complete(&peer_1);

        // Request the next set of peers to poll multiple times and
        // verify peer 1 is returned each time.
        for _ in 0..10 {
            let peers_to_poll = poller.identify_peers_to_poll(poll_priority_peers).unwrap();
            assert_eq!(peers_to_poll, hashset![peer_1]);
            poller.in_flight_request_started(poll_priority_peers, &peer_1);
            poller.in_flight_request_complete(&peer_1);
        }

        // Disconnect peer 1
        mock_network.disconnect_peer(peer_1);

        // Request the next set of peers to poll and verify we have no peers
        assert_matches!(
            poller.identify_peers_to_poll(poll_priority_peers),
            Err(Error::NoConnectedPeers(_))
        );
    }
}

#[tokio::test]
async fn identify_peers_to_poll_max_in_flight() {
    // Create a data client with max in-flight requests of 2
    let data_client_config = VelorDataClientConfig {
        data_poller_config: VelorDataPollerConfig {
            max_num_in_flight_priority_polls: 2,
            max_num_in_flight_regular_polls: 2,
            ..Default::default()
        },
        ..Default::default()
    };

    // Create a mock network with a poller
    let (mut mock_network, _, _, poller) = MockNetwork::new(None, Some(data_client_config), None);

    // Ensure the properties hold for both priority and non-priority peers
    for poll_priority_peers in [true, false] {
        // Determine the peer priority
        let peer_priority = utils::get_peer_priority_for_polling(poll_priority_peers);

        // Add peer 1
        let peer_1 = mock_network.add_peer(peer_priority);

        // Request the next set of peers to poll and verify it's peer 1.
        // Mark the request as in-flight but not completed.
        let peers_to_poll = poller.identify_peers_to_poll(poll_priority_peers).unwrap();
        assert_eq!(peers_to_poll, hashset![peer_1]);
        poller.in_flight_request_started(poll_priority_peers, &peer_1);

        // Add peer 2
        let peer_2 = mock_network.add_peer(peer_priority);

        // Request the next set of peers to poll and verify it's peer 2
        // (peer 1's request has not yet completed). Mark the request as
        // in-flight but not completed.
        let peers_to_poll = poller.identify_peers_to_poll(poll_priority_peers).unwrap();
        assert_eq!(peers_to_poll, hashset![peer_2]);
        poller.in_flight_request_started(poll_priority_peers, &peer_2);

        // Add peer 3
        let peer_3 = mock_network.add_peer(peer_priority);

        // Request the next set of peers to poll and verify none are returned
        // (we already have the maximum number of in-flight requests).
        assert_eq!(
            poller.identify_peers_to_poll(poll_priority_peers),
            Ok(hashset![])
        );

        // Mark peer 2's in-flight request as complete
        poller.in_flight_request_complete(&peer_2);

        // Request the next set of peers to poll and verify it's either peer 2 or peer 3
        let peers_to_poll = poller.identify_peers_to_poll(poll_priority_peers).unwrap();
        assert!(peers_to_poll == hashset![peer_2] || peers_to_poll == hashset![peer_3]);
        poller.in_flight_request_started(
            poll_priority_peers,
            &get_single_peer_from_set(&peers_to_poll),
        );

        // Request the next set of peers to poll and verify none are returned
        // (we already have the maximum number of in-flight requests).
        assert_eq!(
            poller.identify_peers_to_poll(poll_priority_peers),
            Ok(hashset![])
        );

        // Mark peer 1's in-flight request as complete
        poller.in_flight_request_complete(&peer_1);

        // Request the next set of peers to poll and verify it's not the
        // peer that already has an in-flight request.
        assert_ne!(
            poller.identify_peers_to_poll(poll_priority_peers).unwrap(),
            peers_to_poll
        );
    }
}

#[tokio::test]
async fn identify_peers_to_poll_max_in_flight_disjoint() {
    // Create a data client with max in-flight requests of 3
    let max_num_in_flight_polls = 3;
    let data_client_config = VelorDataClientConfig {
        data_poller_config: VelorDataPollerConfig {
            max_num_in_flight_priority_polls: max_num_in_flight_polls,
            max_num_in_flight_regular_polls: max_num_in_flight_polls,
            ..Default::default()
        },
        ..Default::default()
    };

    // Create a mock network with a poller
    let (mut mock_network, _, _, poller) = MockNetwork::new(None, Some(data_client_config), None);

    // Ensure the properties hold for both priority and non-priority peers
    for poll_priority_peers in [true, false] {
        // Determine the peer priority
        let peer_priority = utils::get_peer_priority_for_polling(poll_priority_peers);

        // Add several peers
        let _ = utils::add_several_peers(&mut mock_network, 100, peer_priority);

        // Keep requesting peers to poll until we get the maximum number of in-flight requests
        let mut peers_with_polls = hashset![];
        poll_peers_until_max_in_flight(
            max_num_in_flight_polls,
            &poller,
            poll_priority_peers,
            &mut peers_with_polls,
        );

        // Verify we have the maximum number of in-flight requests
        assert_eq!(peers_with_polls.len(), max_num_in_flight_polls as usize);

        // Mark only 1 of the in-flight requests as complete
        let peer_with_completed_poll = remove_first_peer(&mut peers_with_polls);
        poller.in_flight_request_complete(&peer_with_completed_poll);

        // Request the next set of peers to poll and verify 1 peer is returned
        let mut new_peers_to_poll = poller.identify_peers_to_poll(poll_priority_peers).unwrap();
        assert_eq!(new_peers_to_poll.len(), 1);
        assert!(peers_with_polls.is_disjoint(&new_peers_to_poll));

        // Mark the in-flight request as started and add the peer to the set of peers with polls
        let peer = remove_first_peer(&mut new_peers_to_poll);
        poller.in_flight_request_started(poll_priority_peers, &peer);
        peers_with_polls.extend(hashset![peer]);

        // Mark 2 of the in-flight requests as complete
        for _ in 0..2 {
            let peer_with_completed_poll = remove_first_peer(&mut peers_with_polls);
            poller.in_flight_request_complete(&peer_with_completed_poll);
        }

        // Request the next set of peers to poll and verify 2 peers are returned
        let new_peers_to_poll = poller.identify_peers_to_poll(poll_priority_peers).unwrap();
        assert_eq!(new_peers_to_poll.len(), 2);
        assert!(peers_with_polls.is_disjoint(&new_peers_to_poll));

        // Mark the in-flight requests as started and add the peers to the set of peers with polls
        for peer in new_peers_to_poll {
            poller.in_flight_request_started(poll_priority_peers, &peer);
            peers_with_polls.extend(hashset![peer]);
        }

        // Mark 3 of the in-flight requests as complete
        for _ in 0..3 {
            let peer_with_completed_poll = remove_first_peer(&mut peers_with_polls);
            poller.in_flight_request_complete(&peer_with_completed_poll);
        }

        // Verify that peers with polls is empty
        assert!(peers_with_polls.is_empty());

        // Request the next set of peers to poll and verify 3 peers are returned
        let new_peers_to_poll = poller.identify_peers_to_poll(poll_priority_peers).unwrap();
        assert_eq!(new_peers_to_poll.len(), 3);
    }
}

#[tokio::test]
async fn peers_with_active_polls() {
    // Create a data client with max in-flight requests of 3
    let max_num_in_flight_polls = 3;
    let data_client_config = VelorDataClientConfig {
        data_poller_config: VelorDataPollerConfig {
            max_num_in_flight_priority_polls: max_num_in_flight_polls,
            max_num_in_flight_regular_polls: max_num_in_flight_polls,
            ..Default::default()
        },
        ..Default::default()
    };

    // Create a mock network with a poller
    let (mut mock_network, _, _, poller) = MockNetwork::new(None, Some(data_client_config), None);

    // Ensure the properties hold for both priority and non-priority peers
    for poll_priority_peers in [true, false] {
        // Determine the peer priority
        let peer_priority = utils::get_peer_priority_for_polling(poll_priority_peers);

        // Add several peers
        let _ = utils::add_several_peers(&mut mock_network, 100, peer_priority);

        // Keep requesting peers to poll until we get the maximum number of in-flight requests
        let mut peers_with_polls = hashset![];
        poll_peers_until_max_in_flight(
            max_num_in_flight_polls,
            &poller,
            poll_priority_peers,
            &mut peers_with_polls,
        );

        // Verify we have the maximum number of in-flight requests and that
        // the peers with active polls matches our expectations.
        assert_eq!(peers_with_polls.len(), max_num_in_flight_polls as usize);
        assert_eq!(poller.all_peers_with_in_flight_polls(), peers_with_polls);

        // Mark only 1 of the in-flight requests as complete
        let peer_with_completed_poll = remove_first_peer(&mut peers_with_polls);
        poller.in_flight_request_complete(&peer_with_completed_poll);

        // Verify the peers with active polls matches our expectations
        assert_eq!(poller.all_peers_with_in_flight_polls(), peers_with_polls);

        // Request the next set of peers to poll and verify 1 peer is returned
        let mut new_peers_to_poll = poller.identify_peers_to_poll(poll_priority_peers).unwrap();
        assert_eq!(new_peers_to_poll.len(), 1);
        assert!(peers_with_polls.is_disjoint(&new_peers_to_poll));

        // Mark the in-flight request as started and add the peer to the set of peers with polls
        let peer = remove_first_peer(&mut new_peers_to_poll);
        poller.in_flight_request_started(poll_priority_peers, &peer);
        peers_with_polls.extend(hashset![peer]);

        // Verify the peers with active polls matches our expectations
        assert_eq!(poller.all_peers_with_in_flight_polls(), peers_with_polls);

        // Mark 2 of the in-flight requests as complete
        for _ in 0..2 {
            let peer_with_completed_poll = remove_first_peer(&mut peers_with_polls);
            poller.in_flight_request_complete(&peer_with_completed_poll);
        }

        // Verify the peers with active polls matches our expectations
        assert_eq!(poller.all_peers_with_in_flight_polls(), peers_with_polls);

        // Request the next set of peers to poll and verify 2 peers are returned
        let new_peers_to_poll = poller.identify_peers_to_poll(poll_priority_peers).unwrap();
        assert_eq!(new_peers_to_poll.len(), 2);
        assert!(peers_with_polls.is_disjoint(&new_peers_to_poll));

        // Mark the in-flight requests as started and add the peers to the set of peers with polls
        for peer in new_peers_to_poll {
            poller.in_flight_request_started(poll_priority_peers, &peer);
            peers_with_polls.extend(hashset![peer]);
        }

        // Mark 3 of the in-flight requests as complete
        for _ in 0..3 {
            let peer_with_completed_poll = remove_first_peer(&mut peers_with_polls);
            poller.in_flight_request_complete(&peer_with_completed_poll);
        }

        // Verify that peers with polls is empty and that the peers with active polls is empty
        assert!(peers_with_polls.is_empty());
        assert!(poller.all_peers_with_in_flight_polls().is_empty());
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn poll_peers_error_handling() {
    // Create a data client with max in-flight requests of 1
    let data_client_config = VelorDataClientConfig {
        data_poller_config: VelorDataPollerConfig {
            max_num_in_flight_priority_polls: 1,
            max_num_in_flight_regular_polls: 1,
            ..Default::default()
        },
        ..Default::default()
    };

    // Test both invalid and dropped responses
    for invalid_response in [true, false] {
        // Ensure the properties hold for both priority and non-priority peers
        for poll_priority_peers in [true, false] {
            // Create a mock network with a poller
            let (mut mock_network, _, _, poller) =
                MockNetwork::new(None, Some(data_client_config), None);

            // Verify we have no in-flight polls
            let num_in_flight_polls = get_num_in_flight_polls(poller.clone());
            assert_eq!(num_in_flight_polls, 0);

            // Determine the peer priority
            let peer_priority = utils::get_peer_priority_for_polling(poll_priority_peers);

            // Add a peer
            let (peer, network_id) = utils::add_peer_to_network(peer_priority, &mut mock_network);

            // Poll the peer
            let handle = poller::poll_peer(poller.clone(), poll_priority_peers, peer);

            // Handle the poll request
            sleep(std::time::Duration::from_millis(5_000));
            if let Some(network_request) = mock_network.next_request(network_id).await {
                if invalid_response {
                    // Send an invalid response
                    network_request
                        .response_sender
                        .send(Err(StorageServiceError::InternalError(
                            "An unexpected error occurred!".into(),
                        )));
                } else {
                    // Drop the network request
                    drop(network_request)
                }
            }

            // Wait for the poller to complete
            handle.await.unwrap();

            // Verify we have no in-flight polls
            let num_in_flight_polls = get_num_in_flight_polls(poller.clone());
            assert_eq!(num_in_flight_polls, 0);
        }
    }
}

/// Calculates the number of polls per second
fn calculate_polls_per_second(
    data_client_config: VelorDataClientConfig,
    num_polling_rounds: u64,
    total_num_polls: usize,
) -> f64 {
    // Calculate the number of polling rounds per second. Note: we divide
    // by 2.0 because we poll priority and regular peers separately.
    let num_polling_rounds_per_second =
        1_000.0 / (2.0 * data_client_config.data_poller_config.poll_loop_interval_ms as f64);

    // Calculate the number of polls per second
    let num_polling_seconds = (num_polling_rounds as f64) / num_polling_rounds_per_second;
    (total_num_polls as f64) / num_polling_seconds
}

/// Fetches the number of in-flight polling requests for peers
fn get_num_in_flight_polls(poller: DataSummaryPoller) -> u64 {
    poller.all_peers_with_in_flight_polls().len() as u64
}

/// Returns the single peer from the given set
fn get_single_peer_from_set(single_peer_set: &HashSet<PeerNetworkId>) -> PeerNetworkId {
    // Verify the set only contains a single peer
    assert_eq!(single_peer_set.len(), 1);

    // Get the single peer from the set
    *single_peer_set.iter().next().unwrap()
}

/// Polls peers until the maximum number of in-flight requests is reached
fn poll_peers_until_max_in_flight(
    max_num_in_flight_polls: u64,
    poller: &DataSummaryPoller,
    poll_priority_peers: bool,
    peers_with_polls: &mut HashSet<PeerNetworkId>,
) {
    loop {
        // Check if we've hit the maximum number of in-flight requests
        if peers_with_polls.len() >= (max_num_in_flight_polls as usize) {
            return; // We've hit the maximum number of in-flight requests
        }

        // Request the next set of peers to poll
        let new_peers_to_poll = poller.identify_peers_to_poll(poll_priority_peers).unwrap();
        for peer in &new_peers_to_poll {
            poller.in_flight_request_started(poll_priority_peers, peer);
        }

        // Add the new peers to the set of peers with polls
        peers_with_polls.extend(&new_peers_to_poll);
    }
}

/// Removes and returns the first peer in the given set
fn remove_first_peer(peers_with_polls: &mut HashSet<PeerNetworkId>) -> PeerNetworkId {
    let peer = *peers_with_polls.iter().next().unwrap();
    peers_with_polls.remove(&peer);
    peer
}

/// Verifies that the number of polls per second is within a
/// reasonable delta of the expectation (for each peer count).
fn verify_peer_counts_and_polls(
    data_client_config: VelorDataClientConfig,
    poll_priority_peers: bool,
    peer_counts_and_polls_per_second: Vec<(i32, f64)>,
) {
    for (peer_count, expected_polls_per_second) in peer_counts_and_polls_per_second {
        // Create a mock network with a poller
        let (mut mock_network, _, _, poller) =
            MockNetwork::new(None, Some(data_client_config), None);

        // Determine the peer priority
        let peer_priority = utils::get_peer_priority_for_polling(poll_priority_peers);

        // Add the expected number of peers
        let _ = utils::add_several_peers(&mut mock_network, peer_count as u64, peer_priority);

        // Sum the peers to poll over many rounds
        let num_polling_rounds = 1000;
        let mut total_num_polls = 0;
        for _ in 0..num_polling_rounds {
            let peers_to_poll = poller.identify_peers_to_poll(poll_priority_peers).unwrap();
            total_num_polls += peers_to_poll.len();
        }

        // Calculate the number of polls per second
        let num_polls_per_second =
            calculate_polls_per_second(data_client_config, num_polling_rounds, total_num_polls);

        // Verify the number of polls per second is within a reasonable delta (i.e., 10%)
        let reasonable_delta = expected_polls_per_second / 10.0;
        assert!((num_polls_per_second - expected_polls_per_second).abs() < reasonable_delta);
    }
}

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{client::AptosDataClient, error::Error, poller::poll_peer, tests::mock::MockNetwork};
use aptos_config::{config::AptosDataClientConfig, network_id::PeerNetworkId};
use aptos_storage_service_types::StorageServiceError;
use claims::{assert_matches, assert_none};

#[tokio::test]
async fn fetch_peers_frequency() {
    ::aptos_logger::Logger::init_for_testing();
    let (mut mock_network, _, client, poller) = MockNetwork::new(None, None, None);

    // Add regular peer 1 and 2
    let _regular_peer_1 = mock_network.add_peer(false);
    let _regular_peer_2 = mock_network.add_peer(false);

    // Set `always_poll` to true and fetch the regular peers multiple times. Ensure
    // that for each fetch we receive a peer.
    let num_fetches = 20;
    for _ in 0..num_fetches {
        let peer = poller.fetch_regular_peer(true).unwrap();
        client.in_flight_request_complete(&peer);
    }

    // Set `always_poll` to false and fetch the regular peers multiple times
    let mut regular_peer_count = 0;
    for _ in 0..num_fetches {
        if let Some(peer) = poller.fetch_regular_peer(false) {
            regular_peer_count += 1;
            client.in_flight_request_complete(&peer);
        }
    }

    // Verify we received regular peers at a reduced frequency
    assert!(regular_peer_count < num_fetches);

    // Add priority peer 1 and 2
    let _priority_peer_1 = mock_network.add_peer(true);
    let _priority_peer_2 = mock_network.add_peer(true);

    // Fetch the prioritized peers multiple times. Ensure that for
    // each fetch we receive a peer.
    for _ in 0..num_fetches {
        let peer = poller.try_fetch_peer(true).unwrap();
        client.in_flight_request_complete(&peer);
    }
}

#[tokio::test]
async fn fetch_peers_ordering() {
    ::aptos_logger::Logger::init_for_testing();
    let (mut mock_network, _, client, _) = MockNetwork::new(None, None, None);

    // Ensure the properties hold for both priority and non-priority peers
    for is_priority_peer in [true, false] {
        // Add peer 1
        let peer_1 = mock_network.add_peer(is_priority_peer);

        // Request the next peer to poll and verify that we get peer 1
        for _ in 0..3 {
            let peer_to_poll = fetch_peer_to_poll(client.clone(), is_priority_peer)
                .unwrap()
                .unwrap();
            assert_eq!(peer_to_poll, peer_1);
            client.in_flight_request_complete(&peer_to_poll);
        }

        // Add peer 2
        let peer_2 = mock_network.add_peer(is_priority_peer);

        // Request the next peer and verify we get either peer
        let peer_to_poll = fetch_peer_to_poll(client.clone(), is_priority_peer)
            .unwrap()
            .unwrap();
        assert!(peer_to_poll == peer_1 || peer_to_poll == peer_2);
        client.in_flight_request_complete(&peer_to_poll);

        // Request the next peer again, but don't mark the poll as complete
        let peer_to_poll_1 = fetch_peer_to_poll(client.clone(), is_priority_peer)
            .unwrap()
            .unwrap();

        // Request another peer again and verify that it's different to the previous peer
        let peer_to_poll_2 = fetch_peer_to_poll(client.clone(), is_priority_peer)
            .unwrap()
            .unwrap();
        assert_ne!(peer_to_poll_1, peer_to_poll_2);

        // Neither poll has completed (they're both in-flight), so make another request
        // and verify we get no peers.
        assert_none!(fetch_peer_to_poll(client.clone(), is_priority_peer).unwrap());

        // Add peer 3
        let peer_3 = mock_network.add_peer(is_priority_peer);

        // Request another peer again and verify it's peer_3
        let peer_to_poll_3 = fetch_peer_to_poll(client.clone(), is_priority_peer)
            .unwrap()
            .unwrap();
        assert_eq!(peer_to_poll_3, peer_3);

        // Mark the second poll as completed
        client.in_flight_request_complete(&peer_to_poll_2);

        // Make another request and verify we get peer 2 now (as it was ready)
        let peer_to_poll = fetch_peer_to_poll(client.clone(), is_priority_peer)
            .unwrap()
            .unwrap();
        assert_eq!(peer_to_poll, peer_to_poll_2);

        // Mark the first poll as completed
        client.in_flight_request_complete(&peer_to_poll_1);

        // Make another request and verify we get peer 1 now
        let peer_to_poll = fetch_peer_to_poll(client.clone(), is_priority_peer)
            .unwrap()
            .unwrap();
        assert_eq!(peer_to_poll, peer_to_poll_1);

        // Mark the third poll as completed
        client.in_flight_request_complete(&peer_to_poll_3);

        // Make another request and verify we get peer 3 now
        let peer_to_poll = fetch_peer_to_poll(client.clone(), is_priority_peer)
            .unwrap()
            .unwrap();
        assert_eq!(peer_to_poll, peer_to_poll_3);
        client.in_flight_request_complete(&peer_to_poll_3);
    }
}

#[tokio::test]
async fn fetch_peers_disconnect() {
    ::aptos_logger::Logger::init_for_testing();
    let (mut mock_network, _, client, _) = MockNetwork::new(None, None, None);

    // Ensure the properties hold for both priority and non-priority peers
    for is_priority_peer in [true, false] {
        // Request the next peer to poll and verify we have no peers
        assert_matches!(
            fetch_peer_to_poll(client.clone(), is_priority_peer),
            Err(Error::DataIsUnavailable(_))
        );

        // Add peer 1
        let peer_1 = mock_network.add_peer(is_priority_peer);

        // Request the next peer to poll and verify it's peer 1
        let peer_to_poll = fetch_peer_to_poll(client.clone(), is_priority_peer)
            .unwrap()
            .unwrap();
        assert_eq!(peer_to_poll, peer_1);
        client.in_flight_request_complete(&peer_to_poll);

        // Add peer 2 and disconnect peer 1
        let peer_2 = mock_network.add_peer(is_priority_peer);
        mock_network.disconnect_peer(peer_1);

        // Request the next peer to poll and verify it's peer 2
        let peer_to_poll = fetch_peer_to_poll(client.clone(), is_priority_peer)
            .unwrap()
            .unwrap();
        assert_eq!(peer_to_poll, peer_2);
        client.in_flight_request_complete(&peer_to_poll);

        // Disconnect peer 2
        mock_network.disconnect_peer(peer_2);

        // Request the next peer to poll and verify an error is returned because
        // there are no connected peers.
        assert_matches!(
            fetch_peer_to_poll(client.clone(), is_priority_peer),
            Err(Error::DataIsUnavailable(_))
        );

        // Add peer 3
        let peer_3 = mock_network.add_peer(is_priority_peer);

        // Request the next peer to poll and verify it's peer 3
        let peer_to_poll = fetch_peer_to_poll(client.clone(), is_priority_peer)
            .unwrap()
            .unwrap();
        assert_eq!(peer_to_poll, peer_3);
        client.in_flight_request_complete(&peer_to_poll);

        // Disconnect peer 3
        mock_network.disconnect_peer(peer_3);

        // Request the next peer to poll and verify an error is returned because
        // there are no connected peers.
        assert_matches!(
            fetch_peer_to_poll(client.clone(), is_priority_peer),
            Err(Error::DataIsUnavailable(_))
        );
    }
}

#[tokio::test]
async fn fetch_peers_reconnect() {
    ::aptos_logger::Logger::init_for_testing();
    let (mut mock_network, _, client, _) = MockNetwork::new(None, None, None);

    // Ensure the properties hold for both priority and non-priority peers
    for is_priority_peer in [true, false] {
        // Request the next peer to poll and verify we have no peers
        assert_matches!(
            fetch_peer_to_poll(client.clone(), is_priority_peer),
            Err(Error::DataIsUnavailable(_))
        );

        // Add peer 1
        let peer_1 = mock_network.add_peer(is_priority_peer);

        // Request the next peer to poll and verify it's peer 1
        let peer_to_poll = fetch_peer_to_poll(client.clone(), is_priority_peer)
            .unwrap()
            .unwrap();
        assert_eq!(peer_to_poll, peer_1);
        client.in_flight_request_complete(&peer_to_poll);

        // Add peer 2 and disconnect peer 1
        let peer_2 = mock_network.add_peer(is_priority_peer);
        mock_network.disconnect_peer(peer_1);

        // Request the next peer to poll and verify it's peer 2
        let peer_to_poll = fetch_peer_to_poll(client.clone(), is_priority_peer)
            .unwrap()
            .unwrap();
        assert_eq!(peer_to_poll, peer_2);
        client.in_flight_request_complete(&peer_to_poll);

        // Disconnect peer 2 and reconnect peer 1
        mock_network.disconnect_peer(peer_2);
        mock_network.reconnect_peer(peer_1);

        // Request the next peer to poll and verify it's peer 1
        let peer_to_poll = fetch_peer_to_poll(client.clone(), is_priority_peer)
            .unwrap()
            .unwrap();
        assert_eq!(peer_to_poll, peer_1);

        // Reconnect peer 2
        mock_network.reconnect_peer(peer_2);

        // Request the next peer to poll several times and verify it's peer 2
        // (the in-flight request for peer 1 has yet to complete).
        for _ in 0..3 {
            let peer_to_poll = fetch_peer_to_poll(client.clone(), is_priority_peer)
                .unwrap()
                .unwrap();
            assert_eq!(peer_to_poll, peer_2);
            client.in_flight_request_complete(&peer_to_poll);
        }

        // Disconnect peer 2 and mark peer 1's in-flight request as complete
        mock_network.disconnect_peer(peer_2);
        client.in_flight_request_complete(&peer_1);

        // Request the next peer to poll several times and verify it's peer 1
        for _ in 0..3 {
            let peer_to_poll = fetch_peer_to_poll(client.clone(), is_priority_peer)
                .unwrap()
                .unwrap();
            assert_eq!(peer_to_poll, peer_1);
            client.in_flight_request_complete(&peer_to_poll);
        }

        // Disconnect peer 1
        mock_network.disconnect_peer(peer_1);

        // Request the next peer to poll and verify an error is returned because
        // there are no connected peers.
        assert_matches!(
            fetch_peer_to_poll(client.clone(), is_priority_peer),
            Err(Error::DataIsUnavailable(_))
        );
    }
}

#[tokio::test]
async fn fetch_peers_max_in_flight() {
    ::aptos_logger::Logger::init_for_testing();

    // Create a data client with max in-flight requests of 2
    let data_client_config = AptosDataClientConfig {
        max_num_in_flight_priority_polls: 2,
        max_num_in_flight_regular_polls: 2,
        ..Default::default()
    };
    let (mut mock_network, _, client, _) = MockNetwork::new(None, Some(data_client_config), None);

    // Ensure the properties hold for both priority and non-priority peers
    for is_priority_peer in [true, false] {
        // Add peer 1
        let peer_1 = mock_network.add_peer(is_priority_peer);

        // Request the next peer to poll and verify it's peer 1
        let peer_to_poll = fetch_peer_to_poll(client.clone(), is_priority_peer)
            .unwrap()
            .unwrap();
        assert_eq!(peer_to_poll, peer_1);

        // Add peer 2
        let peer_2 = mock_network.add_peer(is_priority_peer);

        // Request the next peer to poll and verify it's peer 2 (peer 1's in-flight
        // request has not yet completed).
        let peer_to_poll = fetch_peer_to_poll(client.clone(), is_priority_peer)
            .unwrap()
            .unwrap();
        assert_eq!(peer_to_poll, peer_2);

        // Add peer 3
        let peer_3 = mock_network.add_peer(is_priority_peer);

        // Request the next peer to poll and verify it's empty (we already have
        // the maximum number of in-flight requests).
        assert_none!(fetch_peer_to_poll(client.clone(), is_priority_peer).unwrap());

        // Mark peer 2's in-flight request as complete
        client.in_flight_request_complete(&peer_2);

        // Request the next peer to poll and verify it's either peer 2 or peer 3
        let peer_to_poll_1 = fetch_peer_to_poll(client.clone(), is_priority_peer)
            .unwrap()
            .unwrap();
        assert!(peer_to_poll_1 == peer_2 || peer_to_poll_1 == peer_3);

        // Request the next peer to poll and verify it's empty (we already have
        // the maximum number of in-flight requests).
        assert_none!(fetch_peer_to_poll(client.clone(), is_priority_peer).unwrap());

        // Mark peer 1's in-flight request as complete
        client.in_flight_request_complete(&peer_1);

        // Request the next peer to poll and verify it's not the peer that already
        // has an in-flight request.
        let peer_to_poll_2 = fetch_peer_to_poll(client.clone(), is_priority_peer)
            .unwrap()
            .unwrap();
        assert_ne!(peer_to_poll_1, peer_to_poll_2);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn in_flight_error_handling() {
    ::aptos_logger::Logger::init_for_testing();

    // Create a data client with max in-flight requests of 1
    let data_client_config = AptosDataClientConfig {
        max_num_in_flight_priority_polls: 1,
        max_num_in_flight_regular_polls: 1,
        ..Default::default()
    };
    let (mut mock_network, _, client, _) = MockNetwork::new(None, Some(data_client_config), None);

    // Verify we have no in-flight polls
    let num_in_flight_polls = get_num_in_flight_polls(client.clone(), true);
    assert_eq!(num_in_flight_polls, 0);

    // Add a peer
    let peer = mock_network.add_peer(true);

    // Poll the peer
    client.in_flight_request_started(&peer);
    let handle = poll_peer(client.clone(), peer, None);

    // Respond to the peer poll with an error
    if let Some(network_request) = mock_network.next_request().await {
        network_request
            .response_sender
            .send(Err(StorageServiceError::InternalError(
                "An unexpected error occurred!".into(),
            )));
    }

    // Wait for the poller to complete
    handle.await.unwrap();

    // Verify we have no in-flight polls
    let num_in_flight_polls = get_num_in_flight_polls(client.clone(), true);
    assert_eq!(num_in_flight_polls, 0);
}

/// A helper method that fetches peers to poll depending on the peer priority
fn fetch_peer_to_poll(
    client: AptosDataClient,
    is_priority_peer: bool,
) -> Result<Option<PeerNetworkId>, Error> {
    // Fetch the next peer to poll
    let result = if is_priority_peer {
        client.fetch_prioritized_peer_to_poll()
    } else {
        client.fetch_regular_peer_to_poll()
    };

    // If we get a peer, mark the peer as having an in-flight request
    if let Ok(Some(peer_to_poll)) = result {
        client.in_flight_request_started(&peer_to_poll);
    }

    result
}

/// Fetches the number of in flight requests for peers depending on priority
fn get_num_in_flight_polls(client: AptosDataClient, is_priority_peer: bool) -> u64 {
    if is_priority_peer {
        client.get_peer_states().num_in_flight_priority_polls()
    } else {
        client.get_peer_states().num_in_flight_regular_polls()
    }
}

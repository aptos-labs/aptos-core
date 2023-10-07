// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    client::AptosDataClient,
    error::Error,
    interface::AptosDataClientInterface,
    poller,
    poller::{poll_peer, DataSummaryPoller},
    tests::{mock::MockNetwork, utils},
};
use aptos_config::{config::AptosDataClientConfig, network_id::PeerNetworkId};
use aptos_storage_service_types::{
    requests::DataRequest,
    responses::{CompleteDataRange, DataResponse, StorageServerSummary, StorageServiceResponse},
    StorageServiceError,
};
use aptos_types::transaction::TransactionListWithProof;
use claims::{assert_err, assert_matches};
use maplit::hashset;
use std::{collections::HashSet, time::Duration};

#[tokio::test]
async fn bad_peer_is_eventually_banned_internal() {
    // Ensure the properties hold for both priority and non-priority peers
    for poll_priority_peers in [true, false] {
        // Create the mock network and client
        let data_client_config = AptosDataClientConfig::default();
        let (mut mock_network, _, client, _) =
            MockNetwork::new(None, Some(data_client_config), None);

        // Add a good and bad peer (on the same network)
        let (good_peer, good_network_id) =
            utils::add_peer_to_network(poll_priority_peers, &mut mock_network);
        let (bad_peer, network_id) =
            utils::add_peer_to_network(poll_priority_peers, &mut mock_network);
        assert_eq!(good_network_id, network_id);

        // The good peer advertises txns 0 -> 100 and the bad peer advertises txns 0 -> 200.
        client.update_peer_storage_summary(good_peer, utils::create_storage_summary(100));
        client.update_peer_storage_summary(bad_peer, utils::create_storage_summary(200));
        client.update_global_summary_cache().unwrap();

        // The global summary should contain the bad peer's advertisement.
        let global_summary = client.get_global_data_summary();
        assert!(global_summary
            .advertised_data
            .transactions
            .contains(&CompleteDataRange::new(0, 200).unwrap()));

        // Spawn a handler for both peers.
        tokio::spawn(async move {
            while let Some(network_request) = mock_network.next_request(network_id).await {
                let peer_network_id = network_request.peer_network_id;
                let response_sender = network_request.response_sender;
                if peer_network_id == good_peer {
                    // Good peer responds with good response.
                    let data_response =
                        DataResponse::TransactionsWithProof(TransactionListWithProof::new_empty());
                    response_sender
                        .send(Ok(StorageServiceResponse::new(data_response, true).unwrap()));
                } else if peer_network_id == bad_peer {
                    // Bad peer responds with error.
                    response_sender.send(Err(StorageServiceError::InternalError("".to_string())));
                }
            }
        });

        // Sending a bunch of requests to the bad peer's upper range will fail.
        let mut seen_data_unavailable_err = false;
        let request_timeout = data_client_config.response_timeout_ms;
        for _ in 0..20 {
            let result = client
                .get_transactions_with_proof(200, 200, 200, false, request_timeout)
                .await;

            // While the score is still decreasing, we should see a bunch of
            // InternalError's. Once we see a `DataIsUnavailable` error, we should
            // only see that error.
            if !seen_data_unavailable_err {
                assert_err!(&result);
                if let Err(Error::DataIsUnavailable(_)) = result {
                    seen_data_unavailable_err = true;
                }
            } else {
                assert_matches!(result, Err(Error::DataIsUnavailable(_)));
            }
        }

        // Peer should eventually get ignored and we should consider this request
        // range unserviceable.
        assert!(seen_data_unavailable_err);

        // The global summary should no longer contain the bad peer's advertisement.
        client.update_global_summary_cache().unwrap();
        let global_summary = client.get_global_data_summary();
        assert!(!global_summary
            .advertised_data
            .transactions
            .contains(&CompleteDataRange::new(0, 200).unwrap()));

        // We should still be able to send the good peer a request.
        let response = client
            .get_transactions_with_proof(100, 50, 100, false, request_timeout)
            .await
            .unwrap();
        assert_eq!(response.payload, TransactionListWithProof::new_empty());
    }
}

#[tokio::test]
async fn bad_peer_is_eventually_banned_callback() {
    // Ensure the properties hold for both priority and non-priority peers
    for poll_priority_peers in [true, false] {
        // Create the mock network and client
        let data_client_config = AptosDataClientConfig::default();
        let (mut mock_network, _, client, _) =
            MockNetwork::new(None, Some(data_client_config), None);

        // Add a bad peer
        let (bad_peer, network_id) =
            utils::add_peer_to_network(poll_priority_peers, &mut mock_network);

        // Bypass poller and just add the storage summaries directly.
        // Bad peer advertises txns 0 -> 200 (but can't actually service).
        client.update_peer_storage_summary(bad_peer, utils::create_storage_summary(200));
        client.update_global_summary_cache().unwrap();

        // Spawn a handler for both peers.
        tokio::spawn(async move {
            while let Some(network_request) = mock_network.next_request(network_id).await {
                let data_response =
                    DataResponse::TransactionsWithProof(TransactionListWithProof::new_empty());
                network_request
                    .response_sender
                    .send(Ok(StorageServiceResponse::new(data_response, true).unwrap()));
            }
        });

        let mut seen_data_unavailable_err = false;

        // Sending a bunch of requests to the bad peer (that we later decide are bad).
        let request_timeout = data_client_config.response_timeout_ms;
        for _ in 0..20 {
            let result = client
                .get_transactions_with_proof(200, 200, 200, false, request_timeout)
                .await;

            // While the score is still decreasing, we should see a bunch of
            // InternalError's. Once we see a `DataIsUnavailable` error, we should
            // only see that error.
            if !seen_data_unavailable_err {
                match result {
                    Ok(response) => {
                        response.context.response_callback.notify_bad_response(
                            crate::interface::ResponseError::ProofVerificationError,
                        );
                    },
                    Err(Error::DataIsUnavailable(_)) => {
                        seen_data_unavailable_err = true;
                    },
                    Err(_) => panic!("unexpected result: {:?}", result),
                }
            } else {
                assert_matches!(result, Err(Error::DataIsUnavailable(_)));
            }
        }

        // Peer should eventually get ignored and we should consider this request
        // range unserviceable.
        assert!(seen_data_unavailable_err);

        // The global summary should no longer contain the bad peer's advertisement.
        client.update_global_summary_cache().unwrap();
        let global_summary = client.get_global_data_summary();
        assert!(!global_summary
            .advertised_data
            .transactions
            .contains(&CompleteDataRange::new(0, 200).unwrap()));
    }
}

#[tokio::test]
async fn bad_peer_is_eventually_added_back() {
    // Ensure the properties hold for both priority and non-priority peers
    for poll_priority_peers in [true, false] {
        // Create the mock network, client and poller
        let data_client_config = AptosDataClientConfig::default();
        let (mut mock_network, mock_time, client, poller) =
            MockNetwork::new(None, Some(data_client_config), None);

        // Add a connected peer
        let (_, network_id) = utils::add_peer_to_network(poll_priority_peers, &mut mock_network);

        // Start the poller
        tokio::spawn(poller::start_poller(poller));

        // Spawn a handler that emulates peer responses
        tokio::spawn(async move {
            while let Some(network_request) = mock_network.next_request(network_id).await {
                match network_request.storage_service_request.data_request {
                    DataRequest::GetTransactionsWithProof(_) => {
                        let data_response = DataResponse::TransactionsWithProof(
                            TransactionListWithProof::new_empty(),
                        );
                        network_request
                            .response_sender
                            .send(Ok(StorageServiceResponse::new(
                                data_response,
                                network_request.storage_service_request.use_compression,
                            )
                            .unwrap()));
                    },
                    DataRequest::GetStorageServerSummary => {
                        let data_response =
                            DataResponse::StorageServerSummary(utils::create_storage_summary(200));
                        network_request
                            .response_sender
                            .send(Ok(StorageServiceResponse::new(
                                data_response,
                                network_request.storage_service_request.use_compression,
                            )
                            .unwrap()));
                    },
                    _ => panic!(
                        "Unexpected storage request: {:?}",
                        network_request.storage_service_request
                    ),
                }
            }
        });

        // Advance time so the poller sends data summary requests.
        let poll_loop_interval_ms = data_client_config.data_poller_config.poll_loop_interval_ms;
        for _ in 0..10 {
            tokio::task::yield_now().await;
            mock_time
                .advance_async(Duration::from_millis(poll_loop_interval_ms))
                .await;
        }

        // Initially this request range is serviceable by this peer.
        let global_summary = client.get_global_data_summary();
        assert!(global_summary
            .advertised_data
            .transactions
            .contains(&CompleteDataRange::new(0, 200).unwrap()));

        // Keep decreasing this peer's score by considering its responses bad.
        // Eventually its score drops below IGNORE_PEER_THRESHOLD.
        let request_timeout = data_client_config.response_timeout_ms;
        for _ in 0..20 {
            let result = client
                .get_transactions_with_proof(200, 0, 200, false, request_timeout)
                .await;

            if let Ok(response) = result {
                response
                    .context
                    .response_callback
                    .notify_bad_response(crate::interface::ResponseError::ProofVerificationError);
            }
        }

        // Peer is eventually ignored and this request range unserviceable.
        client.update_global_summary_cache().unwrap();
        let global_summary = client.get_global_data_summary();
        assert!(!global_summary
            .advertised_data
            .transactions
            .contains(&CompleteDataRange::new(0, 200).unwrap()));

        // This peer still responds to the StorageServerSummary requests.
        // Its score keeps increasing and this peer is eventually added back.
        for _ in 0..100 {
            mock_time
                .advance_async(Duration::from_millis(poll_loop_interval_ms))
                .await;
        }

        // Verify the peer is no longer ignored and this request range is serviceable
        let global_summary = client.get_global_data_summary();
        assert!(global_summary
            .advertised_data
            .transactions
            .contains(&CompleteDataRange::new(0, 200).unwrap()));
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn disconnected_peers_garbage_collection() {
    // Ensure the properties hold for both priority and non-priority peers
    for poll_priority_peers in [true, false] {
        // Create the mock network, client and poller
        let (mut mock_network, _, client, poller) = MockNetwork::new(None, None, None);

        // Connect several priority peers
        let priority_peer_1 = mock_network.add_peer(poll_priority_peers);
        let priority_peer_2 = mock_network.add_peer(poll_priority_peers);
        let priority_peer_3 = mock_network.add_peer(poll_priority_peers);

        // Poll all of the peers to initialize the peer states
        let all_peers = hashset![priority_peer_1, priority_peer_2, priority_peer_3];
        poll_peers(
            &mut mock_network,
            &poller,
            poll_priority_peers,
            all_peers.clone(),
        )
        .await;

        // Verify we have peer states for all peers
        verify_peer_states(&client, all_peers.clone());

        // Disconnect priority peer 1 and update the global data summary
        mock_network.disconnect_peer(priority_peer_1);
        client.update_global_summary_cache().unwrap();

        // Verify we have peer states for only the remaining peers
        verify_peer_states(&client, hashset![priority_peer_2, priority_peer_3]);

        // Disconnect priority peer 2 and update the global data summary
        mock_network.disconnect_peer(priority_peer_2);
        client.update_global_summary_cache().unwrap();

        // Verify we have peer states for only priority peer 3
        verify_peer_states(&client, hashset![priority_peer_3]);

        // Reconnect priority peer 1, poll it and update the global data summary
        mock_network.reconnect_peer(priority_peer_1);
        poll_peers(&mut mock_network, &poller, poll_priority_peers, hashset![
            priority_peer_1
        ])
        .await;
        client.update_global_summary_cache().unwrap();

        // Verify we have peer states for priority peer 1 and 3
        verify_peer_states(&client, hashset![priority_peer_1, priority_peer_3]);

        // Reconnect priority peer 2, poll it and update the global data summary
        mock_network.reconnect_peer(priority_peer_2);
        poll_peers(&mut mock_network, &poller, poll_priority_peers, hashset![
            priority_peer_2
        ])
        .await;
        client.update_global_summary_cache().unwrap();

        // Verify we have peer states for all peers
        verify_peer_states(&client, all_peers);
    }
}

/// A simple helper function that polls all the specified peers
/// and returns storage server summaries for each.
async fn poll_peers(
    mock_network: &mut MockNetwork,
    poller: &DataSummaryPoller,
    poll_priority_peers: bool,
    all_peers: HashSet<PeerNetworkId>,
) {
    for peer in all_peers {
        // Poll the peer
        let handle = poll_peer(poller.clone(), poll_priority_peers, peer);

        // Respond to the poll request
        let network_request = mock_network.next_request(peer.network_id()).await.unwrap();
        let data_response = DataResponse::StorageServerSummary(StorageServerSummary::default());
        network_request
            .response_sender
            .send(Ok(StorageServiceResponse::new(data_response, true).unwrap()));

        // Wait for the poll to complete
        handle.await.unwrap();
    }
}

/// Verifies the exclusive existence of peer states for all the specified peers
fn verify_peer_states(client: &AptosDataClient, all_peers: HashSet<PeerNetworkId>) {
    let peer_to_states = client.get_peer_states().get_peer_to_states();
    for peer in &all_peers {
        assert!(peer_to_states.contains_key(peer));
    }
    assert_eq!(peer_to_states.len(), all_peers.len());
}

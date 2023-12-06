// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    client::AptosDataClient,
    error::Error,
    interface::AptosDataClientInterface,
    poller,
    poller::{poll_peer, DataSummaryPoller},
    priority::PeerPriority,
    tests::{mock::MockNetwork, utils},
};
use aptos_config::{
    config::AptosDataClientConfig,
    network_id::{NetworkId, PeerNetworkId},
};
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
    // Ensure the properties hold for all peer priorities
    for peer_priority in PeerPriority::get_all_ordered_priorities() {
        // Create a base config for a validator
        let base_config = utils::create_validator_base_config();

        // Create the mock network and client
        let data_client_config = AptosDataClientConfig::default();
        let (mut mock_network, _, client, _) =
            MockNetwork::new(Some(base_config), Some(data_client_config), None);

        // Add a good and a bad peer (with the same priority, on the same network)
        let (good_peer, good_network_id) =
            utils::add_peer_to_network(peer_priority, &mut mock_network);
        let (bad_peer, network_id) = utils::add_peer_to_network(peer_priority, &mut mock_network);
        assert_eq!(good_network_id, network_id);

        // The good peer advertises txns 0 -> 100 and the bad peer advertises txns 0 -> 200
        client.update_peer_storage_summary(good_peer, utils::create_storage_summary(100));
        client.update_peer_storage_summary(bad_peer, utils::create_storage_summary(200));
        client.update_global_summary_cache().unwrap();

        // Verify the global summary contains the bad peer's advertisement
        let global_summary = client.get_global_data_summary();
        let transaction_range = CompleteDataRange::new(0, 200).unwrap();
        assert!(global_summary
            .advertised_data
            .transactions
            .contains(&transaction_range));

        // Spawn a handler for both peers
        tokio::spawn(async move {
            while let Some(network_request) = mock_network.next_request(network_id).await {
                // Extract the peer network id and response sender
                let peer_network_id = network_request.peer_network_id;
                let response_sender = network_request.response_sender;

                // Determine the response to send based on the peer's network id
                let response = if peer_network_id == good_peer {
                    let data_response =
                        DataResponse::TransactionsWithProof(TransactionListWithProof::new_empty());
                    Ok(StorageServiceResponse::new(data_response, true).unwrap())
                } else if peer_network_id == bad_peer {
                    Err(StorageServiceError::InternalError(
                        "Oops! Something went wrong!".to_string(),
                    ))
                } else {
                    panic!("Unexpected peer network id: {:?}", peer_network_id);
                };

                // Send the response
                response_sender.send(response);
            }
        });

        // Sending a bunch of requests to the bad peer will fail
        let mut seen_data_unavailable_err = false;
        for _ in 0..20 {
            // Send a request to fetch transactions from the bad peer
            let response_timeout_ms = data_client_config.response_timeout_ms;
            let result = client
                .get_transactions_with_proof(200, 200, 200, false, response_timeout_ms)
                .await;

            // While the score is still decreasing, we should see internal errors.
            // Once we see that data is unavailable, we should only see that error.
            if !seen_data_unavailable_err {
                assert_err!(&result);
                if let Err(Error::DataIsUnavailable(_)) = result {
                    seen_data_unavailable_err = true;
                }
            } else {
                assert_matches!(result, Err(Error::DataIsUnavailable(_)));
            }
        }

        // The bad peer should eventually get ignored
        assert!(seen_data_unavailable_err);

        // Verify the global summary no longer contains the bad peer's advertisement
        client.update_global_summary_cache().unwrap();
        let global_summary = client.get_global_data_summary();
        assert!(!global_summary
            .advertised_data
            .transactions
            .contains(&transaction_range));

        // Verify that we can still send a request to the good peer
        let response_timeout_ms = data_client_config.response_timeout_ms;
        let response = client
            .get_transactions_with_proof(100, 50, 100, false, response_timeout_ms)
            .await
            .unwrap();
        assert_eq!(response.payload, TransactionListWithProof::new_empty());
    }
}

#[tokio::test]
async fn bad_peer_is_eventually_banned_callback() {
    // Ensure the properties hold for all peer priorities
    for peer_priority in PeerPriority::get_all_ordered_priorities() {
        // Create a base config for a VFN
        let base_config = utils::create_fullnode_base_config();
        let networks = vec![NetworkId::Vfn, NetworkId::Public];

        // Create the mock network and client
        let data_client_config = AptosDataClientConfig::default();
        let (mut mock_network, _, client, _) =
            MockNetwork::new(Some(base_config), Some(data_client_config), Some(networks));

        // Add a bad peer
        let (bad_peer, network_id) = utils::add_peer_to_network(peer_priority, &mut mock_network);

        // Bypass the data poller and just add the storage summaries directly.
        // Update the bad peer to advertise txns 0 -> 200.
        client.update_peer_storage_summary(bad_peer, utils::create_storage_summary(200));
        client.update_global_summary_cache().unwrap();

        // Spawn a handler for the bad peer
        tokio::spawn(async move {
            while let Some(network_request) = mock_network.next_request(network_id).await {
                let data_response =
                    DataResponse::TransactionsWithProof(TransactionListWithProof::new_empty());
                network_request
                    .response_sender
                    .send(Ok(StorageServiceResponse::new(data_response, true).unwrap()));
            }
        });

        // Send a bunch of requests to the bad peer (that we later decide are invalid)
        let mut seen_data_unavailable_err = false;
        for _ in 0..20 {
            // Send a request to fetch transactions from the bad peer
            let result = client
                .get_transactions_with_proof(
                    200,
                    200,
                    200,
                    false,
                    data_client_config.response_timeout_ms,
                )
                .await;

            // While the score is still decreasing, we should see internal errors.
            // Once we see that data is unavailable, we should only see that error.
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

        // The bad peer should eventually get ignored
        assert!(seen_data_unavailable_err);

        // Verify the global summary no longer contains the bad peer's advertisement
        client.update_global_summary_cache().unwrap();
        let global_summary = client.get_global_data_summary();
        let transaction_range = CompleteDataRange::new(0, 200).unwrap();
        assert!(!global_summary
            .advertised_data
            .transactions
            .contains(&transaction_range));
    }
}

#[tokio::test]
async fn bad_peer_is_eventually_added_back() {
    // Ensure the properties hold for all peer priorities
    for peer_priority in PeerPriority::get_all_ordered_priorities() {
        // Create a base config for a validator
        let base_config = utils::create_validator_base_config();

        // Create the mock network, mock time, client and poller
        let data_client_config = AptosDataClientConfig::default();
        let (mut mock_network, mock_time, client, poller) =
            MockNetwork::new(Some(base_config), Some(data_client_config), None);

        // Add a connected peer
        let (_, network_id) = utils::add_peer_to_network(peer_priority, &mut mock_network);

        // Start the poller
        tokio::spawn(poller::start_poller(poller));

        // Spawn a handler for the peer
        tokio::spawn(async move {
            while let Some(network_request) = mock_network.next_request(network_id).await {
                // Determine the data response based on the request
                let data_response = match network_request.storage_service_request.data_request {
                    DataRequest::GetTransactionsWithProof(_) => {
                        DataResponse::TransactionsWithProof(TransactionListWithProof::new_empty())
                    },
                    DataRequest::GetStorageServerSummary => {
                        DataResponse::StorageServerSummary(utils::create_storage_summary(200))
                    },
                    _ => panic!(
                        "Unexpected storage request: {:?}",
                        network_request.storage_service_request
                    ),
                };

                // Send the response
                let storage_response = StorageServiceResponse::new(
                    data_response,
                    network_request.storage_service_request.use_compression,
                )
                .unwrap();
                network_request.response_sender.send(Ok(storage_response));
            }
        });

        // Advance time so the poller sends data summary requests
        let poll_loop_interval_ms = data_client_config.data_poller_config.poll_loop_interval_ms;
        for _ in 0..10 {
            tokio::task::yield_now().await;
            mock_time
                .advance_async(Duration::from_millis(poll_loop_interval_ms))
                .await;
        }

        // Verify that this request range is serviceable by the peer
        let global_summary = client.get_global_data_summary();
        let transaction_range = CompleteDataRange::new(0, 200).unwrap();
        assert!(global_summary
            .advertised_data
            .transactions
            .contains(&transaction_range));

        // Keep decreasing this peer's score by considering its responses bad.
        // Eventually its score drops below threshold and it is ignored.
        for _ in 0..20 {
            // Send a request to fetch transactions from the peer
            let request_timeout = data_client_config.response_timeout_ms;
            let result = client
                .get_transactions_with_proof(200, 0, 200, false, request_timeout)
                .await;

            // Notify the client that the response was bad
            if let Ok(response) = result {
                response
                    .context
                    .response_callback
                    .notify_bad_response(crate::interface::ResponseError::ProofVerificationError);
            }
        }

        // Verify that the peer is eventually ignored and this data range becomes unserviceable
        client.update_global_summary_cache().unwrap();
        let global_summary = client.get_global_data_summary();
        assert!(!global_summary
            .advertised_data
            .transactions
            .contains(&transaction_range));

        // Keep elapsed time so the peer is eventually added back (it
        // will still respond to the storage summary requests).
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
            .contains(&transaction_range));
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn disconnected_peers_garbage_collection() {
    // Ensure the properties hold for all peer priorities
    for peer_priority in PeerPriority::get_all_ordered_priorities() {
        // Create a base config for a validator
        let base_config = utils::create_validator_base_config();

        // Create the mock network, client and poller
        let data_client_config = AptosDataClientConfig::default();
        let (mut mock_network, _, client, poller) =
            MockNetwork::new(Some(base_config), Some(data_client_config), None);

        // Connect several peers
        let peer_1 = mock_network.add_peer(peer_priority);
        let peer_2 = mock_network.add_peer(peer_priority);
        let peer_3 = mock_network.add_peer(peer_priority);

        // Poll all of the peers to initialize the peer states
        let all_peers = hashset![peer_1, peer_2, peer_3];
        poll_peers(&mut mock_network, &poller, peer_priority, all_peers.clone()).await;

        // Verify we have peer states for all peers
        verify_peer_states(&client, all_peers.clone());

        // Disconnect peer 1 and update the global data summary
        mock_network.disconnect_peer(peer_1);
        client.update_global_summary_cache().unwrap();

        // Verify we have peer states for only peer 2 and 3
        verify_peer_states(&client, hashset![peer_2, peer_3]);

        // Disconnect peer 2 and update the global data summary
        mock_network.disconnect_peer(peer_2);
        client.update_global_summary_cache().unwrap();

        // Verify we have peer states for only peer 3
        verify_peer_states(&client, hashset![peer_3]);

        // Reconnect peer 1, poll it and update the global data summary
        mock_network.reconnect_peer(peer_1);
        poll_peers(&mut mock_network, &poller, peer_priority, hashset![peer_1]).await;
        client.update_global_summary_cache().unwrap();

        // Verify we have peer states for peers 1 and 3
        verify_peer_states(&client, hashset![peer_1, peer_3]);

        // Reconnect peer 2, poll it and update the global data summary
        mock_network.reconnect_peer(peer_2);
        poll_peers(&mut mock_network, &poller, peer_priority, hashset![peer_2]).await;
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
    peer_priority: PeerPriority,
    all_peers: HashSet<PeerNetworkId>,
) {
    for peer in all_peers {
        // Poll the peer
        let handle = poll_peer(poller.clone(), peer_priority.is_high_priority(), peer);

        // Respond to the poll request
        let network_request = mock_network.next_request(peer.network_id()).await.unwrap();
        let data_response = DataResponse::StorageServerSummary(StorageServerSummary::default());
        let storage_response = StorageServiceResponse::new(data_response, true).unwrap();
        network_request.response_sender.send(Ok(storage_response));

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

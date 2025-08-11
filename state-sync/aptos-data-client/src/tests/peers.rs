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
    config::{AptosDataClientConfig, AptosDataMultiFetchConfig},
    network_id::{NetworkId, PeerNetworkId},
};
use aptos_storage_service_server::network::NetworkRequest;
use aptos_storage_service_types::{
    requests::DataRequest,
    responses::{CompleteDataRange, DataResponse, StorageServerSummary, StorageServiceResponse},
    StorageServiceError,
};
use aptos_types::transaction::{TransactionListWithProof, TransactionListWithProofV2};
use claims::{assert_err, assert_matches, assert_ok};
use maplit::hashset;
use rand::{rngs::OsRng, Rng};
use std::{collections::HashSet, time::Duration};

#[tokio::test]
async fn all_bad_peers_with_invalid_responses() {
    // Ensure the properties hold for all peer priorities
    for peer_priority in PeerPriority::get_all_ordered_priorities() {
        // Create a base config for a validator
        let base_config = utils::create_validator_base_config();

        // Create a data client with multi-fetch enabled (10 peers per request)
        let peers_for_multi_fetch = 10;
        let data_client_config = AptosDataClientConfig {
            data_multi_fetch_config: AptosDataMultiFetchConfig {
                enable_multi_fetch: true,
                min_peers_for_multi_fetch: peers_for_multi_fetch,
                max_peers_for_multi_fetch: peers_for_multi_fetch,
                ..Default::default()
            },
            ..Default::default()
        };

        // Create the mock network and client
        let (mut mock_network, _, client, _) =
            MockNetwork::new(Some(base_config), Some(data_client_config), None);

        // Add several bad peers
        let bad_peers = utils::add_several_peers(
            &mut mock_network,
            peers_for_multi_fetch as u64,
            peer_priority,
        );

        // Advertise data for all the peers (transactions 0 -> 200)
        let max_transaction_version = 200;
        let storage_summary = utils::create_storage_summary(max_transaction_version);
        for bad_peer in &bad_peers {
            client.update_peer_storage_summary(*bad_peer, storage_summary.clone());
        }
        client.update_global_summary_cache().unwrap();

        // Spawn a handler for the peers to respond with errors
        let network_id = bad_peers.iter().next().unwrap().network_id(); // All peers are on the same network
        tokio::spawn(async move {
            while let Some(network_request) = mock_network.next_request(network_id).await {
                tokio::spawn(async move {
                    // Wait some time to emulate network latencies
                    emulate_network_latencies(None).await;

                    // Respond to the request with an error
                    send_error_response(network_request);
                });
            }
        });

        // Send several requests to the peers
        for _ in 0..5 {
            verify_transactions_response(
                &data_client_config,
                &client,
                max_transaction_version,
                true,
            )
            .await;
        }
    }
}

#[tokio::test]
async fn bad_peer_is_eventually_banned_internal() {
    // Ensure the properties hold for all peer priorities
    for peer_priority in PeerPriority::get_all_ordered_priorities() {
        // Create a base config for a validator
        let base_config = utils::create_validator_base_config();

        // Create a data client config with peer ignoring enabled
        let data_client_config = AptosDataClientConfig {
            ignore_low_score_peers: true,
            ..Default::default()
        };

        // Create the mock network and client
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
                let peer_network_id = &network_request.peer_network_id;

                // Determine the response to send based on the peer's network id
                if *peer_network_id == good_peer {
                    send_transaction_response(network_request)
                } else if *peer_network_id == bad_peer {
                    send_error_response(network_request)
                } else {
                    panic!("Unexpected peer network id: {:?}", peer_network_id);
                };
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
        assert_eq!(response.payload, TransactionListWithProofV2::new_empty());
    }
}

#[tokio::test]
async fn bad_peer_is_eventually_banned_callback() {
    // Ensure the properties hold for all peer priorities
    for peer_priority in PeerPriority::get_all_ordered_priorities() {
        // Create a base config for a VFN
        let base_config = utils::create_fullnode_base_config();
        let networks = vec![NetworkId::Vfn, NetworkId::Public];

        // Create a data client config with peer ignoring enabled
        let data_client_config = AptosDataClientConfig {
            ignore_low_score_peers: true,
            ..Default::default()
        };

        // Create the mock network and client
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

        // Create a data client config with peer ignoring enabled
        let data_client_config = AptosDataClientConfig {
            enable_transaction_data_v2: false,
            ignore_low_score_peers: true,
            ..Default::default()
        };

        // Create the mock network, mock time, client and poller
        let (mut mock_network, mut mock_time, client, poller) =
            MockNetwork::new(Some(base_config), Some(data_client_config), None);

        // Add a connected peer
        let (_, network_id) = utils::add_peer_to_network(peer_priority, &mut mock_network);

        // Start the poller
        tokio::spawn(poller::start_poller(poller));

        // Spawn a handler for the peer
        let highest_synced_version = 200;
        tokio::spawn(async move {
            while let Some(network_request) = mock_network.next_request(network_id).await {
                // Determine the data response based on the request
                let data_response = match network_request.storage_service_request.data_request {
                    DataRequest::GetTransactionsWithProof(_) => {
                        DataResponse::TransactionsWithProof(TransactionListWithProof::new_empty())
                    },
                    DataRequest::GetStorageServerSummary => DataResponse::StorageServerSummary(
                        utils::create_storage_summary(highest_synced_version),
                    ),
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

        // Wait until the request range is serviceable by the peer
        let transaction_range = CompleteDataRange::new(0, highest_synced_version).unwrap();
        utils::wait_for_transaction_advertisement(
            &client,
            &mut mock_time,
            &data_client_config,
            transaction_range,
        )
        .await;

        // Keep decreasing this peer's score by considering their responses invalid.
        // Eventually the score drops below the threshold and it is ignored.
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

        // Keep elapsing time so the peer is eventually added back (it
        // will still respond to the storage summary requests).
        for _ in 0..10 {
            utils::advance_polling_timer(&mut mock_time, &data_client_config).await;
        }

        // Verify the peer is no longer ignored and this request range is serviceable
        utils::wait_for_transaction_advertisement(
            &client,
            &mut mock_time,
            &data_client_config,
            transaction_range,
        )
        .await;
    }
}

#[ignore] // TODO: This test seems flaky. Debug and fix it.
#[tokio::test]
async fn disable_ignoring_low_score_peers() {
    // Ensure the properties hold for all peer priorities
    for peer_priority in PeerPriority::get_all_ordered_priorities() {
        // Create a base config for a validator
        let base_config = utils::create_validator_base_config();

        // Create a data client config with peer ignoring disabled
        let data_client_config = AptosDataClientConfig {
            ignore_low_score_peers: false,
            ..Default::default()
        };

        // Create the mock network, mock time, client and poller
        let (mut mock_network, mut mock_time, client, poller) =
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
        for _ in 0..10 {
            utils::advance_polling_timer(&mut mock_time, &data_client_config).await;
        }

        // Verify that this request range is serviceable by the peer
        let global_summary = client.get_global_data_summary();
        let transaction_range = CompleteDataRange::new(0, 200).unwrap();
        assert!(global_summary
            .advertised_data
            .transactions
            .contains(&transaction_range));

        // Keep decreasing this peer's score by considering its responses bad
        for _ in 0..1000 {
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

        // Verify that the peer is not ignored, despite many bad responses
        client.update_global_summary_cache().unwrap();
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

#[tokio::test]
async fn single_good_peer() {
    // Ensure the properties hold for all peer priorities
    for peer_priority in PeerPriority::get_all_ordered_priorities() {
        // Create a base config for a validator
        let base_config = utils::create_validator_base_config();

        // Create a data client with multi-fetch enabled (10 peers per request)
        let peers_for_multi_fetch = 10;
        let data_client_config = AptosDataClientConfig {
            data_multi_fetch_config: AptosDataMultiFetchConfig {
                enable_multi_fetch: true,
                min_peers_for_multi_fetch: peers_for_multi_fetch,
                max_peers_for_multi_fetch: peers_for_multi_fetch,
                ..Default::default()
            },
            ..Default::default()
        };

        // Create the mock network and client
        let (mut mock_network, _, client, _) =
            MockNetwork::new(Some(base_config), Some(data_client_config), None);

        // Add several bad peers
        let bad_peers = utils::add_several_peers(
            &mut mock_network,
            (peers_for_multi_fetch - 1) as u64, // One less than the number of peers needed for multi-fetch
            peer_priority,
        );

        // Add a single good peer
        let (good_peer, network_id) = utils::add_peer_to_network(peer_priority, &mut mock_network);

        // Advertise data for all the peers (transactions 0 -> 1000)
        let max_transaction_version = 1000;
        let storage_summary = utils::create_storage_summary(max_transaction_version);
        for peer in bad_peers.iter().chain(&hashset![good_peer]) {
            client.update_peer_storage_summary(*peer, storage_summary.clone());
        }
        client.update_global_summary_cache().unwrap();

        // Spawn a handler for the peers to respond with errors
        tokio::spawn(async move {
            while let Some(network_request) = mock_network.next_request(network_id).await {
                tokio::spawn(async move {
                    // Wait some time to emulate network latencies
                    emulate_network_latencies(None).await;

                    // If the peer is the good peer, respond with a valid response.
                    // Otherwise, respond with an error or drop the request.
                    if network_request.peer_network_id == good_peer {
                        // Do further latency emulation to ensure the good peer is the slowest
                        emulate_network_latencies(Some(1000)).await;

                        // Finally send the response
                        send_transaction_response(network_request);
                    } else {
                        // Send an error or drop the request
                        if !OsRng.gen::<bool>() {
                            send_error_response(network_request);
                        }
                    }
                });
            }
        });

        // Send several requests to the peers
        for _ in 0..5 {
            verify_transactions_response(
                &data_client_config,
                &client,
                max_transaction_version,
                false,
            )
            .await;
        }
    }
}

#[tokio::test]
async fn single_good_peer_across_priorities() {
    // Create a base config for a validator
    let base_config = utils::create_validator_base_config();

    // Create a data client with multi-fetch enabled (5 peers per request)
    let peers_for_multi_fetch = 5;
    let data_client_config = AptosDataClientConfig {
        data_multi_fetch_config: AptosDataMultiFetchConfig {
            enable_multi_fetch: true,
            min_peers_for_multi_fetch: peers_for_multi_fetch,
            max_peers_for_multi_fetch: peers_for_multi_fetch,
            ..Default::default()
        },
        ..Default::default()
    };

    // Create the mock network and client
    let (mut mock_network, _, client, _) =
        MockNetwork::new(Some(base_config), Some(data_client_config), None);

    // Add several high-priority (but bad) peers
    let high_priority_peers = utils::add_several_peers(
        &mut mock_network,
        (peers_for_multi_fetch - 1) as u64, // One less than the number of peers needed for multi-fetch
        PeerPriority::HighPriority,
    );

    // Add a single medium-priority (but good) peer
    let (medium_priority_peer, _) =
        utils::add_peer_to_network(PeerPriority::MediumPriority, &mut mock_network);

    // Advertise data for all the peers (transactions 0 -> 500)
    let max_transaction_version = 500;
    let storage_summary = utils::create_storage_summary(max_transaction_version);
    for peer in high_priority_peers
        .iter()
        .chain(&hashset![medium_priority_peer])
    {
        client.update_peer_storage_summary(*peer, storage_summary.clone());
    }
    client.update_global_summary_cache().unwrap();

    // Spawn a handler for the peers to respond with errors
    tokio::spawn(async move {
        while let Some(network_request) = mock_network
            .next_request(medium_priority_peer.network_id())
            .await
        {
            tokio::spawn(async move {
                // Wait some time to emulate network latencies
                emulate_network_latencies(None).await;

                // If the peer is the good peer, respond with a valid response.
                // Otherwise, respond with an error or drop the request.
                if network_request.peer_network_id == medium_priority_peer {
                    // Do further latency emulation to ensure the good peer is the slowest
                    emulate_network_latencies(Some(1000)).await;

                    // Finally send the response
                    send_transaction_response(network_request);
                } else {
                    // Send an error or drop the request
                    if !OsRng.gen::<bool>() {
                        send_error_response(network_request);
                    }
                }
            });
        }
    });

    // Send several requests to the peers
    for _ in 0..5 {
        verify_transactions_response(&data_client_config, &client, max_transaction_version, false)
            .await;
    }
}

/// Emulates network latencies by sleeping for some amount of time.
/// If no duration is specified, the sleep duration is randomly chosen.
async fn emulate_network_latencies(sleep_duration_ms: Option<u64>) {
    let sleep_duration_ms = sleep_duration_ms.unwrap_or_else(|| {
        OsRng.gen::<u64>() % 500 // Up to 0.5 seconds
    });
    tokio::time::sleep(Duration::from_millis(sleep_duration_ms)).await;
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

/// Sends an error response to the specified network request
fn send_error_response(network_request: NetworkRequest) {
    network_request
        .response_sender
        .send(Err(StorageServiceError::InternalError(
            "Oops! Something went wrong!".to_string(),
        )));
}

/// Sends a transaction response to the specified network request
fn send_transaction_response(network_request: NetworkRequest) {
    // Create the storage service response
    let data_response = DataResponse::TransactionsWithProof(TransactionListWithProof::new_empty());
    let storage_service_response = StorageServiceResponse::new(data_response, true).unwrap();

    // Send the response
    network_request
        .response_sender
        .send(Ok(storage_service_response));
}

/// Verifies the exclusive existence of peer states for all the specified peers
fn verify_peer_states(client: &AptosDataClient, all_peers: HashSet<PeerNetworkId>) {
    let peer_to_states = client.get_peer_states().get_peer_to_states();
    for peer in &all_peers {
        assert!(peer_to_states.contains_key(peer));
    }
    assert_eq!(peer_to_states.len(), all_peers.len());
}

/// Sends a request to fetch transactions from the peers and
/// verifies that the response is expected.
async fn verify_transactions_response(
    data_client_config: &AptosDataClientConfig,
    client: &AptosDataClient,
    max_transaction_version: u64,
    expect_error: bool,
) {
    // Send a request to fetch transactions from the peers
    let result = client
        .get_transactions_with_proof(
            max_transaction_version,
            max_transaction_version,
            max_transaction_version,
            false,
            data_client_config.response_timeout_ms,
        )
        .await;

    // Verify the response
    if expect_error {
        assert_matches!(result, Err(Error::DataIsUnavailable(_)));
    } else {
        assert_ok!(result);
    }
}

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::tests::{mock, mock::MockClient, utils};
use aptos_config::{
    config::StorageServiceConfig,
    network_id::{NetworkId, PeerNetworkId},
};
use aptos_storage_service_types::requests::{
    DataRequest, NewTransactionsWithProofRequest, StorageServiceRequest,
};
use aptos_types::{epoch_change::EpochChangeProof, PeerId};
use claims::assert_none;
use futures::channel::oneshot::Receiver;

#[tokio::test(flavor = "multi_thread")]
async fn test_get_new_transactions() {
    // Test size and time-aware chunking
    for use_size_and_time_aware_chunking in [false, true] {
        // Test both v1 and v2 data requests
        for use_request_v2 in [false, true] {
            // Test small and large chunk sizes
            let max_transaction_chunk_size =
                StorageServiceConfig::default().max_transaction_chunk_size;
            for chunk_size in [1, 100, max_transaction_chunk_size] {
                // Test event inclusion
                for include_events in [true, false] {
                    // Create test data
                    let highest_version = 45576;
                    let highest_epoch = 43;
                    let lowest_version = 4566;
                    let peer_version = highest_version - chunk_size;
                    let highest_ledger_info =
                        utils::create_test_ledger_info_with_sigs(highest_epoch, highest_version);
                    let transaction_list_with_proof = utils::create_transaction_list_with_proof(
                        peer_version + 1,
                        highest_version,
                        highest_version,
                        include_events,
                        use_request_v2,
                    );

                    // Create the mock db reader
                    let mut db_reader = mock::create_mock_db_with_summary_updates(
                        highest_ledger_info.clone(),
                        lowest_version,
                    );
                    utils::expect_get_transactions(
                        &mut db_reader,
                        peer_version + 1,
                        highest_version - peer_version,
                        highest_version,
                        include_events,
                        transaction_list_with_proof.clone(),
                        use_size_and_time_aware_chunking,
                    );

                    // Create a storage service config
                    let storage_config = utils::create_storage_config(
                        use_request_v2,
                        use_size_and_time_aware_chunking,
                    );

                    // Create the storage client and server
                    let (mut mock_client, service, storage_service_notifier, mock_time, _) =
                        MockClient::new(Some(db_reader), Some(storage_config));
                    let active_optimistic_fetches = service.get_optimistic_fetches();
                    tokio::spawn(service.start());

                    // Send a request to optimistically fetch new transactions
                    let mut response_receiver = get_new_transactions_with_proof(
                        &mut mock_client,
                        peer_version,
                        highest_epoch,
                        include_events,
                        use_request_v2,
                        storage_config.max_network_chunk_bytes_v2,
                    )
                    .await;

                    // Wait until the optimistic fetch is active
                    utils::wait_for_active_optimistic_fetches(active_optimistic_fetches.clone(), 1)
                        .await;

                    // Verify no optimistic fetch response has been received yet
                    assert_none!(response_receiver.try_recv().unwrap());

                    // Force the optimistic fetch handler to work
                    utils::force_optimistic_fetch_handler_to_run(
                        &mut mock_client,
                        &mock_time,
                        &storage_service_notifier,
                    )
                    .await;

                    // Verify a response is received and that it contains the correct data
                    utils::verify_new_transactions_with_proof(
                        &mut mock_client,
                        response_receiver,
                        use_request_v2,
                        transaction_list_with_proof,
                        highest_ledger_info,
                    )
                    .await;
                }
            }
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_get_new_transactions_different_networks() {
    // Test size and time-aware chunking
    for use_size_and_time_aware_chunking in [false, true] {
        // Test both v1 and v2 data requests
        for use_request_v2 in [false, true] {
            // Test small and large chunk sizes
            let max_transaction_chunk_size =
                StorageServiceConfig::default().max_transaction_chunk_size;
            for chunk_size in [100, max_transaction_chunk_size] {
                // Test event inclusion
                for include_events in [true, false] {
                    // Create test data
                    let highest_version = 45576;
                    let highest_epoch = 43;
                    let lowest_version = 4566;
                    let peer_version_1 = highest_version - chunk_size;
                    let peer_version_2 = highest_version - (chunk_size - 10);
                    let highest_ledger_info =
                        utils::create_test_ledger_info_with_sigs(highest_epoch, highest_version);
                    let transaction_list_with_proof_1 = utils::create_transaction_list_with_proof(
                        peer_version_1 + 1,
                        highest_version,
                        highest_version,
                        include_events,
                        use_request_v2,
                    );
                    let transaction_list_with_proof_2 = utils::create_transaction_list_with_proof(
                        peer_version_2 + 1,
                        highest_version,
                        highest_version,
                        include_events,
                        use_request_v2,
                    );

                    // Create the mock db reader
                    let mut db_reader = mock::create_mock_db_with_summary_updates(
                        highest_ledger_info.clone(),
                        lowest_version,
                    );
                    utils::expect_get_transactions(
                        &mut db_reader,
                        peer_version_1 + 1,
                        highest_version - peer_version_1,
                        highest_version,
                        include_events,
                        transaction_list_with_proof_1.clone(),
                        use_size_and_time_aware_chunking,
                    );
                    utils::expect_get_transactions(
                        &mut db_reader,
                        peer_version_2 + 1,
                        highest_version - peer_version_2,
                        highest_version,
                        include_events,
                        transaction_list_with_proof_2.clone(),
                        use_size_and_time_aware_chunking,
                    );

                    // Create a storage service config
                    let storage_config = utils::create_storage_config(
                        use_request_v2,
                        use_size_and_time_aware_chunking,
                    );

                    // Create the storage client and server
                    let (mut mock_client, service, storage_service_notifier, mock_time, _) =
                        MockClient::new(Some(db_reader), Some(storage_config));
                    let active_optimistic_fetches = service.get_optimistic_fetches();
                    tokio::spawn(service.start());

                    // Send a request to optimistically fetch new transactions for peer 1
                    let peer_id = PeerId::random();
                    let peer_network_1 = PeerNetworkId::new(NetworkId::Public, peer_id);
                    let mut response_receiver_1 = get_new_transactions_with_proof_for_peer(
                        &mut mock_client,
                        peer_version_1,
                        highest_epoch,
                        include_events,
                        Some(peer_network_1),
                        use_request_v2,
                        storage_config.max_network_chunk_bytes_v2,
                    )
                    .await;

                    // Send a request to optimistically fetch new transactions for peer 2
                    let peer_network_2 = PeerNetworkId::new(NetworkId::Vfn, peer_id);
                    let mut response_receiver_2 = get_new_transactions_with_proof_for_peer(
                        &mut mock_client,
                        peer_version_2,
                        highest_epoch,
                        include_events,
                        Some(peer_network_2),
                        use_request_v2,
                        storage_config.max_network_chunk_bytes_v2,
                    )
                    .await;

                    // Wait until the optimistic fetches are active
                    utils::wait_for_active_optimistic_fetches(active_optimistic_fetches.clone(), 2)
                        .await;

                    // Verify no optimistic fetch response has been received yet
                    assert_none!(response_receiver_1.try_recv().unwrap());
                    assert_none!(response_receiver_2.try_recv().unwrap());

                    // Force the optimistic fetch handler to work
                    utils::force_optimistic_fetch_handler_to_run(
                        &mut mock_client,
                        &mock_time,
                        &storage_service_notifier,
                    )
                    .await;

                    // Verify a response is received and that it contains the correct data for both peers
                    utils::verify_new_transactions_with_proof(
                        &mut mock_client,
                        response_receiver_1,
                        use_request_v2,
                        transaction_list_with_proof_1,
                        highest_ledger_info.clone(),
                    )
                    .await;
                    utils::verify_new_transactions_with_proof(
                        &mut mock_client,
                        response_receiver_2,
                        use_request_v2,
                        transaction_list_with_proof_2,
                        highest_ledger_info,
                    )
                    .await;
                }
            }
        }
    }
}

#[tokio::test]
#[should_panic(expected = "Canceled")]
async fn test_get_new_transactions_disable_v2() {
    // Create a storage service config with transaction v2 disabled
    let storage_config = utils::create_storage_config(false, false);

    // Create the storage client and server
    let (mut mock_client, service, _, _, _) = MockClient::new(None, Some(storage_config));
    tokio::spawn(service.start());

    // Send a new transaction v2 request. This will cause a test panic
    // as no response will be received (the receiver is dropped).
    let response_receiver = get_new_transactions_with_proof(
        &mut mock_client,
        0,
        0,
        true,
        true, // use_request_v2
        0,
    )
    .await;

    // Wait for the response, which should never come
    response_receiver.await.unwrap().unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn test_get_new_transactions_epoch_change() {
    // Test size and time-aware chunking
    for use_size_and_time_aware_chunking in [false, true] {
        // Test both v1 and v2 data requests
        for use_request_v2 in [false, true] {
            // Test event inclusion
            for include_events in [true, false] {
                // Create test data
                let highest_version = 45576;
                let highest_epoch = 1032;
                let lowest_version = 4566;
                let peer_version = highest_version - 100;
                let peer_epoch = highest_epoch - 20;
                let epoch_change_version = peer_version + 45;
                let epoch_change_proof = EpochChangeProof {
                    ledger_info_with_sigs: vec![utils::create_test_ledger_info_with_sigs(
                        peer_epoch,
                        epoch_change_version,
                    )],
                    more: false,
                };
                let transaction_list_with_proof = utils::create_transaction_list_with_proof(
                    peer_version + 1,
                    epoch_change_version,
                    epoch_change_version,
                    include_events,
                    use_request_v2,
                );

                // Create the mock db reader
                let mut db_reader = mock::create_mock_db_with_summary_updates(
                    utils::create_test_ledger_info_with_sigs(highest_epoch, highest_version),
                    lowest_version,
                );
                utils::expect_get_transactions(
                    &mut db_reader,
                    peer_version + 1,
                    epoch_change_version - peer_version,
                    epoch_change_version,
                    include_events,
                    transaction_list_with_proof.clone(),
                    use_size_and_time_aware_chunking,
                );
                utils::expect_get_epoch_ending_ledger_infos(
                    &mut db_reader,
                    peer_epoch,
                    peer_epoch + 1,
                    epoch_change_proof.clone(),
                    use_size_and_time_aware_chunking,
                );

                // Create a storage service config
                let storage_config =
                    utils::create_storage_config(use_request_v2, use_size_and_time_aware_chunking);

                // Create the storage client and server
                let (mut mock_client, service, storage_service_notifier, mock_time, _) =
                    MockClient::new(Some(db_reader), Some(storage_config));
                let active_optimistic_fetches = service.get_optimistic_fetches();
                tokio::spawn(service.start());

                // Send a request to optimistically fetch new transactions
                let response_receiver = get_new_transactions_with_proof(
                    &mut mock_client,
                    peer_version,
                    peer_epoch,
                    include_events,
                    use_request_v2,
                    storage_config.max_network_chunk_bytes_v2,
                )
                .await;

                // Wait until the optimistic fetch is active
                utils::wait_for_active_optimistic_fetches(active_optimistic_fetches.clone(), 1)
                    .await;

                // Force the optimistic fetch handler to work
                utils::force_optimistic_fetch_handler_to_run(
                    &mut mock_client,
                    &mock_time,
                    &storage_service_notifier,
                )
                .await;

                // Verify a response is received and that it contains the correct data
                utils::verify_new_transactions_with_proof(
                    &mut mock_client,
                    response_receiver,
                    use_request_v2,
                    transaction_list_with_proof,
                    epoch_change_proof.ledger_info_with_sigs[0].clone(),
                )
                .await;
            }
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_get_new_transactions_max_chunk() {
    // Test size and time-aware chunking
    for use_size_and_time_aware_chunking in [false, true] {
        // Test both v1 and v2 data requests
        for use_request_v2 in [false, true] {
            // Create a storage service config with a configured max chunk size
            let max_transaction_chunk_size = 200;
            let storage_service_config = StorageServiceConfig {
                max_transaction_chunk_size,
                enable_size_and_time_aware_chunking: use_size_and_time_aware_chunking,
                enable_transaction_data_v2: use_request_v2,
                ..StorageServiceConfig::default()
            };

            // Test event inclusion
            for include_events in [true, false] {
                // Create test data
                let highest_version = 1034556;
                let highest_epoch = 343;
                let lowest_version = 3453;
                let requested_chunk_size = max_transaction_chunk_size + 1;
                let peer_version = highest_version - requested_chunk_size;
                let highest_ledger_info =
                    utils::create_test_ledger_info_with_sigs(highest_epoch, highest_version);
                let transaction_list_with_proof = utils::create_transaction_list_with_proof(
                    peer_version + 1,
                    peer_version + max_transaction_chunk_size,
                    peer_version + max_transaction_chunk_size,
                    include_events,
                    use_request_v2,
                );

                // Create the mock db reader
                let mut db_reader = mock::create_mock_db_with_summary_updates(
                    highest_ledger_info.clone(),
                    lowest_version,
                );
                utils::expect_get_transactions(
                    &mut db_reader,
                    peer_version + 1,
                    max_transaction_chunk_size,
                    highest_version,
                    include_events,
                    transaction_list_with_proof.clone(),
                    use_size_and_time_aware_chunking,
                );

                // Create the storage client and server
                let (mut mock_client, service, storage_service_notifier, mock_time, _) =
                    MockClient::new(Some(db_reader), Some(storage_service_config));
                let active_optimistic_fetches = service.get_optimistic_fetches();
                tokio::spawn(service.start());

                // Send a request to optimistically fetch new transactions
                let response_receiver = get_new_transactions_with_proof(
                    &mut mock_client,
                    peer_version,
                    highest_epoch,
                    include_events,
                    use_request_v2,
                    storage_service_config.max_network_chunk_bytes_v2,
                )
                .await;

                // Wait until the optimistic fetch is active
                utils::wait_for_active_optimistic_fetches(active_optimistic_fetches.clone(), 1)
                    .await;

                // Force the optimistic fetch handler to work
                utils::force_optimistic_fetch_handler_to_run(
                    &mut mock_client,
                    &mock_time,
                    &storage_service_notifier,
                )
                .await;

                // Verify a response is received and that it contains the correct data
                utils::verify_new_transactions_with_proof(
                    &mut mock_client,
                    response_receiver,
                    use_request_v2,
                    transaction_list_with_proof,
                    highest_ledger_info,
                )
                .await;
            }
        }
    }
}

/// Creates and sends a request for new transactions
async fn get_new_transactions_with_proof(
    mock_client: &mut MockClient,
    known_version: u64,
    known_epoch: u64,
    include_events: bool,
    use_request_v2: bool,
    max_response_bytes_v2: u64,
) -> Receiver<Result<bytes::Bytes, aptos_network::protocols::network::RpcError>> {
    get_new_transactions_with_proof_for_peer(
        mock_client,
        known_version,
        known_epoch,
        include_events,
        None,
        use_request_v2,
        max_response_bytes_v2,
    )
    .await
}

/// Creates and sends a request for new transactions for the specified peer
async fn get_new_transactions_with_proof_for_peer(
    mock_client: &mut MockClient,
    known_version: u64,
    known_epoch: u64,
    include_events: bool,
    peer_network_id: Option<PeerNetworkId>,
    use_request_v2: bool,
    max_response_bytes_v2: u64,
) -> Receiver<Result<bytes::Bytes, aptos_network::protocols::network::RpcError>> {
    // Create the data request
    let data_request = if use_request_v2 {
        DataRequest::get_new_transaction_data_with_proof(
            known_version,
            known_epoch,
            include_events,
            max_response_bytes_v2,
        )
    } else {
        DataRequest::GetNewTransactionsWithProof(NewTransactionsWithProofRequest {
            known_version,
            known_epoch,
            include_events,
        })
    };
    let storage_request = StorageServiceRequest::new(data_request, true);

    // Send the request
    let (peer_id, network_id) = utils::extract_peer_and_network_id(peer_network_id);
    mock_client
        .send_request(storage_request, peer_id, network_id)
        .await
}

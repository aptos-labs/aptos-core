// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::Error,
    interface::AptosDataClientInterface,
    peer_states::calculate_optimal_chunk_sizes,
    tests::{mock::MockNetwork, utils},
};
use aptos_config::config::AptosDataClientConfig;
use aptos_network::protocols::wire::handshake::v1::ProtocolId;
use aptos_storage_service_types::{
    requests::{DataRequest, TransactionsWithProofRequest},
    responses::{DataResponse, StorageServiceResponse},
};
use aptos_types::transaction::TransactionListWithProof;
use claims::assert_matches;
use std::time::Duration;

#[tokio::test]
async fn request_works_only_when_data_available() {
    ::aptos_logger::Logger::init_for_testing();
    let (mut mock_network, mock_time, client, poller) = MockNetwork::new(None, None, None);

    tokio::spawn(poller.start_poller());

    // This request should fail because no peers are currently connected
    let request_timeout = client.get_response_timeout_ms();
    let error = client
        .get_transactions_with_proof(100, 50, 100, false, request_timeout)
        .await
        .unwrap_err();
    assert_matches!(error, Error::DataIsUnavailable(_));

    // Add a connected peer
    let expected_peer = mock_network.add_peer(true);

    // Requesting some txns now will still fail since no peers are advertising
    // availability for the desired range.
    let error = client
        .get_transactions_with_proof(100, 50, 100, false, request_timeout)
        .await
        .unwrap_err();
    assert_matches!(error, Error::DataIsUnavailable(_));

    // Advance time so the poller sends a data summary request
    tokio::task::yield_now().await;
    mock_time.advance_async(Duration::from_millis(1_000)).await;

    // Receive their request and fulfill it
    let network_request = mock_network.next_request().await.unwrap();
    assert_eq!(network_request.peer_network_id, expected_peer);
    assert_eq!(network_request.protocol_id, ProtocolId::StorageServiceRpc);
    assert!(network_request.storage_service_request.use_compression);
    assert_matches!(
        network_request.storage_service_request.data_request,
        DataRequest::GetStorageServerSummary
    );

    let summary = utils::create_storage_summary(200);
    let data_response = DataResponse::StorageServerSummary(summary);
    network_request
        .response_sender
        .send(Ok(StorageServiceResponse::new(data_response, true).unwrap()));

    // Let the poller finish processing the response
    tokio::task::yield_now().await;

    // Handle the client's transactions request
    tokio::spawn(async move {
        let network_request = mock_network.next_request().await.unwrap();

        assert_eq!(network_request.peer_network_id, expected_peer);
        assert_eq!(network_request.protocol_id, ProtocolId::StorageServiceRpc);
        assert!(network_request.storage_service_request.use_compression);
        assert_matches!(
            network_request.storage_service_request.data_request,
            DataRequest::GetTransactionsWithProof(TransactionsWithProofRequest {
                start_version: 50,
                end_version: 100,
                proof_version: 100,
                include_events: false,
            })
        );

        let data_response =
            DataResponse::TransactionsWithProof(TransactionListWithProof::new_empty());
        network_request
            .response_sender
            .send(Ok(StorageServiceResponse::new(data_response, true).unwrap()));
    });

    // The client's request should succeed since a peer finally has advertised
    // data for this range.
    let response = client
        .get_transactions_with_proof(100, 50, 100, false, request_timeout)
        .await
        .unwrap();
    assert_eq!(response.payload, TransactionListWithProof::new_empty());
}

#[tokio::test]
async fn optimal_chunk_size_calculations() {
    // Create a test storage service config
    let max_epoch_chunk_size = 600;
    let max_state_chunk_size = 500;
    let max_transaction_chunk_size = 700;
    let max_transaction_output_chunk_size = 800;
    let data_client_config = AptosDataClientConfig {
        max_epoch_chunk_size,
        max_state_chunk_size,
        max_transaction_chunk_size,
        max_transaction_output_chunk_size,
        ..Default::default()
    };

    // Test median calculations
    let optimal_chunk_sizes = calculate_optimal_chunk_sizes(
        &data_client_config,
        vec![7, 5, 6, 8, 10],
        vec![100, 200, 300, 100],
        vec![900, 700, 500],
        vec![40],
    );
    assert_eq!(200, optimal_chunk_sizes.state_chunk_size);
    assert_eq!(7, optimal_chunk_sizes.epoch_chunk_size);
    assert_eq!(700, optimal_chunk_sizes.transaction_chunk_size);
    assert_eq!(40, optimal_chunk_sizes.transaction_output_chunk_size);

    // Test no advertised data
    let optimal_chunk_sizes =
        calculate_optimal_chunk_sizes(&data_client_config, vec![], vec![], vec![], vec![]);
    assert_eq!(max_state_chunk_size, optimal_chunk_sizes.state_chunk_size);
    assert_eq!(max_epoch_chunk_size, optimal_chunk_sizes.epoch_chunk_size);
    assert_eq!(
        max_transaction_chunk_size,
        optimal_chunk_sizes.transaction_chunk_size
    );
    assert_eq!(
        max_transaction_output_chunk_size,
        optimal_chunk_sizes.transaction_output_chunk_size
    );

    // Verify the config caps the amount of chunks
    let optimal_chunk_sizes = calculate_optimal_chunk_sizes(
        &data_client_config,
        vec![70, 50, 60, 80, 100],
        vec![1000, 1000, 2000, 3000],
        vec![9000, 7000, 5000],
        vec![400],
    );
    assert_eq!(max_state_chunk_size, optimal_chunk_sizes.state_chunk_size);
    assert_eq!(70, optimal_chunk_sizes.epoch_chunk_size);
    assert_eq!(
        max_transaction_chunk_size,
        optimal_chunk_sizes.transaction_chunk_size
    );
    assert_eq!(400, optimal_chunk_sizes.transaction_output_chunk_size);
}

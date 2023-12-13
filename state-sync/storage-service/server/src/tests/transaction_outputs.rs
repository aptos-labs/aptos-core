// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::tests::{mock, mock::MockClient, utils};
use aptos_config::config::StorageServiceConfig;
use aptos_storage_service_types::{
    requests::{DataRequest, TransactionOutputsWithProofRequest},
    responses::{DataResponse, StorageServiceResponse},
    StorageServiceError,
};
use claims::assert_matches;
use mockall::{predicate::eq, Sequence};

#[tokio::test]
async fn test_get_transaction_outputs_with_proof() {
    // Test small and large chunk requests
    let max_output_chunk_size = StorageServiceConfig::default().max_transaction_output_chunk_size;
    for chunk_size in [1, 100, max_output_chunk_size] {
        // Create test data
        let start_version = 0;
        let end_version = start_version + chunk_size - 1;
        let proof_version = end_version;
        let output_list_with_proof =
            utils::create_output_list_with_proof(start_version, end_version, proof_version);

        // Create the mock db reader
        let mut db_reader = mock::create_mock_db_reader();
        utils::expect_get_transaction_outputs(
            &mut db_reader,
            start_version,
            chunk_size,
            proof_version,
            output_list_with_proof.clone(),
        );

        // Create the storage client and server
        let (mut mock_client, mut service, _, _, _) = MockClient::new(Some(db_reader), None);
        utils::update_storage_server_summary(&mut service, proof_version + 100, 10);
        tokio::spawn(service.start());

        // Create a request to fetch transactions outputs with a proof
        let response = get_outputs_with_proof(
            &mut mock_client,
            start_version,
            end_version,
            end_version,
            true,
        )
        .await
        .unwrap();

        // Verify the response is correct
        match response.get_data_response().unwrap() {
            DataResponse::TransactionOutputsWithProof(outputs_with_proof) => {
                assert_eq!(outputs_with_proof, output_list_with_proof)
            },
            _ => panic!(
                "Expected transaction outputs with proof but got: {:?}",
                response
            ),
        };
    }
}

#[tokio::test]
async fn test_get_transaction_outputs_with_proof_chunk_limit() {
    // Create test data
    let max_output_chunk_size = StorageServiceConfig::default().max_transaction_output_chunk_size;
    let chunk_size = max_output_chunk_size * 10; // Set a chunk request larger than the max
    let start_version = 0;
    let end_version = start_version + max_output_chunk_size - 1;
    let proof_version = end_version;
    let output_list_with_proof =
        utils::create_output_list_with_proof(start_version, end_version, proof_version);

    // Create the mock db reader
    let mut db_reader = mock::create_mock_db_reader();
    utils::expect_get_transaction_outputs(
        &mut db_reader,
        start_version,
        max_output_chunk_size,
        proof_version,
        output_list_with_proof.clone(),
    );

    // Create the storage client and server
    let (mut mock_client, mut service, _, _, _) = MockClient::new(Some(db_reader), None);
    utils::update_storage_server_summary(&mut service, proof_version + chunk_size, 10);
    tokio::spawn(service.start());

    // Create a request to fetch transactions outputs with a proof
    let response = get_outputs_with_proof(
        &mut mock_client,
        start_version,
        start_version + chunk_size - 1,
        end_version,
        true,
    )
    .await
    .unwrap();

    // Verify the response is correct
    match response.get_data_response().unwrap() {
        DataResponse::TransactionOutputsWithProof(outputs_with_proof) => {
            assert_eq!(outputs_with_proof, output_list_with_proof)
        },
        _ => panic!(
            "Expected transaction outputs with proof but got: {:?}",
            response
        ),
    };
}

#[tokio::test]
async fn test_get_transaction_outputs_with_proof_invalid() {
    // Create the storage client and server
    let (mut mock_client, service, _, _, _) = MockClient::new(None, None);
    tokio::spawn(service.start());

    // Test invalid ranges
    let start_version = 1000;
    for end_version in [0, 999] {
        let response = get_outputs_with_proof(
            &mut mock_client,
            start_version,
            end_version,
            end_version,
            true,
        )
        .await
        .unwrap_err();
        assert_matches!(response, StorageServiceError::InvalidRequest(_));
    }
}

#[tokio::test]
async fn test_get_transaction_outputs_with_proof_network_limit() {
    // Test different byte limits
    for network_limit_bytes in [1, 5 * 1024, 50 * 1024, 100 * 1024] {
        get_outputs_with_proof_network_limit(network_limit_bytes).await;
    }
}

#[tokio::test]
async fn test_get_transaction_outputs_with_proof_not_serviceable() {
    // Test small and large chunk requests
    let max_output_chunk_size = StorageServiceConfig::default().max_transaction_output_chunk_size;
    for chunk_size in [2, 100, max_output_chunk_size] {
        // Create test data
        let start_version = 0;
        let end_version = start_version + chunk_size - 1;
        let proof_version = end_version;

        // Create the storage client and server (that cannot service the request)
        let (mut mock_client, mut service, _, _, _) = MockClient::new(None, None);
        utils::update_storage_server_summary(&mut service, proof_version - 1, 10);
        tokio::spawn(service.start());

        // Create a request to fetch transactions outputs with a proof
        let response = get_outputs_with_proof(
            &mut mock_client,
            start_version,
            end_version,
            end_version,
            true,
        )
        .await
        .unwrap_err();

        // Verify the request is not serviceable
        assert_matches!(response, StorageServiceError::InvalidRequest(_));
    }
}

/// Sends a transaction outputs with proof request and processes the response
async fn get_outputs_with_proof(
    mock_client: &mut MockClient,
    start_version: u64,
    end_version: u64,
    proof_version: u64,
    use_compression: bool,
) -> Result<StorageServiceResponse, StorageServiceError> {
    let data_request =
        DataRequest::GetTransactionOutputsWithProof(TransactionOutputsWithProofRequest {
            proof_version,
            start_version,
            end_version,
        });
    utils::send_storage_request(mock_client, use_compression, data_request).await
}

/// A helper method to request a transaction outputs with proof chunk using the
/// the specified network limit.
async fn get_outputs_with_proof_network_limit(network_limit_bytes: u64) {
    for use_compression in [true, false] {
        // Create test data
        let max_output_chunk_size =
            StorageServiceConfig::default().max_transaction_output_chunk_size;
        let min_bytes_per_output = 1536; // 1.5 KB
        let start_version = 455;
        let proof_version = 1000000;

        // Create the mock db reader
        let mut db_reader = mock::create_mock_db_reader();
        let mut expectation_sequence = Sequence::new();
        let mut chunk_size = max_output_chunk_size;
        while chunk_size >= 1 {
            let output_list_with_proof = utils::create_output_list_using_sizes(
                start_version,
                chunk_size,
                min_bytes_per_output,
            );
            db_reader
                .expect_get_transaction_outputs()
                .times(1)
                .with(eq(start_version), eq(chunk_size), eq(proof_version))
                .in_sequence(&mut expectation_sequence)
                .returning(move |_, _, _| Ok(output_list_with_proof.clone()));
            chunk_size /= 2;
        }

        // Create a storage config with the specified max network byte limit
        let storage_config = StorageServiceConfig {
            max_network_chunk_bytes: network_limit_bytes,
            ..Default::default()
        };

        // Create the storage client and server
        let (mut mock_client, mut service, _, _, _) =
            MockClient::new(Some(db_reader), Some(storage_config));
        utils::update_storage_server_summary(&mut service, proof_version, 10);
        tokio::spawn(service.start());

        // Process a request to fetch outputs with a proof
        let response = get_outputs_with_proof(
            &mut mock_client,
            start_version,
            start_version + (max_output_chunk_size * 10), // Request more than the max chunk
            proof_version,
            use_compression,
        )
        .await
        .unwrap();

        // Verify the response is correct
        match response.get_data_response().unwrap() {
            DataResponse::TransactionOutputsWithProof(outputs_with_proof) => {
                let num_response_bytes = bcs::serialized_size(&response).unwrap() as u64;
                let num_outputs = outputs_with_proof.transactions_and_outputs.len() as u64;
                if num_response_bytes > network_limit_bytes {
                    assert_eq!(num_outputs, 1); // Data cannot be reduced more than a single item
                } else {
                    let max_outputs = network_limit_bytes / min_bytes_per_output;
                    assert!(num_outputs <= max_outputs); // Verify data fits correctly into the limit
                }
            },
            _ => panic!("Expected outputs with proof but got: {:?}", response),
        };
    }
}

// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::tests::{mock, mock::MockClient, utils};
use velor_config::config::StorageServiceConfig;
use velor_storage_service_types::{
    requests::{DataRequest, TransactionOutputsWithProofRequest},
    responses::{DataResponse, StorageServiceResponse, TransactionDataResponseType},
    StorageServiceError,
};
use velor_types::transaction::TransactionOutputListWithProofV2;
use claims::assert_matches;
use mockall::{predicate::eq, Sequence};
use std::cmp::min;

#[tokio::test]
async fn test_get_transaction_outputs_with_proof() {
    // Test both v1 and v2 data requests
    for use_request_v2 in [false, true] {
        // Test small and large chunk requests
        let max_output_chunk_size =
            StorageServiceConfig::default().max_transaction_output_chunk_size;
        for chunk_size in [1, 100, max_output_chunk_size] {
            // Create test data
            let start_version = 0;
            let end_version = start_version + chunk_size - 1;
            let proof_version = end_version;
            let output_list_with_proof = utils::create_output_list_with_proof(
                start_version,
                end_version,
                proof_version,
                use_request_v2,
            );

            // Create the mock db reader
            let mut db_reader = mock::create_mock_db_reader();
            utils::expect_get_transaction_outputs(
                &mut db_reader,
                start_version,
                chunk_size,
                proof_version,
                output_list_with_proof.clone(),
            );

            // Create a storage service config
            let storage_config = utils::create_storage_config(use_request_v2);

            // Create the storage client and server
            let (mut mock_client, mut service, _, _, _) =
                MockClient::new(Some(db_reader), Some(storage_config));
            utils::update_storage_server_summary(&mut service, proof_version + 100, 10);
            tokio::spawn(service.start());

            // Create a request to fetch transactions outputs with a proof
            let response = get_outputs_with_proof(
                &mut mock_client,
                start_version,
                end_version,
                end_version,
                true,
                use_request_v2,
                storage_config.max_network_chunk_bytes_v2,
            )
            .await
            .unwrap();

            // Verify the response is correct
            verify_output_with_proof_response(use_request_v2, output_list_with_proof, response);
        }
    }
}

#[tokio::test]
async fn test_get_transaction_outputs_with_proof_chunk_limit() {
    // Test both v1 and v2 data requests
    for use_request_v2 in [false, true] {
        // Create test data
        let max_output_chunk_size =
            StorageServiceConfig::default().max_transaction_output_chunk_size;
        let chunk_size = max_output_chunk_size * 10; // Set a chunk request larger than the max
        let start_version = 0;
        let end_version = start_version + max_output_chunk_size - 1;
        let proof_version = end_version;
        let output_list_with_proof = utils::create_output_list_with_proof(
            start_version,
            end_version,
            proof_version,
            use_request_v2,
        );

        // Create the mock db reader
        let mut db_reader = mock::create_mock_db_reader();
        utils::expect_get_transaction_outputs(
            &mut db_reader,
            start_version,
            max_output_chunk_size,
            proof_version,
            output_list_with_proof.clone(),
        );

        // Create a storage service config
        let storage_config = utils::create_storage_config(use_request_v2);

        // Create the storage client and server
        let (mut mock_client, mut service, _, _, _) =
            MockClient::new(Some(db_reader), Some(storage_config));
        utils::update_storage_server_summary(&mut service, proof_version + chunk_size, 10);
        tokio::spawn(service.start());

        // Create a request to fetch transaction outputs with a proof
        let response = get_outputs_with_proof(
            &mut mock_client,
            start_version,
            start_version + chunk_size - 1,
            end_version,
            true,
            use_request_v2,
            storage_config.max_network_chunk_bytes_v2,
        )
        .await
        .unwrap();

        // Verify the response is correct
        verify_output_with_proof_response(use_request_v2, output_list_with_proof, response);
    }
}

#[tokio::test]
#[should_panic(expected = "Canceled")]
async fn test_get_transaction_outputs_with_proof_disable_v2() {
    // Create a storage service config with transaction v2 disabled
    let storage_config = utils::create_storage_config(false);

    // Create the storage client and server
    let (mut mock_client, service, _, _, _) = MockClient::new(None, Some(storage_config));
    tokio::spawn(service.start());

    // Send a transaction v2 request. This will cause a test panic
    // as no response will be received (the receiver is dropped).
    get_outputs_with_proof(
        &mut mock_client,
        0,
        10,
        10,
        true,
        true, // Use transaction v2
        storage_config.max_network_chunk_bytes_v2,
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn test_get_transaction_outputs_with_proof_invalid() {
    // Test both v1 and v2 data requests
    for use_request_v2 in [false, true] {
        // Create a storage service config
        let storage_config = utils::create_storage_config(use_request_v2);

        // Create the storage client and server
        let (mut mock_client, service, _, _, _) = MockClient::new(None, Some(storage_config));
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
                use_request_v2,
                storage_config.max_network_chunk_bytes_v2,
            )
            .await
            .unwrap_err();
            assert_matches!(response, StorageServiceError::InvalidRequest(_));
        }
    }
}

#[tokio::test]
async fn test_get_transaction_outputs_with_proof_network_limit() {
    // Test both v1 and v2 data requests
    for use_request_v2 in [false, true] {
        // Test different byte limits (the client and server limits match)
        for max_specified_bytes in [1, 5 * 1024, 50 * 1024] {
            get_outputs_with_proof_network_limit(
                max_specified_bytes,
                max_specified_bytes,
                use_request_v2,
            )
            .await;
        }
    }
}

#[tokio::test]
async fn test_get_transaction_outputs_with_proof_network_limit_client_bounded() {
    // Test both v1 and v2 data requests
    for use_request_v2 in [false, true] {
        // Test different byte limits
        for max_specified_bytes in [1, 5 * 1024, 50 * 1024] {
            let max_server_specified_bytes = max_specified_bytes * 10; // The server limit is 10x the client limit
            get_outputs_with_proof_network_limit(
                max_specified_bytes,
                max_server_specified_bytes,
                use_request_v2,
            )
            .await;
        }
    }
}

#[tokio::test]
async fn test_get_transaction_outputs_with_proof_network_limit_server_bounded() {
    // Test both v1 and v2 data requests
    for use_request_v2 in [false, true] {
        // Test different byte limits
        for max_specified_bytes in [1, 5 * 1024, 50 * 1024] {
            let max_client_specified_bytes = max_specified_bytes * 10; // The client limit is 10x the server limit
            get_outputs_with_proof_network_limit(
                max_client_specified_bytes,
                max_specified_bytes,
                use_request_v2,
            )
            .await;
        }
    }
}

#[tokio::test]
async fn test_get_transaction_outputs_with_proof_not_serviceable() {
    // Test both v1 and v2 data requests
    for use_request_v2 in [false, true] {
        // Test small and large chunk requests
        let max_output_chunk_size =
            StorageServiceConfig::default().max_transaction_output_chunk_size;
        for chunk_size in [2, 100, max_output_chunk_size] {
            // Create test data
            let start_version = 0;
            let end_version = start_version + chunk_size - 1;
            let proof_version = end_version;

            // Create a storage service config
            let storage_config = utils::create_storage_config(use_request_v2);

            // Create the storage client and server (that cannot service the request)
            let (mut mock_client, mut service, _, _, _) =
                MockClient::new(None, Some(storage_config));
            utils::update_storage_server_summary(&mut service, proof_version - 1, 10);
            tokio::spawn(service.start());

            // Create a request to fetch transactions outputs with a proof
            let response = get_outputs_with_proof(
                &mut mock_client,
                start_version,
                end_version,
                end_version,
                true,
                use_request_v2,
                storage_config.max_network_chunk_bytes_v2,
            )
            .await
            .unwrap_err();

            // Verify the request is not serviceable
            assert_matches!(response, StorageServiceError::InvalidRequest(_));
        }
    }
}

/// Sends a transaction outputs with proof request and processes the response
async fn get_outputs_with_proof(
    mock_client: &mut MockClient,
    start_version: u64,
    end_version: u64,
    proof_version: u64,
    use_compression: bool,
    use_request_v2: bool,
    max_response_bytes_v2: u64,
) -> Result<StorageServiceResponse, StorageServiceError> {
    let data_request = if use_request_v2 {
        DataRequest::get_transaction_output_data_with_proof(
            proof_version,
            start_version,
            end_version,
            max_response_bytes_v2,
        )
    } else {
        DataRequest::GetTransactionOutputsWithProof(TransactionOutputsWithProofRequest {
            proof_version,
            start_version,
            end_version,
        })
    };
    utils::send_storage_request(mock_client, use_compression, data_request).await
}

/// A helper method to request a transaction outputs with proof chunk using
/// the specified network limits (client and server).
async fn get_outputs_with_proof_network_limit(
    max_client_specified_bytes: u64,
    max_server_specified_bytes: u64,
    use_request_v2: bool,
) {
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
            // Expect a call to get outputs with the specified chunk size
            let output_list_with_proof = utils::create_output_list_using_sizes(
                start_version,
                chunk_size,
                min_bytes_per_output,
                use_request_v2,
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
            max_network_chunk_bytes: max_server_specified_bytes,
            max_network_chunk_bytes_v2: max_server_specified_bytes,
            enable_transaction_data_v2: use_request_v2,
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
            use_request_v2,
            max_client_specified_bytes,
        )
        .await
        .unwrap();

        // Verify the response is correct
        let num_response_bytes = bcs::serialized_size(&response).unwrap() as u64;
        let num_outputs = match response.get_data_response().unwrap() {
            DataResponse::TransactionOutputsWithProof(outputs_with_proof) => {
                outputs_with_proof.get_num_outputs() as u64
            },
            DataResponse::TransactionDataWithProof(transaction_data_with_proof) => {
                let output_list_with_proof_v2 = transaction_data_with_proof
                    .transaction_output_list_with_proof
                    .unwrap();
                output_list_with_proof_v2.get_num_outputs() as u64
            },
            _ => panic!("Expected outputs with proof but got: {:?}", response),
        };
        if num_response_bytes > max_server_specified_bytes {
            assert_eq!(num_outputs, 1); // Data cannot be reduced more than a single item
        } else {
            // Determine the max specified bytes
            let max_specified_bytes = if use_request_v2 {
                min(max_client_specified_bytes, max_server_specified_bytes)
            } else {
                max_server_specified_bytes
            };

            // Verify the number of outputs fits within the specified byte limit
            let max_outputs = max_specified_bytes / min_bytes_per_output;
            assert!(num_outputs <= max_outputs);
        }
    }
}

/// Verifies the response for a transaction output with proof request
fn verify_output_with_proof_response(
    use_request_v2: bool,
    output_list_with_proof: TransactionOutputListWithProofV2,
    response: StorageServiceResponse,
) {
    // Get the data response
    let data_response = response.get_data_response().unwrap();

    // Verify the response type (v1 or v2)
    match &data_response {
        DataResponse::TransactionOutputsWithProof(_) => assert!(!use_request_v2),
        DataResponse::TransactionDataWithProof(_) => {
            assert!(use_request_v2)
        },
        _ => panic!(
            "Expected transaction outputs with proof but got: {:?}",
            data_response
        ),
    }

    // Verify the response data
    match data_response {
        DataResponse::TransactionOutputsWithProof(outputs_with_proof) => {
            assert_eq!(
                outputs_with_proof,
                output_list_with_proof.get_output_list_with_proof().clone()
            )
        },
        DataResponse::TransactionDataWithProof(transaction_data_with_proof) => {
            // Verify the data type
            assert_eq!(
                transaction_data_with_proof.transaction_data_response_type,
                TransactionDataResponseType::TransactionOutputData
            );

            // Verify the transactions
            assert!(transaction_data_with_proof
                .transaction_list_with_proof
                .is_none());

            assert_eq!(
                transaction_data_with_proof
                    .transaction_output_list_with_proof
                    .unwrap(),
                output_list_with_proof
            );
        },
        _ => panic!(
            "Expected transaction outputs with proof but got: {:?}",
            data_response
        ),
    }
}

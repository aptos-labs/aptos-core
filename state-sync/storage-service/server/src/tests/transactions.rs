// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::tests::{
    mock,
    mock::{MockClient, MockDatabaseReader},
    utils,
};
use aptos_config::config::StorageServiceConfig;
use aptos_storage_service_types::{responses::DataResponse, StorageServiceError};
use claims::assert_matches;
use std::cmp::min;

#[tokio::test]
async fn test_get_transactions_with_proof() {
    // Test size and time-aware chunking
    for use_size_and_time_aware_chunking in [false, true] {
        // Test both v1 and v2 data requests
        for use_request_v2 in [false, true] {
            // Test small and large chunk requests
            let max_transaction_chunk_size =
                StorageServiceConfig::default().max_transaction_chunk_size;
            for chunk_size in [1, 100, max_transaction_chunk_size] {
                // Test event inclusion
                for include_events in [true, false] {
                    // Create test data
                    let start_version = 0;
                    let end_version = start_version + chunk_size - 1;
                    let proof_version = end_version;
                    let transaction_list_with_proof = utils::create_transaction_list_with_proof(
                        start_version,
                        end_version,
                        proof_version,
                        include_events,
                        use_request_v2,
                    );

                    // Create the mock db reader
                    let mut db_reader = mock::create_mock_db_reader();
                    utils::expect_get_transactions(
                        &mut db_reader,
                        start_version,
                        chunk_size,
                        proof_version,
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
                    let (mut mock_client, mut service, _, _, _) =
                        MockClient::new(Some(db_reader), Some(storage_config));
                    utils::update_storage_server_summary(&mut service, proof_version, 10);
                    tokio::spawn(service.start());

                    // Create a request to fetch transactions with a proof
                    let response = utils::get_transactions_with_proof(
                        &mut mock_client,
                        start_version,
                        end_version,
                        proof_version,
                        include_events,
                        true,
                        use_request_v2,
                        storage_config.max_network_chunk_bytes_v2,
                    )
                    .await
                    .unwrap();

                    // Verify the response is correct
                    utils::verify_transaction_with_proof_response(
                        use_request_v2,
                        transaction_list_with_proof,
                        response,
                    );
                }
            }
        }
    }
}

#[tokio::test]
async fn test_get_transactions_with_chunk_proof_limit() {
    // Test size and time-aware chunking
    for use_size_and_time_aware_chunking in [false, true] {
        // Test both v1 and v2 data requests
        for use_request_v2 in [false, true] {
            // Test event inclusion
            for include_events in [true, false] {
                // Create test data
                let max_transaction_chunk_size =
                    StorageServiceConfig::default().max_transaction_chunk_size;
                let chunk_size = max_transaction_chunk_size * 10; // Set a chunk request larger than the max
                let start_version = 0;
                let end_version = start_version + max_transaction_chunk_size - 1;
                let proof_version = end_version;
                let transaction_list_with_proof = utils::create_transaction_list_with_proof(
                    start_version,
                    end_version,
                    proof_version,
                    include_events,
                    use_request_v2,
                );

                // Create the mock db reader
                let mut db_reader = mock::create_mock_db_reader();
                utils::expect_get_transactions(
                    &mut db_reader,
                    start_version,
                    max_transaction_chunk_size,
                    proof_version,
                    include_events,
                    transaction_list_with_proof.clone(),
                    use_size_and_time_aware_chunking,
                );

                // Create a storage service config
                let storage_config =
                    utils::create_storage_config(use_request_v2, use_size_and_time_aware_chunking);

                // Create the storage client and server
                let (mut mock_client, mut service, _, _, _) =
                    MockClient::new(Some(db_reader), Some(storage_config));
                utils::update_storage_server_summary(&mut service, proof_version + chunk_size, 10);
                tokio::spawn(service.start());

                // Create a request to fetch transactions with a proof
                let response = utils::get_transactions_with_proof(
                    &mut mock_client,
                    start_version,
                    start_version + chunk_size - 1,
                    proof_version,
                    include_events,
                    true,
                    use_request_v2,
                    storage_config.max_network_chunk_bytes_v2,
                )
                .await
                .unwrap();

                // Verify the response is correct
                utils::verify_transaction_with_proof_response(
                    use_request_v2,
                    transaction_list_with_proof,
                    response,
                );
            }
        }
    }
}

#[tokio::test]
#[should_panic(expected = "Canceled")]
async fn test_get_transactions_with_proof_disable_v2() {
    // Create a storage service config with transaction v2 disabled
    let storage_config = utils::create_storage_config(false, false);

    // Create the storage client and server
    let (mut mock_client, service, _, _, _) = MockClient::new(None, Some(storage_config));
    tokio::spawn(service.start());

    // Send a transaction v2 request. This will cause a test panic
    // as no response will be received (the receiver is dropped).
    utils::get_transactions_with_proof(
        &mut mock_client,
        0,
        10,
        10,
        true,
        true,
        true, // Use transaction v2
        storage_config.max_network_chunk_bytes_v2,
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn test_get_transactions_with_proof_invalid() {
    // Test both v1 and v2 data requests
    for use_request_v2 in [false, true] {
        // Create a storage service config
        let storage_config = utils::create_storage_config(use_request_v2, false);

        // Create the storage client and server
        let (mut mock_client, service, _, _, _) = MockClient::new(None, Some(storage_config));
        tokio::spawn(service.start());

        // Test invalid ranges
        let start_version = 1000;
        for end_version in [0, 999] {
            let response = utils::get_transactions_with_proof(
                &mut mock_client,
                start_version,
                end_version,
                end_version,
                true,
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
async fn test_get_transactions_with_proof_network_limit() {
    // Test size and time-aware chunking
    for use_size_and_time_aware_chunking in [false, true] {
        // Test both v1 and v2 data requests
        for use_request_v2 in [false, true] {
            // Test different byte limits (the client and server limits match)
            for max_specified_bytes in [1, 1024, 10 * 1024] {
                get_transactions_with_proof_network_limit(
                    max_specified_bytes,
                    max_specified_bytes,
                    use_request_v2,
                    use_size_and_time_aware_chunking,
                )
                .await;
            }
        }
    }
}

#[tokio::test]
async fn test_get_transactions_with_proof_network_limit_client_bounded() {
    // Test size and time-aware chunking
    for use_size_and_time_aware_chunking in [false, true] {
        // Test both v1 and v2 data requests
        for use_request_v2 in [false, true] {
            // Test different byte limits
            for max_specified_bytes in [1, 1024, 10 * 1024] {
                let max_server_specified_bytes = max_specified_bytes * 10; // The server limit is 10x the client limit
                get_transactions_with_proof_network_limit(
                    max_specified_bytes,
                    max_server_specified_bytes,
                    use_request_v2,
                    use_size_and_time_aware_chunking,
                )
                .await;
            }
        }
    }
}

#[tokio::test]
async fn test_get_transactions_with_proof_network_limit_server_bounded() {
    // Test size and time-aware chunking
    for use_size_and_time_aware_chunking in [false, true] {
        // Test both v1 and v2 data requests
        for use_request_v2 in [false, true] {
            // Test different byte limits
            for max_specified_bytes in [1, 1024, 10 * 1024] {
                let max_client_specified_bytes = max_specified_bytes * 10; // The client limit is 10x the server limit
                get_transactions_with_proof_network_limit(
                    max_client_specified_bytes,
                    max_specified_bytes,
                    use_request_v2,
                    use_size_and_time_aware_chunking,
                )
                .await;
            }
        }
    }
}

#[tokio::test]
async fn test_get_transactions_with_proof_not_serviceable() {
    // Test size and time-aware chunking
    for use_size_and_time_aware_chunking in [false, true] {
        // Test both v1 and v2 data requests
        for use_request_v2 in [false, true] {
            // Test small and large chunk requests
            let max_transaction_chunk_size =
                StorageServiceConfig::default().max_transaction_chunk_size;
            for chunk_size in [2, 100, max_transaction_chunk_size] {
                // Test event inclusion
                for include_events in [true, false] {
                    // Create test data
                    let start_version = 0;
                    let end_version = start_version + chunk_size - 1;
                    let proof_version = end_version;

                    // Create a storage service config
                    let storage_config = utils::create_storage_config(
                        use_request_v2,
                        use_size_and_time_aware_chunking,
                    );

                    // Create the storage client and server (that cannot service the request)
                    let (mut mock_client, mut service, _, _, _) =
                        MockClient::new(None, Some(storage_config));
                    utils::update_storage_server_summary(&mut service, proof_version - 1, 10);
                    tokio::spawn(service.start());

                    // Create a request to fetch transactions with a proof
                    let response = utils::get_transactions_with_proof(
                        &mut mock_client,
                        start_version,
                        end_version,
                        proof_version,
                        include_events,
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
    }
}

/// A helper method to request a transactions with proof chunk using
/// the specified network limits (client and server).
async fn get_transactions_with_proof_network_limit(
    max_client_specified_bytes: u64,
    max_server_specified_bytes: u64,
    use_request_v2: bool,
    use_size_and_time_aware_chunking: bool,
) {
    for use_compression in [true, false] {
        for include_events in [true, false] {
            // Create test data
            let max_transaction_chunk_size =
                StorageServiceConfig::default().max_transaction_chunk_size;
            let min_bytes_per_transaction = 512; // 0.5 KB
            let start_version = 121245;
            let proof_version = 202020;

            // Create the mock db reader
            let db_reader = create_mock_db_with_transaction_expectations(
                use_request_v2,
                use_size_and_time_aware_chunking,
                include_events,
                max_transaction_chunk_size,
                min_bytes_per_transaction,
                start_version,
                proof_version,
            );

            // Create a storage config with the specified max network byte limit
            let storage_config = StorageServiceConfig {
                max_network_chunk_bytes: max_server_specified_bytes,
                max_network_chunk_bytes_v2: max_server_specified_bytes,
                enable_transaction_data_v2: use_request_v2,
                enable_size_and_time_aware_chunking: use_size_and_time_aware_chunking,
                ..Default::default()
            };

            // Create the storage client and server
            let (mut mock_client, mut service, _, _, _) =
                MockClient::new(Some(db_reader), Some(storage_config));
            utils::update_storage_server_summary(&mut service, proof_version + 1, 10);
            tokio::spawn(service.start());

            // Process a request to fetch transactions with a proof
            let response = utils::get_transactions_with_proof(
                &mut mock_client,
                start_version,
                start_version + max_transaction_chunk_size - 1,
                proof_version,
                include_events,
                use_compression,
                use_request_v2,
                max_client_specified_bytes,
            )
            .await
            .unwrap();

            // Verify the response is correct
            let num_response_bytes = bcs::serialized_size(&response).unwrap() as u64;
            let num_transactions = match response.get_data_response().unwrap() {
                DataResponse::TransactionsWithProof(transactions_with_proof) => {
                    transactions_with_proof.get_num_transactions() as u64
                },
                DataResponse::TransactionDataWithProof(transaction_data_with_proof) => {
                    let transaction_list_with_proof_v2 = transaction_data_with_proof
                        .transaction_list_with_proof
                        .unwrap();
                    transaction_list_with_proof_v2.get_num_transactions() as u64
                },
                _ => panic!("Expected transactions with proof but got: {:?}", response),
            };
            if num_response_bytes > max_server_specified_bytes {
                assert_eq!(num_transactions, 1); // Data cannot be reduced more than a single item
            } else {
                // Determine the max specified bytes
                let max_specified_bytes = if use_request_v2 {
                    min(max_client_specified_bytes, max_server_specified_bytes)
                } else {
                    max_server_specified_bytes
                };

                // Verify the number of transactions fits within the specified byte limit
                let max_transactions = max_specified_bytes / min_bytes_per_transaction;
                assert!(num_transactions <= max_transactions);
            }
        }
    }
}

/// Creates a mock db reader with expectations for fetching transactions
fn create_mock_db_with_transaction_expectations(
    use_request_v2: bool,
    use_size_and_time_aware_chunking: bool,
    include_events: bool,
    mut chunk_size: u64,
    min_bytes_per_transaction: u64,
    start_version: u64,
    proof_version: u64,
) -> MockDatabaseReader {
    // Create the mock DB reader
    let mut db_reader = mock::create_mock_db_reader();

    // Create a transaction list using the initial chunk size
    let transaction_list_with_proof = utils::create_transaction_list_using_sizes(
        start_version,
        chunk_size,
        min_bytes_per_transaction,
        include_events,
        use_request_v2,
    );

    // If size and time-aware chunking are enabled, expect iterator usage
    if use_size_and_time_aware_chunking {
        utils::expect_get_transactions(
            &mut db_reader,
            start_version,
            chunk_size,
            proof_version,
            include_events,
            transaction_list_with_proof.clone(),
            use_size_and_time_aware_chunking,
        );
    }

    // Otherwise, expect the legacy implementation that halves the chunk size until it fits
    while chunk_size >= 1 {
        let transaction_list_with_proof = utils::create_transaction_list_using_sizes(
            start_version,
            chunk_size,
            min_bytes_per_transaction,
            include_events,
            use_request_v2,
        );

        utils::expect_get_transactions(
            &mut db_reader,
            start_version,
            chunk_size,
            proof_version,
            include_events,
            transaction_list_with_proof.clone(),
            use_size_and_time_aware_chunking,
        );

        chunk_size /= 2;
    }

    db_reader
}

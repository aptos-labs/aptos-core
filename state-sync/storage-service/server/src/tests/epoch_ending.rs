// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::tests::{
    mock,
    mock::{MockClient, MockDatabaseReader},
    utils,
};
use aptos_bitvec::BitVec;
use aptos_config::config::StorageServiceConfig;
use aptos_crypto::HashValue;
use aptos_storage_service_types::{
    requests::{DataRequest, EpochEndingLedgerInfoRequest, StorageServiceRequest},
    responses::DataResponse,
    Epoch, StorageServiceError,
};
use aptos_types::{
    aggregate_signature::AggregateSignature,
    block_info::BlockInfo,
    epoch_change::EpochChangeProof,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
};
use claims::assert_matches;
use mockall::{predicate::eq, Sequence};
use rand::Rng;

#[tokio::test]
async fn test_get_epoch_ending_ledger_infos() {
    // Test size and time-aware chunking
    for use_size_and_time_aware_chunking in [false, true] {
        // Test small and large chunk requests
        let max_epoch_chunk_size = StorageServiceConfig::default().max_epoch_chunk_size;
        for chunk_size in [1, 100, max_epoch_chunk_size] {
            // Create test data
            let start_epoch = 11;
            let expected_end_epoch = start_epoch + chunk_size - 1;
            let epoch_change_proof = EpochChangeProof {
                ledger_info_with_sigs: create_epoch_ending_ledger_infos(
                    start_epoch,
                    expected_end_epoch,
                ),
                more: false,
            };

            // Create the mock db reader
            let mut db_reader = mock::create_mock_db_reader();
            utils::expect_get_epoch_ending_ledger_infos(
                &mut db_reader,
                start_epoch,
                expected_end_epoch + 1,
                epoch_change_proof.clone(),
                use_size_and_time_aware_chunking,
            );

            // Create the storage client and server
            let storage_service_config =
                utils::create_storage_config(false, use_size_and_time_aware_chunking);
            let (mut mock_client, mut service, _, _, _) =
                MockClient::new(Some(db_reader), Some(storage_service_config));
            utils::update_storage_server_summary(&mut service, 1000, expected_end_epoch);
            tokio::spawn(service.start());

            // Create a request to fetch epoch ending ledger infos
            let data_request =
                DataRequest::GetEpochEndingLedgerInfos(EpochEndingLedgerInfoRequest {
                    start_epoch,
                    expected_end_epoch,
                });
            let storage_request = StorageServiceRequest::new(data_request, true);

            // Process the request
            let response = mock_client.process_request(storage_request).await.unwrap();

            // Verify the response is correct
            match response.get_data_response().unwrap() {
                DataResponse::EpochEndingLedgerInfos(response_epoch_change_proof) => {
                    assert_eq!(response_epoch_change_proof, epoch_change_proof)
                },
                _ => panic!("Expected epoch ending ledger infos but got: {:?}", response),
            };
        }
    }
}

#[tokio::test]
async fn test_get_epoch_ending_ledger_infos_chunk_limit() {
    // Test size and time-aware chunking
    for use_size_and_time_aware_chunking in [false, true] {
        // Create test data
        let max_epoch_chunk_size = StorageServiceConfig::default().max_epoch_chunk_size;
        let chunk_size = max_epoch_chunk_size * 10; // Set a chunk request larger than the max
        let start_epoch = 11;
        let expected_end_epoch = start_epoch + max_epoch_chunk_size - 1;
        let epoch_change_proof = EpochChangeProof {
            ledger_info_with_sigs: create_epoch_ending_ledger_infos(
                start_epoch,
                expected_end_epoch,
            ),
            more: false,
        };

        // Create the mock db reader
        let mut db_reader = mock::create_mock_db_reader();
        utils::expect_get_epoch_ending_ledger_infos(
            &mut db_reader,
            start_epoch,
            expected_end_epoch + 1,
            epoch_change_proof.clone(),
            use_size_and_time_aware_chunking,
        );

        // Create a request to fetch epoch ending ledger infos
        let expected_end_epoch = start_epoch + chunk_size - 1;
        let data_request = DataRequest::GetEpochEndingLedgerInfos(EpochEndingLedgerInfoRequest {
            start_epoch,
            expected_end_epoch,
        });
        let storage_request = StorageServiceRequest::new(data_request, true);

        // Create the storage client and server
        let storage_service_config =
            utils::create_storage_config(false, use_size_and_time_aware_chunking);
        let (mut mock_client, mut service, _, _, _) =
            MockClient::new(Some(db_reader), Some(storage_service_config));
        utils::update_storage_server_summary(&mut service, 1000, expected_end_epoch);
        tokio::spawn(service.start());

        // Process the request
        let response = mock_client.process_request(storage_request).await.unwrap();

        // Verify the response is correct
        match response.get_data_response().unwrap() {
            DataResponse::EpochEndingLedgerInfos(response_epoch_change_proof) => {
                assert_eq!(response_epoch_change_proof, epoch_change_proof)
            },
            _ => panic!("Expected epoch ending ledger infos but got: {:?}", response),
        };
    }
}

#[tokio::test]
async fn test_get_epoch_ending_ledger_infos_invalid() {
    // Create the storage client and server
    let (mut mock_client, service, _, _, _) = MockClient::new(None, None);
    tokio::spawn(service.start());

    // Test invalid ranges
    let start_epoch = 11;
    for expected_end_epoch in [0, 10] {
        let data_request = DataRequest::GetEpochEndingLedgerInfos(EpochEndingLedgerInfoRequest {
            start_epoch,
            expected_end_epoch,
        });
        let storage_request = StorageServiceRequest::new(data_request, true);

        // Process and verify the response
        let response = mock_client
            .process_request(storage_request)
            .await
            .unwrap_err();
        assert_matches!(response, StorageServiceError::InvalidRequest(_));
    }
}

#[tokio::test]
async fn test_get_epoch_ending_ledger_infos_network_limit() {
    // Test different byte limits
    for network_limit_bytes in [1, 10 * 1024, 50 * 1024, 100 * 1024, 1024 * 1024] {
        get_epoch_ending_ledger_infos_network_limit(network_limit_bytes).await;
    }
}

#[tokio::test]
async fn test_get_epoch_ending_ledger_infos_not_serviceable() {
    // Test small and large chunk requests
    let max_epoch_chunk_size = StorageServiceConfig::default().max_epoch_chunk_size;
    for chunk_size in [1, 100, max_epoch_chunk_size] {
        // Create test data
        let start_epoch = 11;
        let expected_end_epoch = start_epoch + chunk_size - 1;

        // Create the storage client and server (that cannot service the request)
        let (mut mock_client, mut service, _, _, _) = MockClient::new(None, None);
        utils::update_storage_server_summary(&mut service, 1000, expected_end_epoch - 1);
        tokio::spawn(service.start());

        // Create a request to fetch epoch ending ledger infos
        let data_request = DataRequest::GetEpochEndingLedgerInfos(EpochEndingLedgerInfoRequest {
            start_epoch,
            expected_end_epoch,
        });
        let storage_request = StorageServiceRequest::new(data_request, true);

        // Process the request
        let response = mock_client
            .process_request(storage_request)
            .await
            .unwrap_err();

        // Verify the request is not serviceable
        assert_matches!(response, StorageServiceError::InvalidRequest(_));
    }
}

/// Creates a test epoch change proof
fn create_epoch_ending_ledger_infos(
    start_epoch: Epoch,
    end_epoch: Epoch,
) -> Vec<LedgerInfoWithSignatures> {
    let mut ledger_info_with_sigs = vec![];
    for epoch in start_epoch..end_epoch {
        ledger_info_with_sigs.push(utils::create_test_ledger_info_with_sigs(epoch, 0));
    }
    ledger_info_with_sigs
}

/// Creates a test epoch change proof with the given sizes
fn create_epoch_ending_ledger_infos_using_sizes(
    num_ledger_infos: u64,
    min_bytes_per_ledger_info: u64,
) -> Vec<LedgerInfoWithSignatures> {
    // Create a mock ledger info
    let ledger_info = LedgerInfo::new(
        BlockInfo::new(0, 0, HashValue::zero(), HashValue::zero(), 0, 0, None),
        HashValue::zero(),
    );

    // Generate random bytes of the given size
    let mut rng = rand::thread_rng();
    let random_bytes: Vec<u8> = (0..min_bytes_per_ledger_info)
        .map(|_| rng.gen::<u8>())
        .collect();

    // Create the ledger infos with signatures
    (0..num_ledger_infos)
        .map(|_| {
            let multi_signatures =
                AggregateSignature::new(BitVec::from(random_bytes.clone()), None);
            LedgerInfoWithSignatures::new(ledger_info.clone(), multi_signatures)
        })
        .collect()
}

/// A helper method to request a states with proof chunk using the
/// the specified network limit.
async fn get_epoch_ending_ledger_infos_network_limit(network_limit_bytes: u64) {
    // Test size and time-aware chunking
    for use_size_and_time_aware_chunking in [false, true] {
        for use_compression in [true, false] {
            // Create test data
            let max_epoch_chunk_size = StorageServiceConfig::default().max_epoch_chunk_size;
            let min_bytes_per_ledger_info = 5000;
            let start_epoch = 98754;
            let expected_end_epoch = start_epoch + max_epoch_chunk_size - 1;

            // Create the mock db reader
            let db_reader = create_mock_db_with_epoch_ending_expectations(
                max_epoch_chunk_size,
                min_bytes_per_ledger_info,
                start_epoch,
                use_size_and_time_aware_chunking,
            );

            // Create a storage config with the specified max network byte limit
            let storage_config = StorageServiceConfig {
                max_network_chunk_bytes: network_limit_bytes,
                enable_size_and_time_aware_chunking: use_size_and_time_aware_chunking,
                ..Default::default()
            };

            // Create the storage client and server
            let (mut mock_client, mut service, _, _, _) =
                MockClient::new(Some(db_reader), Some(storage_config));
            utils::update_storage_server_summary(&mut service, 1000, expected_end_epoch);
            tokio::spawn(service.start());

            // Process a request to fetch epoch ending ledger infos
            let data_request =
                DataRequest::GetEpochEndingLedgerInfos(EpochEndingLedgerInfoRequest {
                    start_epoch,
                    expected_end_epoch,
                });
            let storage_request = StorageServiceRequest::new(data_request, use_compression);
            let response = mock_client.process_request(storage_request).await.unwrap();

            // Verify the response adheres to the network limits
            match response.get_data_response().unwrap() {
                DataResponse::EpochEndingLedgerInfos(epoch_change_proof) => {
                    let num_response_bytes = bcs::serialized_size(&response).unwrap() as u64;
                    let num_ledger_infos = epoch_change_proof.ledger_info_with_sigs.len() as u64;
                    if num_response_bytes > network_limit_bytes {
                        assert_eq!(num_ledger_infos, 1); // Data cannot be reduced more than a single item
                    } else {
                        let max_num_ledger_infos = network_limit_bytes / min_bytes_per_ledger_info;
                        assert!(num_ledger_infos <= max_num_ledger_infos); // Verify data fits correctly into the limit
                    }
                },
                _ => panic!("Expected epoch ending ledger infos but got: {:?}", response),
            }
        }
    }
}

/// Creates a mock db reader with expectations for fetching epoch ending ledger infos
fn create_mock_db_with_epoch_ending_expectations(
    mut chunk_size: u64,
    min_bytes_per_ledger_info: u64,
    start_epoch: u64,
    use_size_and_time_aware_chunking: bool,
) -> MockDatabaseReader {
    // Create the mock DB reader
    let mut db_reader = mock::create_mock_db_reader();

    // Create an epoch change proof with the initial chunk size
    let ledger_info_with_sigs =
        create_epoch_ending_ledger_infos_using_sizes(chunk_size, min_bytes_per_ledger_info);
    let epoch_change_proof = EpochChangeProof {
        ledger_info_with_sigs,
        more: false,
    };

    // If size and time-aware chunking are enabled, expect iterator usage
    if use_size_and_time_aware_chunking {
        utils::expect_get_epoch_ending_ledger_infos(
            &mut db_reader,
            start_epoch,
            start_epoch + chunk_size,
            epoch_change_proof.clone(),
            use_size_and_time_aware_chunking,
        );
        return db_reader;
    }

    // Otherwise, expect the legacy implementation that halves the chunk size until it fits
    let mut expectation_sequence = Sequence::new();
    while chunk_size >= 1 {
        let ledger_info_with_sigs =
            create_epoch_ending_ledger_infos_using_sizes(chunk_size, min_bytes_per_ledger_info);
        let epoch_change_proof = EpochChangeProof {
            ledger_info_with_sigs,
            more: false,
        };

        db_reader
            .expect_get_epoch_ending_ledger_infos()
            .times(1)
            .with(eq(start_epoch), eq(start_epoch + chunk_size))
            .in_sequence(&mut expectation_sequence)
            .returning(move |_, _| Ok(epoch_change_proof.clone()));

        chunk_size /= 2;
    }

    db_reader
}

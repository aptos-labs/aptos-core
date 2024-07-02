// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::tests::{mock, mock::MockClient, utils};
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
use mockall::predicate::eq;
use rand::Rng;

#[tokio::test]
async fn test_get_epoch_ending_ledger_infos() {
    // Test small and large chunk requests
    let max_epoch_chunk_size = StorageServiceConfig::default().max_epoch_chunk_size;
    for chunk_size in [1, 100, max_epoch_chunk_size] {
        // Create test data
        let start_epoch = 11;
        let expected_end_epoch = start_epoch + chunk_size - 1;
        let ledger_info_with_sigs =
            create_epoch_ending_ledger_infos(start_epoch, expected_end_epoch);
        let epoch_change_proof = EpochChangeProof {
            ledger_info_with_sigs,
            more: false,
        };

        // Create the mock db reader
        let mut db_reader = mock::create_mock_db_reader();
        utils::expect_get_epoch_ending_ledger_infos(
            &mut db_reader,
            start_epoch,
            expected_end_epoch,
            epoch_change_proof.clone(),
        );

        // Create the storage client and server
        let (mut mock_client, mut service, _, _, _) = MockClient::new(Some(db_reader), None);
        utils::update_storage_server_summary(&mut service, 1000, expected_end_epoch);
        tokio::spawn(service.start());

        // Create a request to fetch epoch ending ledger infos
        let data_request = DataRequest::GetEpochEndingLedgerInfos(EpochEndingLedgerInfoRequest {
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

#[tokio::test]
async fn test_get_epoch_ending_ledger_infos_chunk_limit() {
    // Create test data
    let max_epoch_chunk_size = StorageServiceConfig::default().max_epoch_chunk_size;
    let chunk_size = max_epoch_chunk_size * 10; // Set a chunk request larger than the max
    let start_epoch = 11;
    let expected_end_epoch = start_epoch + max_epoch_chunk_size - 1;
    let ledger_info_with_sigs = create_epoch_ending_ledger_infos(start_epoch, expected_end_epoch);
    let epoch_change_proof = EpochChangeProof {
        ledger_info_with_sigs,
        more: false,
    };

    // Create the mock db reader
    let mut db_reader = mock::create_mock_db_reader();
    utils::expect_get_epoch_ending_ledger_infos(
        &mut db_reader,
        start_epoch,
        expected_end_epoch,
        epoch_change_proof.clone(),
    );

    // Create a request to fetch epoch ending ledger infos
    let expected_end_epoch = start_epoch + chunk_size - 1;
    let data_request = DataRequest::GetEpochEndingLedgerInfos(EpochEndingLedgerInfoRequest {
        start_epoch,
        expected_end_epoch,
    });
    let storage_request = StorageServiceRequest::new(data_request, true);

    // Create the storage client and server
    let (mut mock_client, mut service, _, _, _) = MockClient::new(Some(db_reader), None);
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
    for network_limit_bytes in [1, 1024, 10 * 1024, 50 * 1024, 100 * 1024] {
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
    for epoch in start_epoch..=end_epoch {
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

/// A helper method to request a states with proof chunk using
/// the specified network limit.
async fn get_epoch_ending_ledger_infos_network_limit(network_limit_bytes: u64) {
    for use_compression in [true, false] {
        // Create test data
        let max_epoch_chunk_size = StorageServiceConfig::default().max_epoch_chunk_size;
        let min_bytes_per_ledger_info = 5000;
        let start_epoch = 98754;
        let expected_end_epoch = start_epoch + max_epoch_chunk_size - 1;

        // Create an iterator that returns the relevant epoch ending ledger infos
        let epoch_ending_ledger_infos = create_epoch_ending_ledger_infos_using_sizes(
            max_epoch_chunk_size,
            min_bytes_per_ledger_info,
        );
        let ledger_info_iterator = Box::new(epoch_ending_ledger_infos.into_iter().map(Ok))
            as Box<
                dyn Iterator<Item = aptos_storage_interface::Result<LedgerInfoWithSignatures>>
                    + Send,
            >;

        // Create the mock db reader with expectations
        let mut db_reader = mock::create_mock_db_reader();
        let end_epoch = expected_end_epoch + 1; // The end epoch is exclusive in DbReader
        db_reader
            .expect_get_epoch_ending_ledger_info_iterator()
            .times(1)
            .with(eq(start_epoch), eq(end_epoch))
            .return_once(move |_, _| Ok(ledger_info_iterator));

        // Create a storage config with the specified max network byte limit
        let storage_config = StorageServiceConfig {
            max_network_chunk_bytes: network_limit_bytes,
            ..Default::default()
        };

        // Create the storage client and server
        let (mut mock_client, mut service, _, _, _) =
            MockClient::new(Some(db_reader), Some(storage_config));
        utils::update_storage_server_summary(&mut service, 1000, expected_end_epoch);
        tokio::spawn(service.start());

        // Process a request to fetch epoch ending ledger infos
        let data_request = DataRequest::GetEpochEndingLedgerInfos(EpochEndingLedgerInfoRequest {
            start_epoch,
            expected_end_epoch,
        });
        let storage_request = StorageServiceRequest::new(data_request, use_compression);
        let response = mock_client.process_request(storage_request).await.unwrap();

        // Verify the response adheres to the network limits
        match response.get_data_response().unwrap() {
            DataResponse::EpochEndingLedgerInfos(epoch_change_proof) => {
                let num_response_bytes =
                    utils::get_num_serialized_bytes(&response.get_data_response());
                if num_response_bytes > network_limit_bytes {
                    // Verify the ledger infos are larger than the network limit
                    let epoch_ending_ledger_infos =
                        epoch_change_proof.ledger_info_with_sigs.clone();
                    let num_ledger_info_bytes =
                        utils::get_num_serialized_bytes(&epoch_ending_ledger_infos);
                    assert!(num_ledger_info_bytes > network_limit_bytes);

                    // Verify the response is only 1 ledger info over the network limit
                    let epoch_ending_ledger_infos =
                        &epoch_ending_ledger_infos[0..epoch_ending_ledger_infos.len() - 1];
                    let num_ledger_info_bytes =
                        utils::get_num_serialized_bytes(epoch_ending_ledger_infos);
                    assert!(num_ledger_info_bytes <= network_limit_bytes);
                } else {
                    // Verify data fits correctly into the limit
                    let num_ledger_infos = epoch_change_proof.ledger_info_with_sigs.len() as u64;
                    let max_num_ledger_infos = network_limit_bytes / min_bytes_per_ledger_info;
                    assert!(num_ledger_infos <= max_num_ledger_infos);
                }
            },
            _ => panic!("Expected epoch ending ledger infos but got: {:?}", response),
        }
    }
}

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::tests::{
    mock,
    mock::{MockClient, MockDatabaseReader},
    utils,
};
use aptos_config::config::StorageServiceConfig;
use aptos_crypto::hash::HashValue;
use aptos_storage_service_types::{
    requests::{DataRequest, StateValuesWithProofRequest},
    responses::{DataResponse, StorageServiceResponse},
    StorageServiceError,
};
use aptos_types::{
    proof::definition::SparseMerkleRangeProof,
    state_store::{
        state_key::StateKey,
        state_value::{StateValue, StateValueChunkWithProof},
    },
    transaction::Version,
};
use bytes::Bytes;
use claims::assert_matches;
use mockall::{
    predicate::{always, eq},
    Sequence,
};
use rand::Rng;

#[tokio::test]
async fn test_get_states_with_proof() {
    // Test size and time-aware chunking
    for use_size_and_time_aware_chunking in [false, true] {
        // Test small and large chunk requests
        let max_state_chunk_size = StorageServiceConfig::default().max_state_chunk_size;
        for chunk_size in [1, 100, max_state_chunk_size] {
            // Create test data
            let version = 101;
            let start_index = 100;
            let end_index = start_index + chunk_size - 1;
            let state_value_chunk_with_proof = StateValueChunkWithProof {
                first_index: start_index,
                last_index: end_index,
                first_key: HashValue::random(),
                last_key: HashValue::random(),
                raw_values: vec![],
                proof: SparseMerkleRangeProof::new(vec![]),
                root_hash: HashValue::random(),
            };

            // Create the mock db reader
            let mut db_reader = mock::create_mock_db_reader();
            expect_get_state_values_with_proof(
                &mut db_reader,
                version,
                start_index,
                chunk_size,
                state_value_chunk_with_proof.clone(),
                use_size_and_time_aware_chunking,
            );

            // Create a storage service config
            let storage_config =
                utils::create_storage_config(false, use_size_and_time_aware_chunking);

            // Create the storage client and server
            let (mut mock_client, mut service, _, _, _) =
                MockClient::new(Some(db_reader), Some(storage_config));
            utils::update_storage_server_summary(&mut service, version, 10);
            tokio::spawn(service.start());

            // Process a request to fetch a states chunk with a proof
            let response = get_state_values_with_proof(
                &mut mock_client,
                version,
                start_index,
                end_index,
                false,
            )
            .await
            .unwrap();

            // Verify the response is correct
            assert_matches!(response, StorageServiceResponse::RawResponse(_));
            assert_eq!(
                response.get_data_response().unwrap(),
                DataResponse::StateValueChunkWithProof(state_value_chunk_with_proof)
            );
        }
    }
}

#[tokio::test]
async fn test_get_states_with_proof_chunk_limit() {
    // Test size and time-aware chunking
    for use_size_and_time_aware_chunking in [false, true] {
        // Create test data
        let max_state_chunk_size = StorageServiceConfig::default().max_state_chunk_size;
        let chunk_size = max_state_chunk_size * 10; // Set a chunk request larger than the max
        let version = 101;
        let start_index = 100;
        let state_value_chunk_with_proof = StateValueChunkWithProof {
            first_index: start_index,
            last_index: start_index + max_state_chunk_size - 1,
            first_key: HashValue::random(),
            last_key: HashValue::random(),
            raw_values: vec![],
            proof: SparseMerkleRangeProof::new(vec![]),
            root_hash: HashValue::random(),
        };

        // Create the mock db reader
        let mut db_reader = mock::create_mock_db_reader();
        expect_get_state_values_with_proof(
            &mut db_reader,
            version,
            start_index,
            max_state_chunk_size,
            state_value_chunk_with_proof.clone(),
            use_size_and_time_aware_chunking,
        );

        // Create a storage service config
        let storage_config = utils::create_storage_config(false, use_size_and_time_aware_chunking);

        // Create the storage client and server
        let (mut mock_client, mut service, _, _, _) =
            MockClient::new(Some(db_reader), Some(storage_config));
        utils::update_storage_server_summary(&mut service, version, 10);
        tokio::spawn(service.start());

        // Process a request to fetch a states chunk with a proof
        let response = get_state_values_with_proof(
            &mut mock_client,
            version,
            start_index,
            start_index + chunk_size - 1,
            false,
        )
        .await
        .unwrap();

        // Verify the response is correct
        assert_matches!(response, StorageServiceResponse::RawResponse(_));
        assert_eq!(
            response.get_data_response().unwrap(),
            DataResponse::StateValueChunkWithProof(state_value_chunk_with_proof)
        );
    }
}

#[tokio::test]
async fn test_get_states_with_proof_invalid() {
    // Create the storage client and server
    let (mut mock_client, service, _, _, _) = MockClient::new(None, None);
    tokio::spawn(service.start());

    // Test invalid ranges
    let start_index = 100;
    for end_index in [0, 99] {
        let response =
            get_state_values_with_proof(&mut mock_client, 0, start_index, end_index, false)
                .await
                .unwrap_err();
        assert_matches!(response, StorageServiceError::InvalidRequest(_));
    }
}

#[tokio::test]
async fn test_get_states_with_proof_network_limit() {
    // Test different byte limits
    for network_limit_bytes in [1, 500, 2500, 5500, 10 * 1024] {
        get_states_with_proof_network_limit(network_limit_bytes).await;
    }
}

#[tokio::test]
async fn test_get_states_with_proof_not_serviceable() {
    // Test small and large chunk requests
    let max_state_chunk_size = StorageServiceConfig::default().max_state_chunk_size;
    for chunk_size in [1, 100, max_state_chunk_size] {
        // Create test data
        let version = 101;
        let start_index = 100;
        let end_index = start_index + chunk_size - 1;

        // Create the storage client and server (that cannot service the request)
        let (mut mock_client, mut service, _, _, _) = MockClient::new(None, None);
        utils::update_storage_server_summary(&mut service, version - 1, 10);
        tokio::spawn(service.start());

        // Process a request to fetch a states chunk with a proof
        let response =
            get_state_values_with_proof(&mut mock_client, version, start_index, end_index, false)
                .await
                .unwrap_err();

        // Verify the request is not serviceable
        assert_matches!(response, StorageServiceError::InvalidRequest(_));
    }
}

/// Creates a set of state keys and values using the specified number and size
fn create_state_keys_and_values(
    num_keys_and_values: u64,
    min_bytes_per_key_value: u64,
) -> Vec<(StateKey, StateValue)> {
    // Generate random bytes of the given size
    let mut rng = rand::thread_rng();
    let random_bytes: Bytes = (0..min_bytes_per_key_value)
        .map(|_| rng.gen::<u8>())
        .collect::<Vec<_>>()
        .into();

    // Create the requested keys and values
    (0..num_keys_and_values)
        .map(|_| {
            let state_value = StateValue::new_legacy(random_bytes.clone());
            (StateKey::raw(&[]), state_value)
        })
        .collect()
}

/// Creates a state value chunk with proof
fn create_state_value_chunk_with_proof(
    start_index: u64,
    chunk_size: u64,
    min_bytes_per_state_value: u64,
) -> StateValueChunkWithProof {
    StateValueChunkWithProof {
        first_index: start_index,
        last_index: start_index + chunk_size - 1,
        first_key: HashValue::random(),
        last_key: HashValue::random(),
        raw_values: create_state_keys_and_values(chunk_size, min_bytes_per_state_value),
        proof: SparseMerkleRangeProof::new(vec![]),
        root_hash: HashValue::random(),
    }
}

/// Sets an expectation on the given mock db for a call to fetch state values with proof
fn expect_get_state_values_with_proof(
    mock_db: &mut MockDatabaseReader,
    version: u64,
    start_index: u64,
    chunk_size: u64,
    mut state_value_chunk_with_proof: StateValueChunkWithProof,
    use_size_and_time_aware_chunking: bool,
) {
    // If size and time-aware chunking are disabled, expect the legacy implementation
    if !use_size_and_time_aware_chunking {
        mock_db
            .expect_get_state_value_chunk_with_proof()
            .times(1)
            .with(
                eq(version),
                eq(start_index as usize),
                eq(chunk_size as usize),
            )
            .returning(move |_, _, _| Ok(state_value_chunk_with_proof.clone()));
        return;
    }

    // Expect a call to get a state value iterator
    let mut expectation_sequence = Sequence::new();
    let state_value_iterator = state_value_chunk_with_proof
        .clone()
        .raw_values
        .into_iter()
        .map(Ok);
    mock_db
        .expect_get_state_value_chunk_iter()
        .times(1)
        .with(
            eq(version),
            eq(start_index as usize),
            eq(chunk_size as usize),
        )
        .returning(move |_, _, _| Ok(Box::new(state_value_iterator.clone())))
        .in_sequence(&mut expectation_sequence);

    // Expect a call to get the state value chunk proof
    mock_db
        .expect_get_state_value_chunk_proof()
        .times(1)
        .with(eq(version), eq(start_index as usize), always())
        .return_once(move |_, _, given_state_values| {
            state_value_chunk_with_proof.raw_values = given_state_values;
            Ok(state_value_chunk_with_proof)
        })
        .in_sequence(&mut expectation_sequence);
}

/// Sends a state values with proof request and processes the response
async fn get_state_values_with_proof(
    mock_client: &mut MockClient,
    version: u64,
    start_index: u64,
    end_index: u64,
    use_compression: bool,
) -> Result<StorageServiceResponse, StorageServiceError> {
    let data_request = DataRequest::GetStateValuesWithProof(StateValuesWithProofRequest {
        version,
        start_index,
        end_index,
    });
    utils::send_storage_request(mock_client, use_compression, data_request).await
}

/// A helper method to request a states with proof chunk using the
/// the specified network limit.
async fn get_states_with_proof_network_limit(network_limit_bytes: u64) {
    // Test size and time-aware chunking
    for use_size_and_time_aware_chunking in [false, true] {
        for use_compression in [true, false] {
            // Create test data
            let max_state_chunk_size = StorageServiceConfig::default().max_state_chunk_size;
            let min_bytes_per_state_value = 1000;
            let version = 101;
            let start_index = 100;

            // Create the mock db reader
            let db_reader = create_mock_db_with_state_value_expectations(
                max_state_chunk_size,
                min_bytes_per_state_value,
                version,
                start_index,
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
            utils::update_storage_server_summary(&mut service, version, 10);
            tokio::spawn(service.start());

            // Process a request to fetch a states chunk with a proof
            let response = get_state_values_with_proof(
                &mut mock_client,
                version,
                start_index,
                start_index + max_state_chunk_size + 1000, // Request more than the max chunk
                use_compression,
            )
            .await
            .unwrap();

            // Verify the response adheres to the network limits
            match response.get_data_response().unwrap() {
                DataResponse::StateValueChunkWithProof(state_value_chunk_with_proof) => {
                    let num_response_bytes = bcs::serialized_size(&response).unwrap() as u64;
                    let num_state_values = state_value_chunk_with_proof.raw_values.len() as u64;
                    if num_response_bytes > network_limit_bytes {
                        assert_eq!(num_state_values, 1); // Data cannot be reduced more than a single item
                    } else {
                        let max_num_state_values = network_limit_bytes / min_bytes_per_state_value;
                        assert!(num_state_values <= max_num_state_values); // Verify data fits correctly into the limit
                    }
                },
                _ => panic!("Expected state values with proof but got: {:?}", response),
            }
        }
    }
}

/// Creates a mock db reader with expectations for fetching state values
fn create_mock_db_with_state_value_expectations(
    mut chunk_size: u64,
    min_bytes_per_state_value: u64,
    version: Version,
    start_index: u64,
    use_size_and_time_aware_chunking: bool,
) -> MockDatabaseReader {
    // Create the mock DB reader
    let mut db_reader = mock::create_mock_db_reader();

    // Create a state value chunk with proof using the initial chunk size
    let state_value_chunk_with_proof =
        create_state_value_chunk_with_proof(start_index, chunk_size, min_bytes_per_state_value);

    // If size and time-aware chunking are enabled, expect iterator usage
    if use_size_and_time_aware_chunking {
        expect_get_state_values_with_proof(
            &mut db_reader,
            version,
            start_index,
            chunk_size,
            state_value_chunk_with_proof.clone(),
            use_size_and_time_aware_chunking,
        );
        return db_reader;
    }

    // Otherwise, expect the legacy implementation that halves the chunk size until it fits
    let mut expectation_sequence = Sequence::new();
    while chunk_size >= 1 {
        let state_value_chunk_with_proof =
            create_state_value_chunk_with_proof(start_index, chunk_size, min_bytes_per_state_value);

        db_reader
            .expect_get_state_value_chunk_with_proof()
            .times(1)
            .with(
                eq(version),
                eq(start_index as usize),
                eq(chunk_size as usize),
            )
            .in_sequence(&mut expectation_sequence)
            .returning(move |_, _, _| Ok(state_value_chunk_with_proof.clone()));

        chunk_size /= 2;
    }

    db_reader
}

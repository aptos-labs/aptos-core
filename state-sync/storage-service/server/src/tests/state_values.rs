// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::tests::{mock, mock::MockClient, utils};
use aptos_config::config::StorageServiceConfig;
use aptos_crypto::hash::HashValue;
use aptos_infallible::Mutex;
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
};
use bytes::Bytes;
use claims::assert_matches;
use mockall::predicate::{always, eq};
use rand::Rng;
use std::sync::Arc;

#[tokio::test]
async fn test_get_states_with_proof() {
    // Test small and large chunk requests
    let max_state_chunk_size = StorageServiceConfig::default().max_state_chunk_size;
    for chunk_size in [1, 100, max_state_chunk_size] {
        // Create test data
        let version = 101;
        let start_index = 100;
        let end_index = start_index + chunk_size - 1;
        let raw_values = create_state_keys_and_values(chunk_size, 1);
        let state_value_chunk_with_proof = StateValueChunkWithProof {
            first_index: start_index,
            last_index: end_index,
            first_key: HashValue::random(),
            last_key: HashValue::random(),
            raw_values,
            proof: SparseMerkleRangeProof::new(vec![]),
            root_hash: HashValue::random(),
        };

        // Create the mock db reader
        let mut db_reader = mock::create_mock_db_reader();
        utils::expect_get_state_values_with_proof(
            &mut db_reader,
            version,
            start_index,
            chunk_size,
            state_value_chunk_with_proof.clone(),
        );

        // Create the storage client and server
        let (mut mock_client, mut service, _, _, _) = MockClient::new(Some(db_reader), None);
        utils::update_storage_server_summary(&mut service, version, 10);
        tokio::spawn(service.start());

        // Process a request to fetch a states chunk with a proof
        let response =
            get_state_values_with_proof(&mut mock_client, version, start_index, end_index, false)
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
async fn test_get_states_with_proof_chunk_limit() {
    // Create test data
    let max_state_chunk_size = StorageServiceConfig::default().max_state_chunk_size;
    let chunk_size = max_state_chunk_size * 10; // Set a chunk request larger than the max
    let version = 101;
    let start_index = 100;
    let raw_values = create_state_keys_and_values(max_state_chunk_size, 1);
    let state_value_chunk_with_proof = StateValueChunkWithProof {
        first_index: start_index,
        last_index: start_index + max_state_chunk_size - 1,
        first_key: HashValue::random(),
        last_key: HashValue::random(),
        raw_values,
        proof: SparseMerkleRangeProof::new(vec![]),
        root_hash: HashValue::random(),
    };

    // Create the mock db reader
    let mut db_reader = mock::create_mock_db_reader();
    utils::expect_get_state_values_with_proof(
        &mut db_reader,
        version,
        start_index,
        max_state_chunk_size,
        state_value_chunk_with_proof.clone(),
    );

    // Create the storage client and server
    let (mut mock_client, mut service, _, _, _) = MockClient::new(Some(db_reader), None);
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
    for network_limit_bytes in [1, 1024, 10 * 1024, 50 * 1024, 100 * 1024] {
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

/// A helper method to request a states with proof chunk using
/// the specified network limit.
async fn get_states_with_proof_network_limit(network_limit_bytes: u64) {
    for use_compression in [true, false] {
        // Create test data
        let max_state_chunk_size = StorageServiceConfig::default().max_state_chunk_size;
        let min_bytes_per_state_value = 10_000;
        let version = 101;
        let start_index = 100;
        let end_index = start_index + max_state_chunk_size - 1;

        // Create an iterator that returns the relevant state values
        let state_values =
            create_state_keys_and_values(max_state_chunk_size, min_bytes_per_state_value);
        let state_value_iterator = Box::new(state_values.clone().into_iter().map(Ok))
            as Box<
                dyn Iterator<Item = aptos_storage_interface::Result<(StateKey, StateValue)>>
                    + Send
                    + Sync,
            >;

        // Create the mock db reader with expectations for the state value iterator
        let mut db_reader = mock::create_mock_db_reader();
        db_reader
            .expect_get_state_value_chunk_iter()
            .times(1)
            .with(
                eq(version),
                eq(start_index as usize),
                eq(max_state_chunk_size as usize),
            )
            .return_once(move |_, _, _| Ok(state_value_iterator));

        // Create a shared object to store the state values found by the db reader
        let shared_state_values = Arc::new(Mutex::new(state_values));
        let state_values = shared_state_values.clone();

        // Set expectations for the chunk proof
        db_reader
            .expect_get_state_value_chunk_proof()
            .times(1)
            .with(eq(version), eq(start_index as usize), always())
            .return_once(move |_, given_start_index, given_state_values| {
                // Save the state values for verification
                *state_values.lock() = given_state_values.clone();

                // Return a state value chunk with proof
                let end_index = given_start_index + given_state_values.len() - 1;
                let state_value_chunk_with_proof = StateValueChunkWithProof {
                    first_index: given_start_index as u64,
                    last_index: end_index as u64,
                    first_key: HashValue::random(),
                    last_key: HashValue::random(),
                    raw_values: given_state_values,
                    proof: SparseMerkleRangeProof::new(vec![]),
                    root_hash: HashValue::random(),
                };
                Ok(state_value_chunk_with_proof)
            });

        // Create a storage config with the specified max network byte limit
        let storage_config = StorageServiceConfig {
            max_network_chunk_bytes: network_limit_bytes,
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
            end_index,
            use_compression,
        )
        .await
        .unwrap();

        // Verify the response adheres to the network limits
        match response.get_data_response().unwrap() {
            DataResponse::StateValueChunkWithProof(mut state_value_chunk_with_proof) => {
                // Update the chunk with the shared state values
                state_value_chunk_with_proof.raw_values = shared_state_values.lock().clone();

                // Verify the response
                let num_response_bytes =
                    utils::get_num_serialized_bytes(&response.get_data_response());
                if num_response_bytes > network_limit_bytes {
                    // Verify the state values are larger than the network limit
                    let state_values = state_value_chunk_with_proof.raw_values.clone();
                    let num_state_value_bytes = utils::get_num_serialized_bytes(&state_values);
                    assert!(num_state_value_bytes > network_limit_bytes);

                    // Verify the response is only 1 state value over the network limit
                    let state_values = &state_values[0..state_values.len() - 1];
                    let num_state_value_bytes = utils::get_num_serialized_bytes(&state_values);
                    assert!(num_state_value_bytes <= network_limit_bytes);
                } else {
                    // Verify data fits correctly into the limit
                    let num_state_values = state_value_chunk_with_proof.raw_values.len() as u64;
                    let max_num_state_values = network_limit_bytes / min_bytes_per_state_value;
                    assert!(num_state_values <= max_num_state_values);
                }
            },
            _ => panic!("Expected state values with proof but got: {:?}", response),
        }
    }
}

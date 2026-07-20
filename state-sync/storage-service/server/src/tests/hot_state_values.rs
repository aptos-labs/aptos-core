// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::{
    mock,
    mock::{MockClient, MockDatabaseReader},
    utils,
};
use aptos_config::config::StorageServiceConfig;
use aptos_crypto::hash::HashValue;
use aptos_storage_service_types::{
    requests::{DataRequest, HotStateValuesWithProofRequest},
    responses::{DataResponse, StorageServiceResponse},
    StorageServiceError,
};
use aptos_types::{
    proof::definition::SparseMerkleRangeProof, state_store::hot_state::HotStateValueChunkWithProof,
};
use claims::assert_matches;
use mockall::{
    predicate::{always, eq},
    Sequence,
};

#[tokio::test]
async fn test_get_hot_state_values_with_proof() {
    // Test size and time-aware chunking
    for use_size_and_time_aware_chunking in [false, true] {
        // Test small and large chunk requests
        let max_state_chunk_size = StorageServiceConfig::default().max_state_chunk_size;
        for chunk_size in [1, 100, max_state_chunk_size] {
            // Create test data
            let version = 101;
            let start_index = 100;
            let end_index = start_index + chunk_size - 1;
            let hot_state_value_chunk_with_proof = HotStateValueChunkWithProof {
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
            expect_get_hot_state_values_with_proof(
                &mut db_reader,
                version,
                start_index,
                chunk_size,
                hot_state_value_chunk_with_proof.clone(),
            );

            // Create a storage service config
            let storage_config =
                utils::create_storage_config(false, use_size_and_time_aware_chunking);

            // Create the storage client and server
            let (mut mock_client, mut service, _, _, _) =
                MockClient::new(Some(db_reader), Some(storage_config));
            utils::update_storage_server_summary(&mut service, version, 10);
            tokio::spawn(service.start());

            // Process a request to fetch a hot states chunk with a proof
            let response = get_hot_state_values_with_proof(
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
                DataResponse::HotStateValueChunkWithProof(hot_state_value_chunk_with_proof)
            );
        }
    }
}

#[tokio::test]
async fn test_get_hot_state_values_with_proof_not_serviceable() {
    // The hot state is currently advertised over the same range as cold state, so a
    // request for a version the server has not synced to is rejected.
    let version = 101;
    let start_index = 100;
    let end_index = start_index + 99;

    // Create the storage client and server (that cannot service the request)
    let (mut mock_client, mut service, _, _, _) = MockClient::new(None, None);
    utils::update_storage_server_summary(&mut service, version - 1, 10);
    tokio::spawn(service.start());

    // Process a request to fetch a hot states chunk with a proof
    let response =
        get_hot_state_values_with_proof(&mut mock_client, version, start_index, end_index, false)
            .await
            .unwrap_err();

    // Verify the request is not serviceable
    assert_matches!(response, StorageServiceError::InvalidRequest(_));
}

#[tokio::test]
async fn test_get_number_of_hot_states() {
    // Create test data
    let version = 101;
    let number_of_hot_states: u64 = 560;

    // Create the mock db reader
    let mut db_reader = mock::create_mock_db_reader();
    db_reader
        .expect_get_hot_state_item_count()
        .times(1)
        .with(eq(version))
        .returning(move |_| Ok(number_of_hot_states as usize));

    // Create the storage client and server
    let (mut mock_client, mut service, _, _, _) = MockClient::new(Some(db_reader), None);
    utils::update_storage_server_summary(&mut service, version, 10);
    tokio::spawn(service.start());

    // Process a request to fetch the number of hot states at a version
    let data_request = DataRequest::GetNumberOfHotStatesAtVersion(version);
    let response = utils::send_storage_request(&mut mock_client, false, data_request)
        .await
        .unwrap();

    // Verify the response is correct
    assert_matches!(response, StorageServiceResponse::RawResponse(_));
    assert_eq!(
        response.get_data_response().unwrap(),
        DataResponse::NumberOfHotStatesAtVersion(number_of_hot_states)
    );
}

/// Sets an expectation on the given mock db for a call to fetch hot state values with proof.
/// Unlike cold state, the hot state path always composes an iterator with a proof (the DB
/// exposes no combined fetch-and-prove call), regardless of the chunking mode.
fn expect_get_hot_state_values_with_proof(
    mock_db: &mut MockDatabaseReader,
    version: u64,
    start_index: u64,
    chunk_size: u64,
    mut hot_state_value_chunk_with_proof: HotStateValueChunkWithProof,
) {
    // Expect a call to get a hot state value iterator
    let mut expectation_sequence = Sequence::new();
    let hot_state_value_iterator = hot_state_value_chunk_with_proof
        .clone()
        .raw_values
        .into_iter()
        .map(Ok);
    mock_db
        .expect_get_hot_state_value_chunk_iter()
        .times(1)
        .with(
            eq(version),
            eq(start_index as usize),
            eq(chunk_size as usize),
        )
        .returning(move |_, _, _| Ok(Box::new(hot_state_value_iterator.clone())))
        .in_sequence(&mut expectation_sequence);

    // Expect a call to get the hot state value chunk proof
    mock_db
        .expect_get_hot_state_value_chunk_proof()
        .times(1)
        .with(eq(version), eq(start_index as usize), always())
        .return_once(move |_, _, given_values| {
            hot_state_value_chunk_with_proof.raw_values = given_values;
            Ok(hot_state_value_chunk_with_proof)
        })
        .in_sequence(&mut expectation_sequence);
}

/// Sends a hot state values with proof request and processes the response
async fn get_hot_state_values_with_proof(
    mock_client: &mut MockClient,
    version: u64,
    start_index: u64,
    end_index: u64,
    use_compression: bool,
) -> Result<StorageServiceResponse, StorageServiceError> {
    let data_request = DataRequest::GetHotStateValuesWithProof(HotStateValuesWithProofRequest {
        version,
        start_index,
        end_index,
    });
    utils::send_storage_request(mock_client, use_compression, data_request).await
}

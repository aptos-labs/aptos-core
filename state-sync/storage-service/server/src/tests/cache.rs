// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::tests::{mock, mock::MockClient, utils};
use aptos_config::config::StorageServiceConfig;
use aptos_crypto::HashValue;
use aptos_storage_service_types::responses::{DataResponse, StorageServiceResponse};
use aptos_types::{
    proof::SparseMerkleRangeProof,
    state_store::{
        state_key::StateKey,
        state_value::{StateValue, StateValueChunkWithProof},
    },
};
use mockall::predicate::{always, eq};

#[tokio::test]
async fn test_cachable_requests_compression() {
    // Create test data
    let start_version = 0;
    let end_version = 454;
    let proof_version = end_version;
    let include_events = false;
    let compression_options = [true, false];

    // Create the mock db reader
    let mut db_reader = mock::create_mock_db_reader();
    let mut transaction_lists_with_proof = vec![];
    for _ in compression_options {
        // Create a test transaction list
        let mut transaction_list_with_proof = utils::create_transaction_list_with_proof(
            start_version,
            end_version,
            proof_version,
            true, // Always create events, even if not included in the response
        );

        // Expect the data to be fetched from storage exactly once
        utils::expect_get_transactions(
            &mut db_reader,
            start_version,
            end_version - start_version + 1,
            proof_version,
            include_events,
            transaction_list_with_proof.clone(),
        );

        // Remove the events and save the transaction lists as expected responses
        transaction_list_with_proof.events = None;
        transaction_lists_with_proof.push(transaction_list_with_proof);
    }

    // Create the storage client and server
    let (mut mock_client, mut service, _, _, _) = MockClient::new(Some(db_reader), None);
    utils::update_storage_server_summary(&mut service, end_version, 10);
    tokio::spawn(service.start());

    // Repeatedly fetch the data and verify the responses
    for (i, use_compression) in compression_options.iter().enumerate() {
        for _ in 0..10 {
            let response = utils::get_transactions_with_proof(
                &mut mock_client,
                start_version,
                end_version,
                proof_version,
                include_events,
                *use_compression,
            )
            .await
            .unwrap();

            // Verify the response is correct
            assert_eq!(response.is_compressed(), *use_compression);
            match response.get_data_response().unwrap() {
                DataResponse::TransactionsWithProof(response) => {
                    assert_eq!(response, transaction_lists_with_proof[i]);
                },
                _ => panic!("Expected transactions with proof but got: {:?}", response),
            };
        }
    }
}

#[tokio::test]
async fn test_cachable_requests_data_versions() {
    // Create test data
    let start_versions = [0, 76, 101, 230, 300, 454];
    let end_version = 454;
    let proof_version = end_version;
    let include_events = false;

    // Create the mock db reader
    let mut db_reader = mock::create_mock_db_reader();
    let mut transaction_lists_with_proof = vec![];
    for start_version in start_versions {
        // Create a test transaction list
        let mut transaction_list_with_proof = utils::create_transaction_list_with_proof(
            start_version,
            end_version,
            proof_version,
            true, // Always create events, even if not included in the response
        );

        // Expect the data to be fetched from storage once
        utils::expect_get_transactions(
            &mut db_reader,
            start_version,
            end_version - start_version + 1,
            proof_version,
            include_events,
            transaction_list_with_proof.clone(),
        );

        // Remove the events and save the transaction lists as expected responses
        transaction_list_with_proof.events = None;
        transaction_lists_with_proof.push(transaction_list_with_proof);
    }

    // Create the storage client and server
    let (mut mock_client, mut service, _, _, _) = MockClient::new(Some(db_reader), None);
    utils::update_storage_server_summary(&mut service, end_version, 10);
    tokio::spawn(service.start());

    // Repeatedly fetch the data and verify the responses
    for (i, start_version) in start_versions.iter().enumerate() {
        for _ in 0..10 {
            let response = utils::get_transactions_with_proof(
                &mut mock_client,
                *start_version,
                end_version,
                proof_version,
                include_events,
                true,
            )
            .await
            .unwrap();

            // Verify the response is correct
            match response {
                StorageServiceResponse::CompressedResponse(_, _) => {
                    match response.get_data_response().unwrap() {
                        DataResponse::TransactionsWithProof(transactions_with_proof) => {
                            assert_eq!(transactions_with_proof, transaction_lists_with_proof[i])
                        },
                        _ => panic!(
                            "Expected compressed transactions with proof but got: {:?}",
                            response
                        ),
                    }
                },
                _ => panic!("Expected compressed response but got: {:?}", response),
            };
        }
    }
}

#[tokio::test]
async fn test_cachable_requests_eviction() {
    // Create test data
    let max_lru_cache_size = StorageServiceConfig::default().max_lru_cache_size;
    let version = 101;
    let start_index = 100;
    let end_index = 199;
    let state_key_and_value = (StateKey::raw(b""), StateValue::new_legacy(vec![].into()));
    let state_value_chunk_with_proof = StateValueChunkWithProof {
        first_index: start_index,
        last_index: end_index,
        first_key: HashValue::random(),
        last_key: HashValue::random(),
        raw_values: vec![state_key_and_value.clone()],
        proof: SparseMerkleRangeProof::new(vec![]),
        root_hash: HashValue::random(),
    };

    // Create the mock db reader
    let mut db_reader = mock::create_mock_db_reader();
    db_reader
        .expect_get_state_leaf_count()
        .times(max_lru_cache_size as usize)
        .with(always())
        .returning(move |_| Ok(165));
    for _ in 0..2 {
        // Set expectations for the chunk iterator
        let state_values = vec![Ok(state_key_and_value.clone())];
        let state_value_iterator = Box::new(state_values.into_iter())
            as Box<
                dyn Iterator<Item = aptos_storage_interface::Result<(StateKey, StateValue)>>
                    + Send
                    + Sync,
            >;
        db_reader
            .expect_get_state_value_chunk_iter()
            .times(1)
            .with(
                eq(version),
                eq(start_index as usize),
                eq((end_index - start_index + 1) as usize),
            )
            .return_once(move |_, _, _| Ok(state_value_iterator));

        // Set expectations for the chunk proof
        let state_values = vec![state_key_and_value.clone()];
        let state_value_chunk_with_proof_clone = state_value_chunk_with_proof.clone();
        db_reader
            .expect_get_state_value_chunk_proof()
            .times(1)
            .with(eq(version), eq(start_index as usize), eq(state_values))
            .return_once(move |_, _, _| Ok(state_value_chunk_with_proof_clone));
    }

    // Create the storage client and server
    let (mut mock_client, mut service, _, _, _) = MockClient::new(Some(db_reader), None);
    utils::update_storage_server_summary(&mut service, version + 10, 10);
    tokio::spawn(service.start());

    // Process a request to fetch a state chunk. This should cache and serve the response.
    for _ in 0..2 {
        let _ = utils::get_state_values_with_proof(
            &mut mock_client,
            version,
            start_index,
            end_index,
            true,
        )
        .await;
    }

    // Process enough requests to evict the previously cached response
    for version in 0..max_lru_cache_size {
        let _ = utils::get_number_of_states(&mut mock_client, version, true).await;
    }

    // Process a request to fetch the state chunk again. This requires refetching the data.
    let _ =
        utils::get_state_values_with_proof(&mut mock_client, version, start_index, end_index, true)
            .await;
}

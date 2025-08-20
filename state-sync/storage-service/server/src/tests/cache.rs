// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::tests::{mock, mock::MockClient, utils};
use aptos_config::config::StorageServiceConfig;
use aptos_crypto::hash::HashValue;
use aptos_types::{
    proof::definition::SparseMerkleRangeProof, state_store::state_value::StateValueChunkWithProof,
};
use mockall::{
    predicate::{always, eq},
    Sequence,
};

#[tokio::test]
async fn test_cachable_requests_compression() {
    // Test both v1 and v2 data requests
    for use_request_v2 in [false, true] {
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
            // Create and save test transaction lists
            let transaction_list_with_proof = utils::create_transaction_list_with_proof(
                start_version,
                end_version,
                proof_version,
                include_events,
                use_request_v2,
            );
            transaction_lists_with_proof.push(transaction_list_with_proof.clone());

            // Expect the data to be fetched from storage exactly once
            utils::expect_get_transactions(
                &mut db_reader,
                start_version,
                end_version - start_version + 1,
                proof_version,
                include_events,
                transaction_list_with_proof.clone(),
            );
        }

        // Create a storage service config
        let storage_config = utils::create_storage_config(use_request_v2);

        // Create the storage client and server
        let (mut mock_client, mut service, _, _, _) =
            MockClient::new(Some(db_reader), Some(storage_config));
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
                    use_request_v2,
                    storage_config.max_network_chunk_bytes_v2,
                )
                .await
                .unwrap();

                // Verify the response is correct
                assert_eq!(response.is_compressed(), *use_compression);
                utils::verify_transaction_with_proof_response(
                    use_request_v2,
                    transaction_lists_with_proof[i].clone(),
                    response,
                );
            }
        }
    }
}

#[tokio::test]
async fn test_cachable_requests_data_versions() {
    // Test both v1 and v2 data requests
    for use_request_v2 in [false, true] {
        // Create test data
        let start_versions = [0, 76, 101, 230, 300, 454];
        let end_version = 454;
        let proof_version = end_version;
        let include_events = false;

        // Create the mock db reader
        let mut db_reader = mock::create_mock_db_reader();
        let mut transaction_lists_with_proof = vec![];
        for start_version in start_versions {
            // Create and save test transaction lists
            let transaction_list_with_proof = utils::create_transaction_list_with_proof(
                start_version,
                end_version,
                proof_version,
                include_events,
                use_request_v2,
            );
            transaction_lists_with_proof.push(transaction_list_with_proof.clone());

            // Expect the data to be fetched from storage once
            utils::expect_get_transactions(
                &mut db_reader,
                start_version,
                end_version - start_version + 1,
                proof_version,
                include_events,
                transaction_list_with_proof.clone(),
            );
        }

        // Create a storage service config
        let storage_config = utils::create_storage_config(use_request_v2);

        // Create the storage client and server
        let (mut mock_client, mut service, _, _, _) =
            MockClient::new(Some(db_reader), Some(storage_config));
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
                    use_request_v2,
                    storage_config.max_network_chunk_bytes_v2,
                )
                .await
                .unwrap();

                // Verify the response is correct
                assert!(response.is_compressed());
                utils::verify_transaction_with_proof_response(
                    use_request_v2,
                    transaction_lists_with_proof[i].clone(),
                    response,
                );
            }
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
    let mut expectation_sequence = Sequence::new();
    db_reader
        .expect_get_state_item_count()
        .times(max_lru_cache_size as usize)
        .with(always())
        .returning(move |_| Ok(165));
    for _ in 0..2 {
        let state_value_chunk_with_proof_clone = state_value_chunk_with_proof.clone();
        db_reader
            .expect_get_state_value_chunk_with_proof()
            .times(1)
            .with(
                eq(version),
                eq(start_index as usize),
                eq((end_index - start_index + 1) as usize),
            )
            .return_once(move |_, _, _| Ok(state_value_chunk_with_proof_clone))
            .in_sequence(&mut expectation_sequence);
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

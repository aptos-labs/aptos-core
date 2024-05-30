// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::tests::{mock, mock::MockClient, utils};
use aptos_config::config::StorageServiceConfig;
use aptos_storage_service_types::{
    requests::{DataRequest, TransactionOutputsWithProofRequest},
    responses::{DataResponse, StorageServiceResponse},
    StorageServiceError,
};
use aptos_types::{
    contract_event::ContractEvent,
    transaction::{Transaction, TransactionAuxiliaryData, TransactionInfo, TransactionOutput},
    write_set::WriteSet,
};
use claims::assert_matches;
use mockall::predicate::{always, eq};

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

/// A helper method to request a transaction outputs with proof chunk using
/// the specified network limit.
async fn get_outputs_with_proof_network_limit(network_limit_bytes: u64) {
    for use_compression in [true, false] {
        // Create test data
        let max_output_chunk_size =
            StorageServiceConfig::default().max_transaction_output_chunk_size;
        let min_bytes_per_transaction_and_output = 15_000; // Make these large so that infos and events are negligible
        let start_version = 455;
        let proof_version = 1000000;
        let output_list_with_proof = utils::create_output_list_using_sizes(
            start_version,
            max_output_chunk_size,
            min_bytes_per_transaction_and_output,
        );

        // Fetch the transactions, infos, write sets, events and auxiliary data from the output list
        let (transactions, transaction_outputs): (Vec<Transaction>, Vec<TransactionOutput>) =
            output_list_with_proof
                .transactions_and_outputs
                .into_iter()
                .unzip();
        let transaction_infos = output_list_with_proof
            .proof
            .transaction_infos
            .into_iter()
            .map(Ok);
        let transaction_write_sets = transaction_outputs
            .clone()
            .into_iter()
            .map(|output| Ok(output.write_set().clone()));
        let transaction_events = transaction_outputs
            .clone()
            .into_iter()
            .map(|output| Ok(output.events().to_vec()));
        let transaction_auxiliary_data = transaction_outputs
            .clone()
            .into_iter()
            .map(|output| Ok(output.auxiliary_data().clone()));

        // Create iterators for the transactions, infos, write sets, events and auxiliary data
        let transaction_iterator = Box::new(transactions.into_iter().map(Ok))
            as Box<dyn Iterator<Item = aptos_storage_interface::Result<Transaction>> + Send>;
        let transaction_info_iterator = Box::new(transaction_infos)
            as Box<dyn Iterator<Item = aptos_storage_interface::Result<TransactionInfo>> + Send>;
        let transaction_write_set_iterator = Box::new(transaction_write_sets)
            as Box<dyn Iterator<Item = aptos_storage_interface::Result<WriteSet>> + Send>;
        let transaction_event_iterator = Box::new(transaction_events)
            as Box<dyn Iterator<Item = aptos_storage_interface::Result<Vec<ContractEvent>>> + Send>;
        let transaction_auxiliary_data_iterator = Box::new(transaction_auxiliary_data)
            as Box<
                dyn Iterator<Item = aptos_storage_interface::Result<TransactionAuxiliaryData>>
                    + Send,
            >;

        // Create the mock db reader and expect calls to get the iterators
        let mut db_reader = mock::create_mock_db_reader();
        db_reader
            .expect_get_transaction_iterator()
            .times(1)
            .with(eq(start_version), eq(max_output_chunk_size))
            .return_once(move |_, _| Ok(transaction_iterator));
        db_reader
            .expect_get_transaction_info_iterator()
            .times(1)
            .with(eq(start_version), eq(max_output_chunk_size))
            .return_once(move |_, _| Ok(transaction_info_iterator));
        db_reader
            .expect_get_write_set_iterator()
            .times(1)
            .with(eq(start_version), eq(max_output_chunk_size))
            .return_once(move |_, _| Ok(transaction_write_set_iterator));
        db_reader
            .expect_get_events_iterator()
            .times(1)
            .with(eq(start_version), eq(max_output_chunk_size))
            .return_once(move |_, _| Ok(transaction_event_iterator));
        db_reader
            .expect_get_auxiliary_data_iterator()
            .times(1)
            .with(eq(start_version), eq(max_output_chunk_size))
            .return_once(move |_, _| Ok(transaction_auxiliary_data_iterator));

        // Expect calls to get the accumulator range proof
        let accumulator_range_proof = output_list_with_proof
            .proof
            .ledger_info_to_transaction_infos_proof;
        db_reader
            .expect_get_transaction_accumulator_range_proof()
            .times(1)
            .with(eq(start_version), always(), eq(proof_version))
            .return_once(move |_, _, _| Ok(accumulator_range_proof));

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
                let num_response_bytes =
                    utils::get_num_serialized_bytes(&response.get_data_response());
                let num_transactions_and_outputs =
                    outputs_with_proof.transactions_and_outputs.len();
                if num_response_bytes > network_limit_bytes {
                    // Verify the transactions and outputs are larger than the network limit
                    let num_serialized_bytes = utils::get_num_serialized_bytes(
                        &outputs_with_proof.transactions_and_outputs,
                    );
                    assert!(num_serialized_bytes > network_limit_bytes);

                    // Verify the response is only 1 data item over the network limit
                    let transactions_and_outputs = &outputs_with_proof.transactions_and_outputs
                        [0..num_transactions_and_outputs - 1];
                    let num_serialized_bytes =
                        utils::get_num_serialized_bytes(&transactions_and_outputs);
                    assert!(num_serialized_bytes <= network_limit_bytes);
                } else {
                    // Verify data fits correctly into the limit
                    let max_transactions_and_outputs =
                        network_limit_bytes / min_bytes_per_transaction_and_output;
                    assert!(num_transactions_and_outputs as u64 <= max_transactions_and_outputs);
                }
            },
            _ => panic!("Expected outputs with proof but got: {:?}", response),
        };
    }
}

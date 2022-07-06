// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{network::StorageServiceNetworkEvents, StorageReader, StorageServiceServer};
use anyhow::{format_err, Result};
use aptos_config::config::StorageServiceConfig;
use aptos_crypto::{ed25519::Ed25519PrivateKey, HashValue, PrivateKey, SigningKey, Uniform};
use aptos_logger::Level;
use aptos_time_service::{MockTimeService, TimeService};
use aptos_types::{
    account_address::AccountAddress,
    block_info::BlockInfo,
    chain_id::ChainId,
    contract_event::EventWithVersion,
    epoch_change::EpochChangeProof,
    event::EventKey,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    proof::{
        AccumulatorConsistencyProof, SparseMerkleProof, SparseMerkleRangeProof,
        TransactionAccumulatorSummary,
    },
    state_proof::StateProof,
    state_store::{
        state_key::StateKey,
        state_value::{StateValue, StateValueChunkWithProof},
    },
    transaction::{
        AccountTransactionsWithProof, ExecutionStatus, RawTransaction, Script, SignedTransaction,
        Transaction, TransactionInfo, TransactionListWithProof, TransactionOutput,
        TransactionOutputListWithProof, TransactionPayload, TransactionStatus,
        TransactionWithProof, Version,
    },
    write_set::WriteSet,
    PeerId,
};
use channel::aptos_channel;
use claim::{assert_matches, assert_none};
use futures::channel::{oneshot, oneshot::Receiver};
use mockall::{
    mock,
    predicate::{always, eq},
    Sequence,
};
use network::{
    peer_manager::PeerManagerNotification,
    protocols::{
        network::NewNetworkEvents, rpc::InboundRpcRequest, wire::handshake::v1::ProtocolId,
    },
};
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use storage_interface::{DbReader, ExecutedTrees, Order, StartupInfo};
use storage_service_types::{
    CompleteDataRange, DataSummary, Epoch, EpochEndingLedgerInfoRequest,
    NewTransactionOutputsWithProofRequest, NewTransactionsWithProofRequest, ProtocolMetadata,
    ServerProtocolVersion, StateValuesWithProofRequest, StorageServerSummary, StorageServiceError,
    StorageServiceMessage, StorageServiceRequest, StorageServiceResponse,
    TransactionOutputsWithProofRequest, TransactionsWithProofRequest,
};
use tokio::time::timeout;

/// Various test constants for storage
const MAX_RESPONSE_TIMEOUT_SECS: u64 = 30;
const PROTOCOL_VERSION: u64 = 1;

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
    let mut db_reader = create_mock_db_reader();
    let mut expectation_sequence = Sequence::new();
    db_reader
        .expect_get_state_leaf_count()
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
    let (mut mock_client, service, _) = MockClient::new(Some(db_reader));
    tokio::spawn(service.start());

    // Process a request to fetch a state chunk. This should cache and serve the response.
    for _ in 0..2 {
        let request = StorageServiceRequest::GetStateValuesWithProof(StateValuesWithProofRequest {
            version,
            start_index,
            end_index,
        });
        let _ = mock_client.process_request(request).await.unwrap();
    }

    // Process enough requests to evict the previously cached response
    for version in 0..max_lru_cache_size {
        let request = StorageServiceRequest::GetNumberOfStatesAtVersion(version);
        let _ = mock_client.process_request(request).await.unwrap();
    }

    // Process a request to fetch the state chunk again. This requires refetching the data.
    let request = StorageServiceRequest::GetStateValuesWithProof(StateValuesWithProofRequest {
        version,
        start_index,
        end_index,
    });
    let _ = mock_client.process_request(request).await.unwrap();
}

#[tokio::test]
async fn test_cachable_requests_data_versions() {
    // Create test data
    let start_versions = [0, 76, 101, 230, 300, 454];
    let end_version = 454;
    let proof_version = end_version;
    let include_events = false;

    // Create the mock db reader
    let mut db_reader = create_mock_db_reader();
    let mut expectation_sequence = Sequence::new();
    let mut transaction_lists_with_proof = vec![];
    for start_version in start_versions {
        // Create and save test transaction lists
        let transaction_list_with_proof = create_transaction_list_with_proof(
            start_version,
            end_version,
            proof_version,
            include_events,
        );
        transaction_lists_with_proof.push(transaction_list_with_proof.clone());

        // Expect the data to be fetched from storage once
        db_reader
            .expect_get_transactions()
            .times(1)
            .with(
                eq(start_version),
                eq(end_version - start_version + 1),
                eq(proof_version),
                eq(include_events),
            )
            .return_once(move |_, _, _, _| Ok(transaction_list_with_proof))
            .in_sequence(&mut expectation_sequence);
    }

    // Create the storage client and server
    let (mut mock_client, service, _) = MockClient::new(Some(db_reader));
    tokio::spawn(service.start());

    // Repeatedly fetch the data and verify the responses
    for (i, start_version) in start_versions.iter().enumerate() {
        for _ in 0..10 {
            let request =
                StorageServiceRequest::GetTransactionsWithProof(TransactionsWithProofRequest {
                    proof_version,
                    start_version: *start_version,
                    end_version,
                    include_events,
                });

            // Process the request
            let response = mock_client.process_request(request).await.unwrap();

            // Verify the response is correct
            match response {
                StorageServiceResponse::TransactionsWithProof(transactions_with_proof) => {
                    assert_eq!(transactions_with_proof, transaction_lists_with_proof[i])
                }
                _ => panic!("Expected transactions with proof but got: {:?}", response),
            };
        }
    }
}

#[tokio::test]
async fn test_get_server_protocol_version() {
    // Create the storage client and server
    let (mut mock_client, service, _) = MockClient::new(None);
    tokio::spawn(service.start());

    // Process a request to fetch the protocol version
    let request = StorageServiceRequest::GetServerProtocolVersion;
    let response = mock_client.process_request(request).await.unwrap();

    // Verify the response is correct
    let expected_response = StorageServiceResponse::ServerProtocolVersion(ServerProtocolVersion {
        protocol_version: PROTOCOL_VERSION,
    });
    assert_eq!(response, expected_response);
}

#[tokio::test]
async fn test_get_states_with_proof() {
    // Test small and large chunk requests
    for chunk_size in [1, 100, StorageServiceConfig::default().max_state_chunk_size] {
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
        let mut db_reader = create_mock_db_reader();
        let state_value_chunk_with_proof_clone = state_value_chunk_with_proof.clone();
        db_reader
            .expect_get_state_value_chunk_with_proof()
            .times(1)
            .with(
                eq(version),
                eq(start_index as usize),
                eq((end_index - start_index + 1) as usize),
            )
            .return_once(move |_, _, _| Ok(state_value_chunk_with_proof_clone));

        // Create the storage client and server
        let (mut mock_client, service, _) = MockClient::new(Some(db_reader));
        tokio::spawn(service.start());

        // Process a request to fetch a states chunk with a proof
        let request = StorageServiceRequest::GetStateValuesWithProof(StateValuesWithProofRequest {
            version,
            start_index,
            end_index,
        });
        let response = mock_client.process_request(request).await.unwrap();

        // Verify the response is correct
        assert_eq!(
            response,
            StorageServiceResponse::StateValueChunkWithProof(state_value_chunk_with_proof)
        );
    }
}

#[tokio::test]
async fn test_get_states_with_proof_invalid() {
    // Create the storage client and server
    let (mut mock_client, service, _) = MockClient::new(None);
    tokio::spawn(service.start());

    // Test invalid ranges and chunks that are too large
    let max_state_chunk_size = StorageServiceConfig::default().max_state_chunk_size;
    let start_index = 100;
    for end_index in [99, start_index + max_state_chunk_size] {
        let request = StorageServiceRequest::GetStateValuesWithProof(StateValuesWithProofRequest {
            version: 0,
            start_index,
            end_index,
        });

        // Process and verify the response
        let response = mock_client.process_request(request).await.unwrap_err();
        assert_matches!(response, StorageServiceError::InvalidRequest(_));
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_get_new_transactions() {
    // Test small and large chunk sizes
    for chunk_size in [
        1,
        100,
        StorageServiceConfig::default().max_transaction_chunk_size,
    ] {
        // Test event inclusion
        for include_events in [true, false] {
            // Create test data
            let highest_version = 45576;
            let highest_epoch = 43;
            let lowest_version = 4566;
            let peer_version = highest_version - chunk_size;
            let highest_ledger_info =
                create_test_ledger_info_with_sigs(highest_epoch, highest_version);
            let transaction_list_with_proof = create_transaction_list_with_proof(
                peer_version + 1,
                highest_version,
                highest_version,
                include_events,
            );

            // Create the mock db reader
            let mut db_reader =
                create_mock_db_for_subscription(highest_ledger_info.clone(), lowest_version);
            expect_get_transactions(
                &mut db_reader,
                peer_version + 1,
                highest_version - peer_version,
                highest_version,
                include_events,
                transaction_list_with_proof.clone(),
            );

            // Create the storage client and server
            let (mut mock_client, service, mock_time) = MockClient::new(Some(db_reader));
            tokio::spawn(service.start());

            // Send a request to subscribe to new transactions
            let mut response_receiver = send_new_transaction_request(
                &mut mock_client,
                peer_version,
                highest_epoch,
                include_events,
            )
            .await;

            // Verify no subscription response has been received yet
            assert_none!(response_receiver.try_recv().unwrap());

            // Elapse enough time to force the subscription thread to work
            wait_for_subscription_service_to_refresh(&mut mock_client, &mock_time).await;

            // Verify a response is received and that it contains the correct data
            verify_new_transactions_with_proof(
                &mut mock_client,
                response_receiver,
                transaction_list_with_proof,
                highest_ledger_info,
            )
            .await;
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_get_new_transactions_epoch_change() {
    // Test event inclusion
    for include_events in [true, false] {
        // Create test data
        let highest_version = 45576;
        let highest_epoch = 1032;
        let lowest_version = 4566;
        let peer_version = highest_version - 100;
        let peer_epoch = highest_epoch - 20;
        let epoch_change_version = peer_version + 45;
        let epoch_change_proof = EpochChangeProof {
            ledger_info_with_sigs: vec![create_test_ledger_info_with_sigs(
                peer_epoch,
                epoch_change_version,
            )],
            more: false,
        };
        let transaction_list_with_proof = create_transaction_list_with_proof(
            peer_version + 1,
            epoch_change_version,
            epoch_change_version,
            include_events,
        );

        // Create the mock db reader
        let mut db_reader = create_mock_db_for_subscription(
            create_test_ledger_info_with_sigs(highest_epoch, highest_version),
            lowest_version,
        );
        expect_get_transactions(
            &mut db_reader,
            peer_version + 1,
            epoch_change_version - peer_version,
            epoch_change_version,
            include_events,
            transaction_list_with_proof.clone(),
        );
        expect_get_epoch_ending_ledger_infos(
            &mut db_reader,
            peer_epoch,
            epoch_change_proof.clone(),
        );

        // Create the storage client and server
        let (mut mock_client, service, mock_time) = MockClient::new(Some(db_reader));
        tokio::spawn(service.start());

        // Send a request to subscribe to new transactions
        let response_receiver = send_new_transaction_request(
            &mut mock_client,
            peer_version,
            peer_epoch,
            include_events,
        )
        .await;

        // Elapse enough time to force the subscription thread to work
        wait_for_subscription_service_to_refresh(&mut mock_client, &mock_time).await;

        // Verify a response is received and that it contains the correct data
        verify_new_transactions_with_proof(
            &mut mock_client,
            response_receiver,
            transaction_list_with_proof,
            epoch_change_proof.ledger_info_with_sigs[0].clone(),
        )
        .await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_get_new_transactions_max_chunk() {
    // Test event inclusion
    for include_events in [true, false] {
        // Create test data
        let highest_version = 1034556;
        let highest_epoch = 343;
        let lowest_version = 3453;
        let max_chunk_size = StorageServiceConfig::default().max_transaction_chunk_size;
        let requested_chunk_size = max_chunk_size + 1;
        let peer_version = highest_version - requested_chunk_size;
        let highest_ledger_info = create_test_ledger_info_with_sigs(highest_epoch, highest_version);
        let transaction_list_with_proof = create_transaction_list_with_proof(
            peer_version + 1,
            peer_version + requested_chunk_size,
            peer_version + requested_chunk_size,
            include_events,
        );

        // Create the mock db reader
        let mut db_reader =
            create_mock_db_for_subscription(highest_ledger_info.clone(), lowest_version);
        expect_get_transactions(
            &mut db_reader,
            peer_version + 1,
            max_chunk_size,
            highest_version,
            include_events,
            transaction_list_with_proof.clone(),
        );

        // Create the storage client and server
        let (mut mock_client, service, mock_time) = MockClient::new(Some(db_reader));
        tokio::spawn(service.start());

        // Send a request to subscribe to new transactions
        let response_receiver = send_new_transaction_request(
            &mut mock_client,
            peer_version,
            highest_epoch,
            include_events,
        )
        .await;

        // Elapse enough time to force the subscription thread to work
        wait_for_subscription_service_to_refresh(&mut mock_client, &mock_time).await;

        // Verify a response is received and that it contains the correct data
        verify_new_transactions_with_proof(
            &mut mock_client,
            response_receiver,
            transaction_list_with_proof,
            highest_ledger_info,
        )
        .await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_get_new_transaction_outputs() {
    // Test small and large chunk sizes
    for chunk_size in [
        1,
        100,
        StorageServiceConfig::default().max_transaction_output_chunk_size,
    ] {
        // Create test data
        let highest_version = 5060;
        let highest_epoch = 30;
        let lowest_version = 101;
        let peer_version = highest_version - chunk_size;
        let highest_ledger_info = create_test_ledger_info_with_sigs(highest_epoch, highest_version);
        let output_list_with_proof =
            create_output_list_with_proof(peer_version + 1, highest_version, highest_version);

        // Create the mock db reader
        let mut db_reader =
            create_mock_db_for_subscription(highest_ledger_info.clone(), lowest_version);
        expect_get_transaction_outputs(
            &mut db_reader,
            peer_version + 1,
            highest_version - peer_version,
            highest_version,
            output_list_with_proof.clone(),
        );

        // Create the storage client and server
        let (mut mock_client, service, mock_time) = MockClient::new(Some(db_reader));
        tokio::spawn(service.start());

        // Send a request to subscribe to new transaction outputs
        let mut response_receiver =
            send_new_transaction_output_request(&mut mock_client, peer_version, highest_epoch)
                .await;

        // Verify no subscription response has been received yet
        assert_none!(response_receiver.try_recv().unwrap());

        // Elapse enough time to force the subscription thread to work
        wait_for_subscription_service_to_refresh(&mut mock_client, &mock_time).await;

        // Verify a response is received and that it contains the correct data
        verify_new_transaction_outputs_with_proof(
            &mut mock_client,
            response_receiver,
            output_list_with_proof,
            highest_ledger_info,
        )
        .await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_get_new_transaction_outputs_epoch_change() {
    // Create test data
    let highest_version = 10000;
    let highest_epoch = 10000;
    let lowest_version = 0;
    let peer_version = highest_version - 1000;
    let peer_epoch = highest_epoch - 1000;
    let epoch_change_version = peer_version + 1;
    let epoch_change_proof = EpochChangeProof {
        ledger_info_with_sigs: vec![create_test_ledger_info_with_sigs(
            peer_epoch,
            epoch_change_version,
        )],
        more: false,
    };
    let output_list_with_proof =
        create_output_list_with_proof(peer_version + 1, epoch_change_version, epoch_change_version);

    // Create the mock db reader
    let mut db_reader = create_mock_db_for_subscription(
        create_test_ledger_info_with_sigs(highest_epoch, highest_version),
        lowest_version,
    );
    expect_get_transaction_outputs(
        &mut db_reader,
        peer_version + 1,
        epoch_change_version - peer_version,
        epoch_change_version,
        output_list_with_proof.clone(),
    );
    expect_get_epoch_ending_ledger_infos(&mut db_reader, peer_epoch, epoch_change_proof.clone());

    // Create the storage client and server
    let (mut mock_client, service, mock_time) = MockClient::new(Some(db_reader));
    tokio::spawn(service.start());

    // Send a request to subscribe to new transaction outputs
    let response_receiver =
        send_new_transaction_output_request(&mut mock_client, peer_version, peer_epoch).await;

    // Elapse enough time to force the subscription thread to work
    wait_for_subscription_service_to_refresh(&mut mock_client, &mock_time).await;

    // Verify a response is received and that it contains the correct data
    verify_new_transaction_outputs_with_proof(
        &mut mock_client,
        response_receiver,
        output_list_with_proof,
        epoch_change_proof.ledger_info_with_sigs[0].clone(),
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_get_new_transaction_outputs_max_chunk() {
    // Create test data
    let highest_version = 65660;
    let highest_epoch = 30;
    let lowest_version = 101;
    let max_chunk_size = StorageServiceConfig::default().max_transaction_output_chunk_size;
    let requested_chunk_size = max_chunk_size + 1;
    let peer_version = highest_version - requested_chunk_size;
    let highest_ledger_info = create_test_ledger_info_with_sigs(highest_epoch, highest_version);
    let output_list_with_proof = create_output_list_with_proof(
        peer_version + 1,
        peer_version + requested_chunk_size,
        highest_version,
    );

    // Create the mock db reader
    let mut db_reader =
        create_mock_db_for_subscription(highest_ledger_info.clone(), lowest_version);
    expect_get_transaction_outputs(
        &mut db_reader,
        peer_version + 1,
        max_chunk_size,
        highest_version,
        output_list_with_proof.clone(),
    );

    // Create the storage client and server
    let (mut mock_client, service, mock_time) = MockClient::new(Some(db_reader));
    tokio::spawn(service.start());

    // Send a request to subscribe to new transaction outputs
    let response_receiver =
        send_new_transaction_output_request(&mut mock_client, peer_version, highest_epoch).await;

    // Elapse enough time to force the subscription thread to work
    wait_for_subscription_service_to_refresh(&mut mock_client, &mock_time).await;

    // Verify a response is received and that it contains the correct data
    verify_new_transaction_outputs_with_proof(
        &mut mock_client,
        response_receiver,
        output_list_with_proof,
        highest_ledger_info,
    )
    .await;
}

#[tokio::test]
async fn test_get_number_of_states_at_version() {
    // Create test data
    let version = 101;
    let number_of_states: u64 = 560;

    // Create the mock db reader
    let mut db_reader = create_mock_db_reader();
    db_reader
        .expect_get_state_leaf_count()
        .times(1)
        .with(eq(version))
        .returning(move |_| Ok(number_of_states as usize));

    // Create the storage client and server
    let (mut mock_client, service, _) = MockClient::new(Some(db_reader));
    tokio::spawn(service.start());

    // Process a request to fetch the number of states at a version
    let request = StorageServiceRequest::GetNumberOfStatesAtVersion(version);
    let response = mock_client.process_request(request).await.unwrap();

    // Verify the response is correct
    assert_eq!(
        response,
        StorageServiceResponse::NumberOfStatesAtVersion(number_of_states)
    );
}

#[tokio::test]
async fn test_get_number_of_states_at_version_invalid() {
    // Create test data
    let version = 1;

    // Create the mock db reader
    let mut db_reader = create_mock_db_reader();
    db_reader
        .expect_get_state_leaf_count()
        .times(1)
        .with(eq(version))
        .returning(move |_| Err(format_err!("Version does not exist!")));

    // Create the storage client and server
    let (mut mock_client, service, _) = MockClient::new(Some(db_reader));
    tokio::spawn(service.start());

    // Process a request to fetch the number of states at a version
    let request = StorageServiceRequest::GetNumberOfStatesAtVersion(version);
    let response = mock_client.process_request(request).await.unwrap_err();

    // Verify the response is correct
    assert_matches!(response, StorageServiceError::InternalError(_));
}

#[tokio::test]
async fn test_get_storage_server_summary() {
    // Create test data
    let highest_version = 506;
    let highest_epoch = 30;
    let lowest_version = 101;
    let state_prune_window = 50;
    let highest_ledger_info = create_test_ledger_info_with_sigs(highest_epoch, highest_version);

    // Create the mock db reader
    let mut db_reader = create_mock_db_reader();
    let highest_ledger_info_clone = highest_ledger_info.clone();
    db_reader
        .expect_get_latest_ledger_info()
        .times(1)
        .return_once(move || Ok(highest_ledger_info_clone));
    db_reader
        .expect_get_first_txn_version()
        .times(1)
        .return_once(move || Ok(Some(lowest_version)));
    db_reader
        .expect_get_first_write_set_version()
        .times(1)
        .return_once(move || Ok(Some(lowest_version)));
    db_reader
        .expect_get_state_prune_window()
        .times(1)
        .return_once(move || Ok(Some(state_prune_window)));

    // Create the storage client and server
    let (mut mock_client, service, mock_time) = MockClient::new(Some(db_reader));
    tokio::spawn(service.start());

    // Fetch the storage summary and verify we get a default summary response
    let request = StorageServiceRequest::GetStorageServerSummary;
    let response = mock_client.process_request(request).await.unwrap();
    let default_response =
        StorageServiceResponse::StorageServerSummary(StorageServerSummary::default());
    assert_eq!(response, default_response);

    // Elapse enough time to force a cache update
    advance_storage_refresh_time(&mock_time).await;

    // Process another request to fetch the storage summary
    let request = StorageServiceRequest::GetStorageServerSummary;
    let response = mock_client.process_request(request).await.unwrap();

    // Verify the response is correct (after the cache update)
    let default_storage_config = StorageServiceConfig::default();
    let expected_server_summary = StorageServerSummary {
        protocol_metadata: ProtocolMetadata {
            max_epoch_chunk_size: default_storage_config.max_epoch_chunk_size,
            max_state_chunk_size: default_storage_config.max_state_chunk_size,
            max_transaction_chunk_size: default_storage_config.max_transaction_chunk_size,
            max_transaction_output_chunk_size: default_storage_config
                .max_transaction_output_chunk_size,
        },
        data_summary: DataSummary {
            synced_ledger_info: Some(highest_ledger_info),
            epoch_ending_ledger_infos: Some(CompleteDataRange::from_genesis(highest_epoch - 1)),
            transactions: Some(CompleteDataRange::new(lowest_version, highest_version).unwrap()),
            transaction_outputs: Some(
                CompleteDataRange::new(lowest_version, highest_version).unwrap(),
            ),
            states: Some(
                CompleteDataRange::new(
                    highest_version - state_prune_window as u64 + 1,
                    highest_version,
                )
                .unwrap(),
            ),
        },
    };
    assert_eq!(
        response,
        StorageServiceResponse::StorageServerSummary(expected_server_summary)
    );
}

#[tokio::test]
async fn test_get_transactions_with_proof() {
    // Test small and large chunk requests
    for chunk_size in [
        1,
        100,
        StorageServiceConfig::default().max_transaction_chunk_size,
    ] {
        // Test event inclusion
        for include_events in [true, false] {
            // Create test data
            let start_version = 0;
            let end_version = start_version + chunk_size - 1;
            let proof_version = end_version;
            let transaction_list_with_proof = create_transaction_list_with_proof(
                start_version,
                end_version,
                proof_version,
                include_events,
            );

            // Create the mock db reader
            let mut db_reader = create_mock_db_reader();
            let transaction_list_with_proof_clone = transaction_list_with_proof.clone();
            db_reader
                .expect_get_transactions()
                .times(1)
                .with(
                    eq(start_version),
                    eq(end_version - start_version + 1),
                    eq(proof_version),
                    eq(include_events),
                )
                .return_once(move |_, _, _, _| Ok(transaction_list_with_proof_clone));

            // Create the storage client and server
            let (mut mock_client, service, _) = MockClient::new(Some(db_reader));
            tokio::spawn(service.start());

            // Create a request to fetch transactions with a proof
            let request =
                StorageServiceRequest::GetTransactionsWithProof(TransactionsWithProofRequest {
                    proof_version,
                    start_version,
                    end_version,
                    include_events,
                });

            // Process the request
            let response = mock_client.process_request(request).await.unwrap();

            // Verify the response is correct
            match response {
                StorageServiceResponse::TransactionsWithProof(transactions_with_proof) => {
                    assert_eq!(transactions_with_proof, transaction_list_with_proof)
                }
                _ => panic!("Expected transactions with proof but got: {:?}", response),
            };
        }
    }
}

#[tokio::test]
async fn test_get_transactions_with_proof_invalid() {
    // Create the storage client and server
    let (mut mock_client, service, _) = MockClient::new(None);
    tokio::spawn(service.start());

    // Test invalid ranges and chunks that are too large
    let max_transaction_chunk_size = StorageServiceConfig::default().max_transaction_chunk_size;
    let start_version = 1000;
    for end_version in [1, start_version + max_transaction_chunk_size] {
        let request =
            StorageServiceRequest::GetTransactionsWithProof(TransactionsWithProofRequest {
                proof_version: start_version,
                start_version,
                end_version,
                include_events: true,
            });

        // Process and verify the response
        let response = mock_client.process_request(request).await.unwrap_err();
        assert_matches!(response, StorageServiceError::InvalidRequest(_));
    }
}

#[tokio::test]
async fn test_get_transaction_outputs_with_proof() {
    // Test small and large chunk requests
    for chunk_size in [
        1,
        100,
        StorageServiceConfig::default().max_transaction_output_chunk_size,
    ] {
        // Create test data
        let start_version = 0;
        let end_version = start_version + chunk_size - 1;
        let proof_version = end_version;
        let output_list_with_proof =
            create_output_list_with_proof(start_version, end_version, proof_version);

        // Create the mock db reader
        let mut db_reader = create_mock_db_reader();
        let output_list_with_proof_clone = output_list_with_proof.clone();
        db_reader
            .expect_get_transaction_outputs()
            .times(1)
            .with(
                eq(start_version),
                eq(end_version - start_version + 1),
                eq(proof_version),
            )
            .return_once(move |_, _, _| Ok(output_list_with_proof_clone));

        // Create the storage client and server
        let (mut mock_client, service, _) = MockClient::new(Some(db_reader));
        tokio::spawn(service.start());

        // Create a request to fetch transactions outputs with a proof
        let request = StorageServiceRequest::GetTransactionOutputsWithProof(
            TransactionOutputsWithProofRequest {
                proof_version,
                start_version,
                end_version,
            },
        );

        // Process the request
        let response = mock_client.process_request(request).await.unwrap();

        // Verify the response is correct
        match response {
            StorageServiceResponse::TransactionOutputsWithProof(outputs_with_proof) => {
                assert_eq!(outputs_with_proof, output_list_with_proof)
            }
            _ => panic!(
                "Expected transaction outputs with proof but got: {:?}",
                response
            ),
        };
    }
}

#[tokio::test]
async fn test_get_transaction_outputs_with_proof_invalid() {
    // Create the storage client and server
    let (mut mock_client, service, _) = MockClient::new(None);
    tokio::spawn(service.start());

    // Test invalid ranges and chunks that are too large
    let max_output_chunk_size = StorageServiceConfig::default().max_transaction_output_chunk_size;
    let start_version = 1000;
    for end_version in [1, start_version + max_output_chunk_size] {
        let request = StorageServiceRequest::GetTransactionOutputsWithProof(
            TransactionOutputsWithProofRequest {
                proof_version: end_version,
                start_version,
                end_version,
            },
        );

        // Process and verify the response
        let response = mock_client.process_request(request).await.unwrap_err();
        assert_matches!(response, StorageServiceError::InvalidRequest(_));
    }
}

#[tokio::test]
async fn test_get_epoch_ending_ledger_infos() {
    // Test small and large chunk requests
    for chunk_size in [1, 100, StorageServiceConfig::default().max_epoch_chunk_size] {
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
        let mut db_reader = create_mock_db_reader();
        let epoch_change_proof_clone = epoch_change_proof.clone();
        db_reader
            .expect_get_epoch_ending_ledger_infos()
            .times(1)
            .with(eq(start_epoch), eq(expected_end_epoch + 1))
            .return_once(move |_, _| Ok(epoch_change_proof_clone));

        // Create the storage client and server
        let (mut mock_client, service, _) = MockClient::new(Some(db_reader));
        tokio::spawn(service.start());

        // Create a request to fetch epoch ending ledger infos
        let request =
            StorageServiceRequest::GetEpochEndingLedgerInfos(EpochEndingLedgerInfoRequest {
                start_epoch,
                expected_end_epoch,
            });

        // Process the request
        let response = mock_client.process_request(request).await.unwrap();

        // Verify the response is correct
        match response {
            StorageServiceResponse::EpochEndingLedgerInfos(response_epoch_change_proof) => {
                assert_eq!(response_epoch_change_proof, epoch_change_proof)
            }
            _ => panic!("Expected epoch ending ledger infos but got: {:?}", response),
        };
    }
}

#[tokio::test]
async fn test_get_epoch_ending_ledger_infos_invalid() {
    // Create the storage client and server
    let (mut mock_client, service, _) = MockClient::new(None);
    tokio::spawn(service.start());

    // Test invalid ranges and chunks that are too large
    let max_epoch_chunk_size = StorageServiceConfig::default().max_epoch_chunk_size;
    let start_epoch = 11;
    for expected_end_epoch in [10, start_epoch + max_epoch_chunk_size] {
        let request =
            StorageServiceRequest::GetEpochEndingLedgerInfos(EpochEndingLedgerInfoRequest {
                start_epoch,
                expected_end_epoch,
            });

        // Process and verify the response
        let response = mock_client.process_request(request).await.unwrap_err();
        assert_matches!(response, StorageServiceError::InvalidRequest(_));
    }
}

/// A wrapper around the inbound network interface/channel for easily sending
/// mock client requests to a [`StorageServiceServer`].
struct MockClient {
    peer_mgr_notifs_tx: aptos_channel::Sender<(PeerId, ProtocolId), PeerManagerNotification>,
}

impl MockClient {
    fn new(
        db_reader: Option<MockDatabaseReader>,
    ) -> (Self, StorageServiceServer<StorageReader>, MockTimeService) {
        initialize_logger();
        let storage_config = StorageServiceConfig::default();
        let storage = StorageReader::new(
            storage_config,
            Arc::new(db_reader.unwrap_or_else(create_mock_db_reader)),
        );

        let queue_cfg = crate::network::network_endpoint_config(storage_config)
            .inbound_queue
            .unwrap();
        let (peer_mgr_notifs_tx, peer_mgr_notifs_rx) = queue_cfg.build();
        let (_connection_notifs_tx, connection_notifs_rx) = queue_cfg.build();
        let network_requests =
            StorageServiceNetworkEvents::new(peer_mgr_notifs_rx, connection_notifs_rx);

        let executor = tokio::runtime::Handle::current();
        let mock_time_service = TimeService::mock();
        let storage_server = StorageServiceServer::new(
            StorageServiceConfig::default(),
            executor,
            storage,
            mock_time_service.clone(),
            network_requests,
        );

        let mock_client = Self { peer_mgr_notifs_tx };
        (mock_client, storage_server, mock_time_service.into_mock())
    }

    /// Send the given storage request and wait for a response
    async fn process_request(
        &mut self,
        request: StorageServiceRequest,
    ) -> Result<StorageServiceResponse, StorageServiceError> {
        let receiver = self.send_request(request).await;
        self.wait_for_response(receiver).await
    }

    /// Send the specified storage request and return the receiver on which to
    /// expect a result.
    async fn send_request(
        &mut self,
        request: StorageServiceRequest,
    ) -> Receiver<Result<bytes::Bytes, network::protocols::network::RpcError>> {
        // Create the inbound rpc request
        let peer_id = PeerId::ZERO;
        let protocol_id = ProtocolId::StorageServiceRpc;
        let data = protocol_id
            .to_bytes(&StorageServiceMessage::Request(request))
            .unwrap();
        let (res_tx, res_rx) = oneshot::channel();
        let inbound_rpc = InboundRpcRequest {
            protocol_id,
            data: data.into(),
            res_tx,
        };
        let notif = PeerManagerNotification::RecvRpc(peer_id, inbound_rpc);

        // Push the request up to the storage service
        self.peer_mgr_notifs_tx
            .push((peer_id, protocol_id), notif)
            .unwrap();

        res_rx
    }

    /// Helper method to wait for and deserialize a response on the specified receiver
    async fn wait_for_response(
        &mut self,
        receiver: Receiver<Result<bytes::Bytes, network::protocols::network::RpcError>>,
    ) -> Result<StorageServiceResponse, StorageServiceError> {
        if let Ok(response) =
            timeout(Duration::from_secs(MAX_RESPONSE_TIMEOUT_SECS), receiver).await
        {
            let response = ProtocolId::StorageServiceRpc
                .from_bytes::<StorageServiceMessage>(&response.unwrap().unwrap())
                .unwrap();
            match response {
                StorageServiceMessage::Response(response) => response,
                _ => panic!("Unexpected response message: {:?}", response),
            }
        } else {
            panic!("Timed out while waiting for a response from the storage service!")
        }
    }
}

/// Waits until the storage summary has refreshed for the first time
async fn wait_for_storage_to_refresh(mock_client: &mut MockClient, mock_time: &MockTimeService) {
    while mock_client
        .process_request(StorageServiceRequest::GetStorageServerSummary)
        .await
        .unwrap()
        == StorageServiceResponse::StorageServerSummary(StorageServerSummary::default())
    {
        advance_storage_refresh_time(mock_time).await;
    }
}

/// Advances enough time that the subscription service is able to refresh
async fn wait_for_subscription_service_to_refresh(
    mock_client: &mut MockClient,
    mock_time: &MockTimeService,
) {
    // Elapse enough time to force storage to be updated
    wait_for_storage_to_refresh(mock_client, mock_time).await;

    // Elapse enough time to force the subscription thread to work
    advance_storage_refresh_time(mock_time).await;
}

/// Advances the given timer by the amount of time it takes to refresh storage
async fn advance_storage_refresh_time(mock_time: &MockTimeService) {
    let default_storage_config = StorageServiceConfig::default();
    let cache_update_freq_ms = default_storage_config.storage_summary_refresh_interval_ms;
    mock_time.advance_ms_async(cache_update_freq_ms).await;
}

/// Creates and sends a request for new transaction outputs
async fn send_new_transaction_output_request(
    mock_client: &mut MockClient,
    known_version: u64,
    known_epoch: u64,
) -> Receiver<Result<bytes::Bytes, network::protocols::network::RpcError>> {
    let request = StorageServiceRequest::GetNewTransactionOutputsWithProof(
        NewTransactionOutputsWithProofRequest {
            known_version,
            known_epoch,
        },
    );
    mock_client.send_request(request).await
}

/// Creates and sends a request for new transactions
async fn send_new_transaction_request(
    mock_client: &mut MockClient,
    known_version: u64,
    known_epoch: u64,
    include_events: bool,
) -> Receiver<Result<bytes::Bytes, network::protocols::network::RpcError>> {
    let request =
        StorageServiceRequest::GetNewTransactionsWithProof(NewTransactionsWithProofRequest {
            known_version,
            known_epoch,
            include_events,
        });
    mock_client.send_request(request).await
}

/// Creates a mock db with the basic expectations required to handle subscription requests
fn create_mock_db_for_subscription(
    highest_ledger_info_clone: LedgerInfoWithSignatures,
    lowest_version: Version,
) -> MockDatabaseReader {
    let mut db_reader = create_mock_db_reader();
    db_reader
        .expect_get_latest_ledger_info()
        .return_once(move || Ok(highest_ledger_info_clone));
    db_reader
        .expect_get_first_txn_version()
        .return_once(move || Ok(Some(lowest_version)));
    db_reader
        .expect_get_first_write_set_version()
        .return_once(move || Ok(Some(lowest_version)));
    db_reader
        .expect_get_state_prune_window()
        .return_once(move || Ok(Some(100)));
    db_reader
}

/// Sets an expectation on the given mock db for a call to fetch transactions
fn expect_get_transactions(
    mock_db: &mut MockDatabaseReader,
    start_version: u64,
    num_items: u64,
    proof_version: u64,
    include_events: bool,
    transaction_list: TransactionListWithProof,
) {
    mock_db
        .expect_get_transactions()
        .times(1)
        .with(
            eq(start_version),
            eq(num_items),
            eq(proof_version),
            eq(include_events),
        )
        .return_once(move |_, _, _, _| Ok(transaction_list));
}

/// Sets an expectation on the given mock db for a call to fetch transaction outputs
fn expect_get_transaction_outputs(
    mock_db: &mut MockDatabaseReader,
    start_version: u64,
    num_items: u64,
    proof_version: u64,
    output_list: TransactionOutputListWithProof,
) {
    mock_db
        .expect_get_transaction_outputs()
        .times(1)
        .with(eq(start_version), eq(num_items), eq(proof_version))
        .return_once(move |_, _, _| Ok(output_list));
}

/// Sets an expectation on the given mock db for a call to fetch an epoch change proof
fn expect_get_epoch_ending_ledger_infos(
    mock_db: &mut MockDatabaseReader,
    epoch_to_end: u64,
    epoch_change_proof: EpochChangeProof,
) {
    mock_db
        .expect_get_epoch_ending_ledger_infos()
        .times(1)
        .with(eq(epoch_to_end), eq(epoch_to_end + 1))
        .return_once(move |_, _| Ok(epoch_change_proof));
}

/// Creates a test epoch change proof
fn create_epoch_ending_ledger_infos(
    start_epoch: Epoch,
    end_epoch: Epoch,
) -> Vec<LedgerInfoWithSignatures> {
    let mut ledger_info_with_sigs = vec![];
    for epoch in start_epoch..end_epoch {
        ledger_info_with_sigs.push(create_test_ledger_info_with_sigs(epoch, 0));
    }
    ledger_info_with_sigs
}

/// Creates a test transaction output list with proof
fn create_output_list_with_proof(
    start_version: u64,
    end_version: u64,
    proof_version: u64,
) -> TransactionOutputListWithProof {
    let transaction_list_with_proof =
        create_transaction_list_with_proof(start_version, end_version, proof_version, false);
    let transactions_and_outputs = transaction_list_with_proof
        .transactions
        .iter()
        .map(|txn| (txn.clone(), create_test_transaction_output()))
        .collect();

    TransactionOutputListWithProof::new(
        transactions_and_outputs,
        Some(start_version),
        transaction_list_with_proof.proof,
    )
}

/// Creates a test ledger info with signatures
fn create_test_ledger_info_with_sigs(epoch: u64, version: u64) -> LedgerInfoWithSignatures {
    // Create a mock ledger info with signatures
    let ledger_info = LedgerInfo::new(
        BlockInfo::new(
            epoch,
            0,
            HashValue::zero(),
            HashValue::zero(),
            version,
            0,
            None,
        ),
        HashValue::zero(),
    );
    LedgerInfoWithSignatures::new(ledger_info, BTreeMap::new())
}

/// Creates a test transaction output
fn create_test_transaction_output() -> TransactionOutput {
    TransactionOutput::new(
        WriteSet::default(),
        vec![],
        0,
        TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(None)),
    )
}

/// Creates a test user transaction
fn create_test_transaction(sequence_number: u64) -> Transaction {
    let private_key = Ed25519PrivateKey::generate_for_testing();
    let public_key = private_key.public_key();

    let transaction_payload = TransactionPayload::Script(Script::new(vec![], vec![], vec![]));
    let raw_transaction = RawTransaction::new(
        AccountAddress::random(),
        sequence_number,
        transaction_payload,
        0,
        0,
        0,
        ChainId::new(10),
    );
    let signed_transaction = SignedTransaction::new(
        raw_transaction.clone(),
        public_key,
        private_key.sign(&raw_transaction),
    );

    Transaction::UserTransaction(signed_transaction)
}

/// Creates a test transaction output list with proof
fn create_transaction_list_with_proof(
    start_version: u64,
    end_version: u64,
    _proof_version: u64,
    include_events: bool,
) -> TransactionListWithProof {
    // Include events if required
    let events = if include_events { Some(vec![]) } else { None };

    // Create the requested transactions
    let mut transactions = vec![];
    for sequence_number in start_version..=end_version {
        transactions.push(create_test_transaction(sequence_number));
    }

    // Create a transaction list with an empty proof
    let mut transaction_list_with_proof = TransactionListWithProof::new_empty();
    transaction_list_with_proof.first_transaction_version = Some(start_version);
    transaction_list_with_proof.events = events;
    transaction_list_with_proof.transactions = transactions;

    transaction_list_with_proof
}

/// Verifies that a new transaction outputs with proof response is received
/// and that the response contains the correct data.
async fn verify_new_transaction_outputs_with_proof(
    mock_client: &mut MockClient,
    receiver: Receiver<Result<bytes::Bytes, network::protocols::network::RpcError>>,
    output_list_with_proof: TransactionOutputListWithProof,
    expected_ledger_info: LedgerInfoWithSignatures,
) {
    match mock_client.wait_for_response(receiver).await.unwrap() {
        StorageServiceResponse::NewTransactionOutputsWithProof((
            outputs_with_proof,
            ledger_info,
        )) => {
            assert_eq!(outputs_with_proof, output_list_with_proof);
            assert_eq!(ledger_info, expected_ledger_info);
        }
        response => panic!(
            "Expected new transaction outputs with proof but got: {:?}",
            response
        ),
    };
}

/// Verifies that a new transactions with proof response is received
/// and that the response contains the correct data.
async fn verify_new_transactions_with_proof(
    mock_client: &mut MockClient,
    receiver: Receiver<Result<bytes::Bytes, network::protocols::network::RpcError>>,
    expected_transactions_with_proof: TransactionListWithProof,
    expected_ledger_info: LedgerInfoWithSignatures,
) {
    match mock_client.wait_for_response(receiver).await.unwrap() {
        StorageServiceResponse::NewTransactionsWithProof((
            transactions_with_proof,
            ledger_info,
        )) => {
            assert_eq!(transactions_with_proof, expected_transactions_with_proof);
            assert_eq!(ledger_info, expected_ledger_info);
        }
        response => panic!(
            "Expected new transaction with proof but got: {:?}",
            response
        ),
    };
}

/// Initializes the Aptos logger for tests
pub fn initialize_logger() {
    aptos_logger::Logger::builder()
        .is_async(false)
        .level(Level::Debug)
        .build();
}

/// Creates a mock database reader
pub fn create_mock_db_reader() -> MockDatabaseReader {
    MockDatabaseReader::new()
}

// This automatically creates a MockDatabaseReader.
// TODO(joshlind): if we frequently use these mocks, we should define a single
// mock test crate to be shared across the codebase.
mock! {
    pub DatabaseReader {}
    impl DbReader for DatabaseReader {
        fn get_epoch_ending_ledger_infos(
            &self,
            start_epoch: u64,
            end_epoch: u64,
        ) -> Result<EpochChangeProof>;

        fn get_transactions(
            &self,
            start_version: Version,
            batch_size: u64,
            ledger_version: Version,
            fetch_events: bool,
        ) -> Result<TransactionListWithProof>;

        fn get_transaction_by_hash(
            &self,
            hash: HashValue,
            ledger_version: Version,
            fetch_events: bool,
        ) -> Result<Option<TransactionWithProof>>;

        fn get_transaction_by_version(
            &self,
            version: Version,
            ledger_version: Version,
            fetch_events: bool,
        ) -> Result<TransactionWithProof>;

        fn get_first_txn_version(&self) -> Result<Option<Version>>;

        fn get_first_write_set_version(&self) -> Result<Option<Version>>;

        fn get_transaction_outputs(
            &self,
            start_version: Version,
            limit: u64,
            ledger_version: Version,
        ) -> Result<TransactionOutputListWithProof>;

        fn get_events(
            &self,
            event_key: &EventKey,
            start: u64,
            order: Order,
            limit: u64,
        ) -> Result<Vec<EventWithVersion>>;

        fn get_block_timestamp(&self, version: u64) -> Result<u64>;

        fn get_last_version_before_timestamp(
            &self,
            _timestamp: u64,
            _ledger_version: Version,
        ) -> Result<Version>;

        fn get_latest_state_value(&self, state_key: StateKey) -> Result<Option<StateValue>>;

        fn get_latest_ledger_info_option(&self) -> Result<Option<LedgerInfoWithSignatures>>;

        fn get_latest_ledger_info(&self) -> Result<LedgerInfoWithSignatures>;

        fn get_latest_version_option(&self) -> Result<Option<Version>>;

        fn get_latest_version(&self) -> Result<Version>;

        fn get_latest_commit_metadata(&self) -> Result<(Version, u64)>;

        fn get_startup_info(&self) -> Result<Option<StartupInfo>>;

        fn get_account_transaction(
            &self,
            address: AccountAddress,
            seq_num: u64,
            include_events: bool,
            ledger_version: Version,
        ) -> Result<Option<TransactionWithProof>>;

        fn get_account_transactions(
            &self,
            address: AccountAddress,
            seq_num: u64,
            limit: u64,
            include_events: bool,
            ledger_version: Version,
        ) -> Result<AccountTransactionsWithProof>;

        fn get_state_proof_with_ledger_info(
            &self,
            known_version: u64,
            ledger_info: LedgerInfoWithSignatures,
        ) -> Result<StateProof>;

        fn get_state_proof(&self, known_version: u64) -> Result<StateProof>;

        fn get_state_value_with_proof_by_version(
            &self,
            state_key: &StateKey,
            version: Version,
        ) -> Result<(Option<StateValue>, SparseMerkleProof)>;

        fn get_latest_executed_trees(&self) -> Result<ExecutedTrees>;

        fn get_epoch_ending_ledger_info(&self, known_version: u64) -> Result<LedgerInfoWithSignatures>;

        fn get_latest_transaction_info_option(&self) -> Result<Option<(Version, TransactionInfo)>>;

        fn get_accumulator_root_hash(&self, _version: Version) -> Result<HashValue>;

        fn get_accumulator_consistency_proof(
            &self,
            _client_known_version: Option<Version>,
            _ledger_version: Version,
        ) -> Result<AccumulatorConsistencyProof>;

        fn get_accumulator_summary(
            &self,
            ledger_version: Version,
        ) -> Result<TransactionAccumulatorSummary>;

        fn get_state_leaf_count(&self, version: Version) -> Result<usize>;

        fn get_state_value_chunk_with_proof(
            &self,
            version: Version,
            start_idx: usize,
            chunk_size: usize,
        ) -> Result<StateValueChunkWithProof>;

        fn get_state_prune_window(&self) -> Result<Option<usize>>;
    }
}

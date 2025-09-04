// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    data_notification::{
        DataClientRequest, DataPayload, EpochEndingLedgerInfosRequest,
        NewTransactionOutputsWithProofRequest, NewTransactionsOrOutputsWithProofRequest,
        NewTransactionsWithProofRequest, NumberOfStatesRequest, StateValuesWithProofRequest,
        SubscribeTransactionOutputsWithProofRequest,
        SubscribeTransactionsOrOutputsWithProofRequest, SubscribeTransactionsWithProofRequest,
        TransactionOutputsWithProofRequest, TransactionsOrOutputsWithProofRequest,
        TransactionsWithProofRequest,
    },
    data_stream::create_missing_data_request,
    stream_engine::{bound_by_range, DataStreamEngine, StreamEngine},
    streaming_client::{
        ContinuouslyStreamTransactionOutputsRequest, GetAllEpochEndingLedgerInfosRequest,
        GetAllStatesRequest, GetAllTransactionsRequest, StreamRequest,
    },
    tests::{utils, utils::create_ledger_info},
};
use velor_config::config::DataStreamingServiceConfig;
use velor_crypto::HashValue;
use velor_data_client::{global_summary::GlobalDataSummary, interface::ResponsePayload};
use velor_id_generator::U64IdGenerator;
use velor_storage_service_types::responses::CompleteDataRange;
use velor_types::{
    proof::{SparseMerkleRangeProof, TransactionInfoListWithProof},
    state_store::{
        state_key::StateKey,
        state_value::{StateValue, StateValueChunkWithProof},
    },
    transaction::{
        Transaction, TransactionAuxiliaryData, TransactionListWithProof,
        TransactionListWithProofV2, TransactionOutput, TransactionOutputListWithProof,
        TransactionOutputListWithProofV2, TransactionStatus, Version,
    },
    write_set::WriteSet,
};
use std::sync::Arc;

#[test]
fn test_bound_by_range() {
    // Test numbers beyond max
    assert_eq!(bound_by_range(11, 5, 10), 10);
    assert_eq!(bound_by_range(100, 10, 10), 10);
    assert_eq!(bound_by_range(1000, 12, 15), 15);

    // Test numbers below min
    assert_eq!(bound_by_range(4, 5, 10), 5);
    assert_eq!(bound_by_range(0, 10, 10), 10);
    assert_eq!(bound_by_range(11, 12, 15), 12);

    // Test numbers within the range
    assert_eq!(bound_by_range(9, 5, 10), 9);
    assert_eq!(bound_by_range(14, 5, 15), 14);
    assert_eq!(bound_by_range(20, 0, 20), 20);
    assert_eq!(bound_by_range(10, 10, 15), 10);
    assert_eq!(bound_by_range(13, 12, 15), 13);
}

#[test]
fn create_missing_data_request_trivial_request_types() {
    // Enumerate all data request types that are trivially satisfied
    let trivial_client_requests = vec![
        DataClientRequest::NewTransactionOutputsWithProof(NewTransactionOutputsWithProofRequest {
            known_version: 0,
            known_epoch: 0,
        }),
        DataClientRequest::NewTransactionsWithProof(NewTransactionsWithProofRequest {
            known_version: 0,
            known_epoch: 0,
            include_events: false,
        }),
        DataClientRequest::NewTransactionsOrOutputsWithProof(
            NewTransactionsOrOutputsWithProofRequest {
                known_version: 0,
                known_epoch: 0,
                include_events: false,
            },
        ),
        DataClientRequest::NumberOfStates(NumberOfStatesRequest { version: 0 }),
        DataClientRequest::SubscribeTransactionOutputsWithProof(
            SubscribeTransactionOutputsWithProofRequest {
                known_version: 0,
                known_epoch: 0,
                subscription_stream_id: 0,
                subscription_stream_index: 0,
            },
        ),
        DataClientRequest::SubscribeTransactionsWithProof(SubscribeTransactionsWithProofRequest {
            known_version: 0,
            known_epoch: 0,
            include_events: false,
            subscription_stream_id: 0,
            subscription_stream_index: 0,
        }),
        DataClientRequest::SubscribeTransactionsOrOutputsWithProof(
            SubscribeTransactionsOrOutputsWithProofRequest {
                known_version: 0,
                known_epoch: 0,
                subscription_stream_id: 0,
                subscription_stream_index: 0,
                include_events: false,
            },
        ),
    ];

    // Verify that the missing data request is empty
    for data_client_request in trivial_client_requests {
        let missing_data_request =
            create_missing_data_request(&data_client_request, &ResponsePayload::NumberOfStates(0))
                .unwrap();
        assert!(missing_data_request.is_none());
    }
}

#[test]
fn create_missing_data_request_epoch_ending_ledger_infos() {
    // Create the data client request
    let start_epoch = 10;
    let end_epoch = 15;
    let data_client_request =
        DataClientRequest::EpochEndingLedgerInfos(EpochEndingLedgerInfosRequest {
            start_epoch,
            end_epoch,
        });

    // Create the partial response payload
    let last_response_epoch = end_epoch - 1;
    let epoch_ending_ledger_infos = (start_epoch..last_response_epoch + 1)
        .map(|epoch| create_ledger_info(epoch * 100, epoch, true))
        .collect::<Vec<_>>();
    let response_payload = ResponsePayload::EpochEndingLedgerInfos(epoch_ending_ledger_infos);

    // Create the missing data request and verify that it's valid
    let missing_data_request =
        create_missing_data_request(&data_client_request, &response_payload).unwrap();
    let expected_missing_data_request =
        DataClientRequest::EpochEndingLedgerInfos(EpochEndingLedgerInfosRequest {
            start_epoch: last_response_epoch + 1,
            end_epoch,
        });
    assert_eq!(missing_data_request.unwrap(), expected_missing_data_request);

    // Create a complete response payload
    let last_response_epoch = end_epoch;
    let epoch_ending_ledger_infos = (start_epoch..last_response_epoch + 1)
        .map(|epoch| create_ledger_info(epoch * 100, epoch, true))
        .collect::<Vec<_>>();
    let response_payload = ResponsePayload::EpochEndingLedgerInfos(epoch_ending_ledger_infos);

    // Create the missing data request and verify that it's empty
    let missing_data_request =
        create_missing_data_request(&data_client_request, &response_payload).unwrap();
    assert!(missing_data_request.is_none());
}

#[test]
fn create_missing_data_request_state_values() {
    // Create the data client request
    let version = 10;
    let start_index = 100;
    let end_index = 200;
    let data_client_request =
        DataClientRequest::StateValuesWithProof(StateValuesWithProofRequest {
            version,
            start_index,
            end_index,
        });

    // Create the partial response payload
    let last_response_index = end_index - 1;
    let raw_values = (start_index..last_response_index + 1)
        .map(|_| (StateKey::raw(&[]), StateValue::new_legacy(vec![].into())))
        .collect::<Vec<_>>();
    let response_payload = ResponsePayload::StateValuesWithProof(StateValueChunkWithProof {
        first_index: start_index,
        last_index: last_response_index,
        first_key: HashValue::zero(),
        last_key: HashValue::zero(),
        raw_values,
        proof: SparseMerkleRangeProof::new(vec![]),
        root_hash: HashValue::zero(),
    });

    // Create the missing data request and verify that it's valid
    let missing_data_request =
        create_missing_data_request(&data_client_request, &response_payload).unwrap();
    let expected_missing_data_request =
        DataClientRequest::StateValuesWithProof(StateValuesWithProofRequest {
            version,
            start_index: last_response_index + 1,
            end_index,
        });
    assert_eq!(missing_data_request.unwrap(), expected_missing_data_request);

    // Create a complete response payload
    let last_response_index = end_index;
    let raw_values = (start_index..last_response_index + 1)
        .map(|_| (StateKey::raw(&[]), StateValue::new_legacy(vec![].into())))
        .collect::<Vec<_>>();
    let response_payload = ResponsePayload::StateValuesWithProof(StateValueChunkWithProof {
        first_index: start_index,
        last_index: last_response_index,
        first_key: HashValue::zero(),
        last_key: HashValue::zero(),
        raw_values,
        proof: SparseMerkleRangeProof::new(vec![]),
        root_hash: HashValue::zero(),
    });

    // Create the missing data request and verify that it's empty
    let missing_data_request =
        create_missing_data_request(&data_client_request, &response_payload).unwrap();
    assert!(missing_data_request.is_none());
}

#[test]
fn create_missing_data_request_transactions() {
    // Create the data client request
    let start_version = 100;
    let end_version = 200;
    let data_client_request =
        DataClientRequest::TransactionsWithProof(TransactionsWithProofRequest {
            start_version,
            end_version,
            proof_version: end_version,
            include_events: true,
        });

    // Create the partial response payload
    let last_response_version = end_version - 50;
    let transactions = (start_version..last_response_version + 1)
        .map(|_| create_test_transaction())
        .collect::<Vec<_>>();
    let transaction_list_with_proof =
        create_transaction_list_with_proof(start_version, transactions);
    let response_payload = ResponsePayload::TransactionsWithProof(transaction_list_with_proof);

    // Create the missing data request and verify that it's valid
    let missing_data_request =
        create_missing_data_request(&data_client_request, &response_payload).unwrap();
    let expected_missing_data_request =
        DataClientRequest::TransactionsWithProof(TransactionsWithProofRequest {
            start_version: last_response_version + 1,
            end_version,
            proof_version: end_version,
            include_events: true,
        });
    assert_eq!(missing_data_request.unwrap(), expected_missing_data_request);

    // Create a complete response payload
    let last_response_version = end_version;
    let transactions = (start_version..last_response_version + 1)
        .map(|_| create_test_transaction())
        .collect::<Vec<_>>();
    let transaction_list_with_proof =
        create_transaction_list_with_proof(start_version, transactions);
    let response_payload = ResponsePayload::TransactionsWithProof(transaction_list_with_proof);

    // Create the missing data request and verify that it's empty
    let missing_data_request =
        create_missing_data_request(&data_client_request, &response_payload).unwrap();
    assert!(missing_data_request.is_none());
}

#[test]
fn create_missing_data_request_transaction_outputs() {
    // Create the data client request
    let start_version = 1000;
    let end_version = 2000;
    let data_client_request =
        DataClientRequest::TransactionOutputsWithProof(TransactionOutputsWithProofRequest {
            start_version,
            end_version,
            proof_version: end_version,
        });

    // Create the partial response payload
    let last_response_version = end_version - 1000;
    let transactions_and_outputs = (start_version..last_response_version + 1)
        .map(|_| (create_test_transaction(), create_test_transaction_output()))
        .collect::<Vec<_>>();
    let output_list_with_proof =
        create_output_list_with_proof(start_version, transactions_and_outputs);
    let response_payload = ResponsePayload::TransactionOutputsWithProof(output_list_with_proof);

    // Create the missing data request and verify that it's valid
    let missing_data_request =
        create_missing_data_request(&data_client_request, &response_payload).unwrap();
    let expected_missing_data_request =
        DataClientRequest::TransactionOutputsWithProof(TransactionOutputsWithProofRequest {
            start_version: last_response_version + 1,
            end_version,
            proof_version: end_version,
        });
    assert_eq!(missing_data_request.unwrap(), expected_missing_data_request);

    // Create a complete response payload
    let last_response_version = end_version;
    let transactions_and_outputs = (start_version..last_response_version + 1)
        .map(|_| (create_test_transaction(), create_test_transaction_output()))
        .collect::<Vec<_>>();
    let output_list_with_proof =
        create_output_list_with_proof(start_version, transactions_and_outputs);
    let response_payload = ResponsePayload::TransactionOutputsWithProof(output_list_with_proof);

    // Create the missing data request and verify that it's empty
    let missing_data_request =
        create_missing_data_request(&data_client_request, &response_payload).unwrap();
    assert!(missing_data_request.is_none());
}

#[test]
fn create_missing_data_request_transactions_or_outputs() {
    // Create the data client request
    let start_version = 0;
    let end_version = 2000;
    let data_client_request =
        DataClientRequest::TransactionsOrOutputsWithProof(TransactionsOrOutputsWithProofRequest {
            start_version,
            end_version,
            include_events: true,
            proof_version: end_version,
        });

    // Create a partial response payload with transactions
    let last_response_version = end_version - 500;
    let transactions = (start_version..last_response_version + 1)
        .map(|_| create_test_transaction())
        .collect::<Vec<_>>();
    let transaction_list_with_proof =
        create_transaction_list_with_proof(start_version, transactions);
    let response_payload_with_transactions =
        ResponsePayload::TransactionsWithProof(transaction_list_with_proof);

    // Create a partial response payload with transaction outputs
    let transactions_and_outputs = (start_version..last_response_version + 1)
        .map(|_| (create_test_transaction(), create_test_transaction_output()))
        .collect::<Vec<_>>();
    let output_list_with_proof =
        create_output_list_with_proof(start_version, transactions_and_outputs);
    let response_payload_with_transaction_outputs =
        ResponsePayload::TransactionOutputsWithProof(output_list_with_proof);

    // Create the missing data requests and verify that they are valid
    for response_payload in [
        response_payload_with_transactions,
        response_payload_with_transaction_outputs,
    ] {
        let missing_data_request =
            create_missing_data_request(&data_client_request, &response_payload).unwrap();
        let expected_missing_data_request = DataClientRequest::TransactionsOrOutputsWithProof(
            TransactionsOrOutputsWithProofRequest {
                start_version: last_response_version + 1,
                end_version,
                proof_version: end_version,
                include_events: true,
            },
        );
        assert_eq!(missing_data_request.unwrap(), expected_missing_data_request);
    }

    // Create a complete response payload with transactions
    let last_response_version = end_version;
    let transactions = (start_version..last_response_version + 1)
        .map(|_| create_test_transaction())
        .collect::<Vec<_>>();
    let transaction_list_with_proof =
        create_transaction_list_with_proof(start_version, transactions);
    let response_payload_with_transactions =
        ResponsePayload::TransactionsWithProof(transaction_list_with_proof);

    // Create a complete response payload with transaction outputs
    let transactions_and_outputs = (start_version..last_response_version + 1)
        .map(|_| (create_test_transaction(), create_test_transaction_output()))
        .collect::<Vec<_>>();
    let output_list_with_proof =
        create_output_list_with_proof(start_version, transactions_and_outputs);
    let response_payload_with_transaction_outputs =
        ResponsePayload::TransactionOutputsWithProof(output_list_with_proof);

    // Create the missing data requests and verify that they are empty
    for response_payload in [
        response_payload_with_transactions,
        response_payload_with_transaction_outputs,
    ] {
        let missing_data_request =
            create_missing_data_request(&data_client_request, &response_payload).unwrap();
        assert!(missing_data_request.is_none());
    }
}

#[test]
fn transform_epoch_ending_stream_notifications() {
    // Create an epoch ending stream request
    let start_epoch = 100;
    let stream_request =
        StreamRequest::GetAllEpochEndingLedgerInfos(GetAllEpochEndingLedgerInfosRequest {
            start_epoch,
        });

    // Create a global data summary with a single epoch range
    let end_epoch = 199;
    let mut global_data_summary = GlobalDataSummary::empty();
    global_data_summary
        .advertised_data
        .epoch_ending_ledger_infos = vec![CompleteDataRange::new(start_epoch, end_epoch).unwrap()];
    global_data_summary.optimal_chunk_sizes.epoch_chunk_size = 100;

    // Create a new epoch ending stream engine
    let mut stream_engine = match StreamEngine::new(
        DataStreamingServiceConfig::default(),
        &stream_request,
        &global_data_summary.advertised_data,
    )
    .unwrap()
    {
        StreamEngine::EpochEndingStreamEngine(stream_engine) => stream_engine,
        unexpected_engine => {
            panic!(
                "Expected epoch ending stream engine but got {:?}",
                unexpected_engine
            );
        },
    };

    // Verify the tracked stream indices
    assert_eq!(stream_engine.next_stream_epoch, start_epoch);
    assert_eq!(stream_engine.next_request_epoch, start_epoch);

    // Create a single data client request
    let notification_id_generator = create_notification_id_generator();
    let data_client_request = stream_engine
        .create_data_client_requests(
            1,
            1,
            0,
            &global_data_summary,
            notification_id_generator.clone(),
        )
        .unwrap();
    assert_eq!(data_client_request.len(), 1);

    // Create an empty client response
    let client_response_payload = ResponsePayload::EpochEndingLedgerInfos(vec![]);

    // Transform the client response into a notification and verify an error is returned
    let _ = stream_engine
        .transform_client_response_into_notification(
            &data_client_request[0].clone(),
            client_response_payload,
            notification_id_generator.clone(),
        )
        .unwrap_err();

    // Create a client response with an invalid epoch
    let invalid_ledger_infos = vec![create_ledger_info(0, start_epoch - 1, true)];
    let client_response_payload =
        ResponsePayload::EpochEndingLedgerInfos(invalid_ledger_infos.clone());

    // Transform the client response into a notification and verify the notification
    let data_notification = stream_engine
        .transform_client_response_into_notification(
            &data_client_request[0].clone(),
            client_response_payload,
            notification_id_generator.clone(),
        )
        .unwrap();
    assert_eq!(
        data_notification.unwrap().data_payload,
        DataPayload::EpochEndingLedgerInfos(invalid_ledger_infos)
    );

    // Verify the tracked stream indices
    assert_eq!(stream_engine.next_stream_epoch, start_epoch + 1);
    assert_eq!(stream_engine.next_request_epoch, end_epoch + 1);

    // Create a partial client response
    let partial_ledger_infos = (start_epoch + 1..end_epoch)
        .map(|epoch| create_ledger_info(epoch * 100, epoch, true))
        .collect::<Vec<_>>();
    let client_response_payload =
        ResponsePayload::EpochEndingLedgerInfos(partial_ledger_infos.clone());

    // Transform the client response into a notification
    let data_client_request =
        DataClientRequest::EpochEndingLedgerInfos(EpochEndingLedgerInfosRequest {
            start_epoch: start_epoch + 1,
            end_epoch,
        });
    let _ = stream_engine
        .transform_client_response_into_notification(
            &data_client_request,
            client_response_payload,
            notification_id_generator,
        )
        .unwrap();

    // Verify the tracked stream indices
    assert_eq!(stream_engine.next_stream_epoch, end_epoch);
    assert_eq!(stream_engine.next_request_epoch, end_epoch + 1);
}

#[test]
fn transform_state_values_stream_notifications() {
    // Create a state values stream request
    let version = 100;
    let start_index = 1000;
    let stream_request = StreamRequest::GetAllStates(GetAllStatesRequest {
        version,
        start_index,
    });

    // Create a global data summary with a single state range
    let mut global_data_summary = GlobalDataSummary::empty();
    global_data_summary.advertised_data.states =
        vec![CompleteDataRange::new(start_index, start_index).unwrap()];
    global_data_summary.optimal_chunk_sizes.state_chunk_size = 20_000;

    // Create a new state values stream engine
    let mut stream_engine = match StreamEngine::new(
        DataStreamingServiceConfig::default(),
        &stream_request,
        &global_data_summary.advertised_data,
    )
    .unwrap()
    {
        StreamEngine::StateStreamEngine(stream_engine) => stream_engine,
        unexpected_engine => {
            panic!(
                "Expected state values stream engine but got {:?}",
                unexpected_engine
            );
        },
    };

    // Update the number of states for the stream
    let number_of_states = 10_000;
    stream_engine.number_of_states = Some(number_of_states);

    // Verify the tracked stream indices
    assert_eq!(stream_engine.next_stream_index, start_index);
    assert_eq!(stream_engine.next_request_index, start_index);

    // Create a single data client request
    let notification_id_generator = create_notification_id_generator();
    let data_client_request = stream_engine
        .create_data_client_requests(
            1,
            1,
            0,
            &global_data_summary,
            notification_id_generator.clone(),
        )
        .unwrap();
    assert_eq!(data_client_request.len(), 1);

    // Create an empty client response
    let client_response_payload = ResponsePayload::StateValuesWithProof(create_state_value_chunk(
        start_index,
        start_index - 1,
        0,
    ));

    // Transform the client response into a notification and verify an error is returned
    let _ = stream_engine
        .transform_client_response_into_notification(
            &data_client_request[0].clone(),
            client_response_payload,
            notification_id_generator.clone(),
        )
        .unwrap_err();

    // Create a client response with an invalid last index
    let state_value_chunk_with_proof = create_state_value_chunk(start_index, start_index - 1, 1);
    let client_response_payload =
        ResponsePayload::StateValuesWithProof(state_value_chunk_with_proof.clone());

    // Transform the client response into a notification and verify the notification
    let data_notification = stream_engine
        .transform_client_response_into_notification(
            &data_client_request[0].clone(),
            client_response_payload,
            notification_id_generator.clone(),
        )
        .unwrap();
    assert_eq!(
        data_notification.unwrap().data_payload,
        DataPayload::StateValuesWithProof(state_value_chunk_with_proof)
    );

    // Verify the tracked stream indices
    assert_eq!(stream_engine.next_stream_index, start_index + 1);
    assert_eq!(stream_engine.next_request_index, number_of_states);

    // Create a partial client response
    let last_index = number_of_states - 500;
    let state_value_chunk_with_proof =
        create_state_value_chunk(start_index, last_index, last_index - start_index);
    let client_response_payload =
        ResponsePayload::StateValuesWithProof(state_value_chunk_with_proof.clone());

    // Transform the client response into a notification
    let data_client_request =
        DataClientRequest::StateValuesWithProof(StateValuesWithProofRequest {
            version,
            start_index: start_index + 1,
            end_index: number_of_states - 1,
        });
    let _ = stream_engine
        .transform_client_response_into_notification(
            &data_client_request,
            client_response_payload,
            notification_id_generator,
        )
        .unwrap();

    // Verify the tracked stream indices
    assert_eq!(stream_engine.next_stream_index, last_index + 1);
    assert_eq!(stream_engine.next_request_index, number_of_states);
}

#[test]
fn transform_transactions_stream_notifications() {
    // Create a transactions stream request
    let start_version = 100;
    let end_version = 200;
    let stream_request = StreamRequest::GetAllTransactions(GetAllTransactionsRequest {
        start_version,
        end_version,
        proof_version: end_version,
        include_events: true,
    });

    // Create a global data summary with a single transaction range
    let mut global_data_summary = GlobalDataSummary::empty();
    global_data_summary.advertised_data.transactions =
        vec![CompleteDataRange::new(start_version, end_version).unwrap()];
    global_data_summary
        .optimal_chunk_sizes
        .transaction_chunk_size = 10_000;

    // Create a new transactions stream engine
    let mut stream_engine = match StreamEngine::new(
        DataStreamingServiceConfig::default(),
        &stream_request,
        &global_data_summary.advertised_data,
    )
    .unwrap()
    {
        StreamEngine::TransactionStreamEngine(stream_engine) => stream_engine,
        unexpected_engine => {
            panic!(
                "Expected transactions stream engine but got {:?}",
                unexpected_engine
            );
        },
    };

    // Verify the tracked stream indices
    assert_eq!(stream_engine.next_stream_version, start_version);
    assert_eq!(stream_engine.next_request_version, start_version);

    // Create a single data client request
    let notification_id_generator = create_notification_id_generator();
    let data_client_request = stream_engine
        .create_data_client_requests(
            1,
            1,
            0,
            &global_data_summary,
            notification_id_generator.clone(),
        )
        .unwrap();
    assert_eq!(data_client_request.len(), 1);

    // Create an empty client response
    let client_response_payload =
        ResponsePayload::TransactionsWithProof(TransactionListWithProofV2::new_empty());

    // Transform the client response into a notification and verify an error is returned
    let _ = stream_engine
        .transform_client_response_into_notification(
            &data_client_request[0].clone(),
            client_response_payload,
            notification_id_generator.clone(),
        )
        .unwrap_err();

    // Create a partial client response
    let last_version = end_version - 50;
    let transactions_with_proof =
        utils::create_transaction_list_with_proof(start_version, last_version, true);
    let client_response_payload =
        ResponsePayload::TransactionsWithProof(transactions_with_proof.clone());

    // Transform the client response into a notification
    let data_client_request =
        DataClientRequest::TransactionsWithProof(TransactionsWithProofRequest {
            start_version,
            end_version,
            proof_version: end_version,
            include_events: true,
        });
    let _ = stream_engine
        .transform_client_response_into_notification(
            &data_client_request,
            client_response_payload,
            notification_id_generator,
        )
        .unwrap();

    // Verify the tracked stream indices
    assert_eq!(stream_engine.next_stream_version, last_version + 1);
    assert_eq!(stream_engine.next_request_version, end_version + 1);
}

#[test]
fn transform_continuous_outputs_stream_notifications() {
    // Create a continuous outputs stream request
    let known_version = 1000;
    let known_epoch = 10;
    let stream_request = StreamRequest::ContinuouslyStreamTransactionOutputs(
        ContinuouslyStreamTransactionOutputsRequest {
            known_version,
            known_epoch,
            target: None,
        },
    );

    // Create a global data summary with a single transaction range
    let mut global_data_summary = GlobalDataSummary::empty();
    global_data_summary.advertised_data.transaction_outputs =
        vec![CompleteDataRange::new(known_version, known_version).unwrap()];
    global_data_summary
        .optimal_chunk_sizes
        .transaction_output_chunk_size = 10_000;

    // Create a new continuous outputs stream engine
    let mut stream_engine = match StreamEngine::new(
        DataStreamingServiceConfig::default(),
        &stream_request,
        &global_data_summary.advertised_data,
    )
    .unwrap()
    {
        StreamEngine::ContinuousTransactionStreamEngine(stream_engine) => stream_engine,
        unexpected_engine => {
            panic!(
                "Expected continuous outputs stream engine but got {:?}",
                unexpected_engine
            );
        },
    };

    // Set the target ledger info for the stream
    let target_version = known_version + 1000;
    stream_engine.current_target_ledger_info =
        Some(create_ledger_info(target_version, known_epoch, false));

    // Verify the tracked stream indices
    assert_eq!(
        stream_engine.next_request_version_and_epoch,
        (known_version + 1, known_epoch)
    );
    assert_eq!(
        stream_engine.next_stream_version_and_epoch,
        (known_version + 1, known_epoch)
    );

    // Create a single data client request
    let notification_id_generator = create_notification_id_generator();
    let data_client_request = stream_engine
        .create_data_client_requests(
            1,
            1,
            0,
            &global_data_summary,
            notification_id_generator.clone(),
        )
        .unwrap();
    assert_eq!(data_client_request.len(), 1);

    // Create an empty client response
    let client_response_payload =
        ResponsePayload::TransactionOutputsWithProof(TransactionOutputListWithProofV2::new_empty());

    // Transform the client response into a notification and verify an error is returned
    let _ = stream_engine
        .transform_client_response_into_notification(
            &data_client_request[0].clone(),
            client_response_payload,
            notification_id_generator.clone(),
        )
        .unwrap_err();

    // Create a partial client response
    let last_version = target_version - 10;
    let transactions_and_outputs = (known_version..last_version + 1)
        .map(|_| (create_test_transaction(), create_test_transaction_output()))
        .collect::<Vec<_>>();
    let output_list_with_proof =
        create_output_list_with_proof(known_version, transactions_and_outputs);
    let client_response_payload =
        ResponsePayload::TransactionOutputsWithProof(output_list_with_proof);

    // Transform the client response into a notification
    let data_client_request =
        DataClientRequest::TransactionOutputsWithProof(TransactionOutputsWithProofRequest {
            start_version: known_version + 1,
            end_version: last_version,
            proof_version: target_version,
        });
    let _ = stream_engine
        .transform_client_response_into_notification(
            &data_client_request,
            client_response_payload,
            notification_id_generator,
        )
        .unwrap();

    // Verify the tracked stream indices
    assert_eq!(
        stream_engine.next_stream_version_and_epoch,
        (last_version + 1, known_epoch)
    );
    assert_eq!(
        stream_engine.next_request_version_and_epoch,
        (target_version + 1, known_epoch)
    );
}

/// Returns a simple notification ID generator for testing purposes
fn create_notification_id_generator() -> Arc<U64IdGenerator> {
    Arc::new(U64IdGenerator::new())
}

/// Creates an output list with proof for testing purposes
fn create_output_list_with_proof(
    start_version: Version,
    transactions_and_outputs: Vec<(Transaction, TransactionOutput)>,
) -> TransactionOutputListWithProofV2 {
    let output_list_with_proof = TransactionOutputListWithProof {
        transactions_and_outputs,
        proof: TransactionInfoListWithProof::new_empty(),
        first_transaction_output_version: Some(start_version),
    };
    TransactionOutputListWithProofV2::new_from_v1(output_list_with_proof)
}

/// Returns a state value chunk with proof for testing purposes
fn create_state_value_chunk(
    first_index: u64,
    last_index: u64,
    num_values: u64,
) -> StateValueChunkWithProof {
    // Create the raw values
    let raw_values = (0..num_values)
        .map(|_| (StateKey::raw(&[]), StateValue::new_legacy(vec![].into())))
        .collect::<Vec<_>>();

    // Create the chunk of state values
    StateValueChunkWithProof {
        first_index,
        last_index,
        first_key: HashValue::zero(),
        last_key: HashValue::zero(),
        raw_values,
        proof: SparseMerkleRangeProof::new(vec![]),
        root_hash: HashValue::zero(),
    }
}

/// Returns a dummy transaction for testing purposes
fn create_test_transaction() -> Transaction {
    Transaction::StateCheckpoint(HashValue::zero())
}

/// Returns a dummy transaction output for testing purposes
fn create_test_transaction_output() -> TransactionOutput {
    TransactionOutput::new(
        WriteSet::default(),
        vec![],
        0,
        TransactionStatus::Retry,
        TransactionAuxiliaryData::default(),
    )
}

/// Creates a transaction list with proof for testing purposes
fn create_transaction_list_with_proof(
    start_version: Version,
    transactions: Vec<Transaction>,
) -> TransactionListWithProofV2 {
    let transaction_list_with_proof = TransactionListWithProof {
        transactions,
        events: None,
        first_transaction_version: Some(start_version),
        proof: TransactionInfoListWithProof::new_empty(),
    };
    TransactionListWithProofV2::new_from_v1(transaction_list_with_proof)
}

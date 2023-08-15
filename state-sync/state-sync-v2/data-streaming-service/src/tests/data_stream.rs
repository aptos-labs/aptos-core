// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    data_notification::{
        DataClientRequest, DataPayload, EpochEndingLedgerInfosRequest,
        NewTransactionOutputsWithProofRequest, NewTransactionsOrOutputsWithProofRequest,
        NewTransactionsWithProofRequest, PendingClientResponse, TransactionOutputsWithProofRequest,
        TransactionsOrOutputsWithProofRequest, TransactionsWithProofRequest,
    },
    data_stream::{DataStream, DataStreamListener},
    streaming_client::{
        ContinuouslyStreamTransactionOutputsRequest,
        ContinuouslyStreamTransactionsOrOutputsRequest, ContinuouslyStreamTransactionsRequest,
        GetAllEpochEndingLedgerInfosRequest, GetAllStatesRequest, GetAllTransactionOutputsRequest,
        GetAllTransactionsOrOutputsRequest, GetAllTransactionsRequest, NotificationFeedback,
        StreamRequest,
    },
    tests::utils::{
        create_data_client_response, create_ledger_info, create_output_list_with_proof,
        create_random_u64, create_transaction_list_with_proof, get_data_notification,
        initialize_logger, MockAptosDataClient, NoopResponseCallback, MAX_ADVERTISED_EPOCH_END,
        MAX_ADVERTISED_STATES, MAX_ADVERTISED_TRANSACTION, MAX_ADVERTISED_TRANSACTION_OUTPUT,
        MAX_NOTIFICATION_TIMEOUT_SECS, MIN_ADVERTISED_EPOCH_END, MIN_ADVERTISED_STATES,
        MIN_ADVERTISED_TRANSACTION, MIN_ADVERTISED_TRANSACTION_OUTPUT,
    },
};
use aptos_config::config::{AptosDataClientConfig, DataStreamingServiceConfig};
use aptos_data_client::{
    global_summary::{AdvertisedData, GlobalDataSummary, OptimalChunkSizes},
    interface::{Response, ResponseContext, ResponsePayload},
};
use aptos_id_generator::U64IdGenerator;
use aptos_infallible::Mutex;
use aptos_storage_service_types::responses::CompleteDataRange;
use aptos_types::{
    ledger_info::LedgerInfoWithSignatures, proof::SparseMerkleRangeProof,
    state_store::state_value::StateValueChunkWithProof, transaction::Version,
};
use claims::{assert_err, assert_ge, assert_matches, assert_none, assert_ok};
use futures::{FutureExt, StreamExt};
use std::{sync::Arc, time::Duration};
use tokio::time::timeout;

#[tokio::test]
async fn test_stream_blocked() {
    // Create a state value stream
    let streaming_service_config = DataStreamingServiceConfig::default();
    let (mut data_stream, mut stream_listener) = create_state_value_stream(
        AptosDataClientConfig::default(),
        streaming_service_config,
        MIN_ADVERTISED_STATES,
    );

    // Initialize the data stream
    let global_data_summary = create_global_data_summary(100);
    initialize_data_requests(&mut data_stream, &global_data_summary);

    let mut number_of_refetches = 0;
    loop {
        // Clear the pending queue and insert a response with an invalid type
        let client_request =
            DataClientRequest::EpochEndingLedgerInfos(EpochEndingLedgerInfosRequest {
                start_epoch: 0,
                end_epoch: 0,
            });
        let context = ResponseContext {
            id: 0,
            response_callback: Box::new(NoopResponseCallback),
        };
        let pending_response = PendingClientResponse {
            client_request: client_request.clone(),
            client_response: Some(Ok(Response {
                context,
                payload: ResponsePayload::NumberOfStates(10),
            })),
        };
        insert_response_into_pending_queue(&mut data_stream, pending_response);

        // Process the data responses and force a data re-fetch
        process_data_responses(&mut data_stream, &global_data_summary).await;

        // If we're sent a data notification, verify it's an end of stream notification!
        if let Ok(data_notification) = timeout(
            Duration::from_secs(MAX_NOTIFICATION_TIMEOUT_SECS),
            stream_listener.select_next_some(),
        )
        .await
        {
            match data_notification.data_payload {
                DataPayload::EndOfStream => {
                    assert_eq!(
                        number_of_refetches,
                        streaming_service_config.max_request_retry
                    );

                    // Provide incorrect feedback for the notification
                    assert_err!(data_stream.handle_notification_feedback(
                        &data_notification.notification_id,
                        &NotificationFeedback::PayloadTypeIsIncorrect
                    ));

                    // Provide valid feedback for the notification
                    assert_ok!(data_stream.handle_notification_feedback(
                        &data_notification.notification_id,
                        &NotificationFeedback::EndOfStream,
                    ));
                    return;
                },
                data_payload => panic!("Unexpected payload type: {:?}", data_payload),
            }
        }
        number_of_refetches += 1;
    }
}

#[tokio::test]
async fn test_stream_garbage_collection() {
    // Create a transaction stream
    let streaming_service_config = DataStreamingServiceConfig::default();
    let (mut data_stream, mut stream_listener) = create_transaction_stream(
        AptosDataClientConfig::default(),
        streaming_service_config,
        MIN_ADVERTISED_TRANSACTION_OUTPUT,
        MAX_ADVERTISED_TRANSACTION_OUTPUT,
    );

    // Initialize the data stream
    let global_data_summary = create_global_data_summary(1);
    initialize_data_requests(&mut data_stream, &global_data_summary);

    loop {
        // Insert a transaction response into the queue
        set_transaction_response_at_queue_head(&mut data_stream);

        // Process the data response
        process_data_responses(&mut data_stream, &global_data_summary).await;

        // Process the data response
        let data_notification = get_data_notification(&mut stream_listener).await.unwrap();
        if matches!(data_notification.data_payload, DataPayload::EndOfStream) {
            return;
        }

        // Verify the notification to response map is garbage collected
        let (_, sent_notifications) = data_stream.get_sent_requests_and_notifications();
        assert!(
            (sent_notifications.len() as u64)
                <= streaming_service_config.max_notification_id_mappings
        );
    }
}

#[tokio::test]
async fn test_stream_initialization() {
    // Create an epoch ending data stream
    let streaming_service_config = DataStreamingServiceConfig::default();
    let (mut data_stream, _) = create_epoch_ending_stream(
        AptosDataClientConfig::default(),
        streaming_service_config,
        MIN_ADVERTISED_EPOCH_END,
    );

    // Verify the data stream is not initialized
    assert!(!data_stream.data_requests_initialized());

    // Initialize the data stream
    let global_data_summary = create_global_data_summary(1);
    initialize_data_requests(&mut data_stream, &global_data_summary);

    // Verify the data stream is now initialized
    assert!(data_stream.data_requests_initialized());

    // Verify that client requests have been made
    let (sent_requests, _) = data_stream.get_sent_requests_and_notifications();
    assert_ne!(sent_requests.as_ref().unwrap().len(), 0);
}

#[tokio::test]
async fn test_stream_data_error() {
    // Create an epoch ending data stream
    let streaming_service_config = DataStreamingServiceConfig::default();
    let (mut data_stream, mut stream_listener) = create_epoch_ending_stream(
        AptosDataClientConfig::default(),
        streaming_service_config,
        MIN_ADVERTISED_EPOCH_END,
    );

    // Initialize the data stream
    let global_data_summary = create_global_data_summary(100);
    initialize_data_requests(&mut data_stream, &global_data_summary);

    // Clear the pending queue and insert an error response
    let client_request = DataClientRequest::EpochEndingLedgerInfos(EpochEndingLedgerInfosRequest {
        start_epoch: MIN_ADVERTISED_EPOCH_END,
        end_epoch: MIN_ADVERTISED_EPOCH_END + 1,
    });
    let pending_response = PendingClientResponse {
        client_request: client_request.clone(),
        client_response: Some(Err(aptos_data_client::error::Error::DataIsUnavailable(
            "Missing data!".into(),
        ))),
    };
    insert_response_into_pending_queue(&mut data_stream, pending_response);

    // Process the responses and verify the data client request was resent to the network
    process_data_responses(&mut data_stream, &global_data_summary).await;
    assert_none!(stream_listener.select_next_some().now_or_never());
    verify_client_request_resubmitted(&mut data_stream, client_request);
}

#[tokio::test]
async fn test_stream_invalid_response() {
    // Create an epoch ending data stream
    let streaming_service_config = DataStreamingServiceConfig::default();
    let (mut data_stream, mut stream_listener) = create_epoch_ending_stream(
        AptosDataClientConfig::default(),
        streaming_service_config,
        MIN_ADVERTISED_EPOCH_END,
    );

    // Initialize the data stream
    let global_data_summary = create_global_data_summary(100);
    initialize_data_requests(&mut data_stream, &global_data_summary);

    // Clear the pending queue and insert a response with an invalid type
    let client_request = DataClientRequest::EpochEndingLedgerInfos(EpochEndingLedgerInfosRequest {
        start_epoch: MIN_ADVERTISED_EPOCH_END,
        end_epoch: MIN_ADVERTISED_EPOCH_END + 1,
    });
    let context = ResponseContext {
        id: 0,
        response_callback: Box::new(NoopResponseCallback),
    };
    let client_response = Response::new(context, ResponsePayload::NumberOfStates(10));
    let pending_response = PendingClientResponse {
        client_request: client_request.clone(),
        client_response: Some(Ok(client_response)),
    };
    insert_response_into_pending_queue(&mut data_stream, pending_response);

    // Process the responses and verify the data client request was resent to the network
    process_data_responses(&mut data_stream, &global_data_summary).await;
    assert_none!(stream_listener.select_next_some().now_or_never());
    verify_client_request_resubmitted(&mut data_stream, client_request);
}

#[tokio::test]
async fn test_epoch_stream_out_of_order_responses() {
    // Create an epoch ending data stream
    let max_concurrent_requests = 3;
    let streaming_service_config = DataStreamingServiceConfig {
        max_concurrent_requests,
        max_concurrent_state_requests: 1,
        ..Default::default()
    };
    let (mut data_stream, mut stream_listener) = create_epoch_ending_stream(
        AptosDataClientConfig::default(),
        streaming_service_config,
        MIN_ADVERTISED_EPOCH_END,
    );

    // Initialize the data stream
    let global_data_summary = create_global_data_summary(1);
    initialize_data_requests(&mut data_stream, &global_data_summary);

    // Verify at least three requests have been made
    let (sent_requests, _) = data_stream.get_sent_requests_and_notifications();
    assert_ge!(
        sent_requests.as_ref().unwrap().len(),
        max_concurrent_requests as usize
    );

    // Set a response for the second request and verify no notifications
    set_epoch_ending_response_in_queue(&mut data_stream, 1, 0);
    process_data_responses(&mut data_stream, &global_data_summary).await;
    assert_none!(stream_listener.select_next_some().now_or_never());

    // Set a response for the first request and verify two notifications
    set_epoch_ending_response_in_queue(&mut data_stream, 0, 0);
    process_data_responses(&mut data_stream, &global_data_summary).await;
    for _ in 0..2 {
        verify_epoch_ending_notification(
            &mut stream_listener,
            create_ledger_info(0, MIN_ADVERTISED_EPOCH_END, true),
        )
        .await;
    }
    assert_none!(stream_listener.select_next_some().now_or_never());

    // Set the response for the first and third request and verify one notification sent
    set_epoch_ending_response_in_queue(&mut data_stream, 0, 0);
    set_epoch_ending_response_in_queue(&mut data_stream, 2, 0);
    process_data_responses(&mut data_stream, &global_data_summary).await;
    verify_epoch_ending_notification(
        &mut stream_listener,
        create_ledger_info(0, MIN_ADVERTISED_EPOCH_END, true),
    )
    .await;
    assert_none!(stream_listener.select_next_some().now_or_never());

    // Set the response for the first and third request and verify three notifications sent
    set_epoch_ending_response_in_queue(&mut data_stream, 0, 0);
    set_epoch_ending_response_in_queue(&mut data_stream, 2, 0);
    process_data_responses(&mut data_stream, &global_data_summary).await;
    for _ in 0..3 {
        verify_epoch_ending_notification(
            &mut stream_listener,
            create_ledger_info(0, MIN_ADVERTISED_EPOCH_END, true),
        )
        .await;
    }
    assert_none!(stream_listener.select_next_some().now_or_never());
}

#[tokio::test]
async fn test_state_stream_out_of_order_responses() {
    // Create a state value data stream
    let max_concurrent_state_requests = 6;
    let streaming_service_config = DataStreamingServiceConfig {
        max_concurrent_requests: 1,
        max_concurrent_state_requests,
        ..Default::default()
    };
    let (mut data_stream, mut stream_listener) = create_state_value_stream(
        AptosDataClientConfig::default(),
        streaming_service_config,
        MIN_ADVERTISED_STATES,
    );

    // Initialize the data stream
    let global_data_summary = create_global_data_summary(1);
    initialize_data_requests(&mut data_stream, &global_data_summary);

    // Verify a single request is made (to fetch the number of state values)
    let (sent_requests, _) = data_stream.get_sent_requests_and_notifications();
    assert_eq!(sent_requests.as_ref().unwrap().len(), 1);

    // Set a response for the number of state values
    set_num_state_values_response_in_queue(&mut data_stream, 0);
    process_data_responses(&mut data_stream, &global_data_summary).await;

    // Verify at least six requests have been made
    let (sent_requests, _) = data_stream.get_sent_requests_and_notifications();
    assert_ge!(
        sent_requests.as_ref().unwrap().len(),
        max_concurrent_state_requests as usize
    );

    // Set a response for the second request and verify no notifications
    set_state_value_response_in_queue(&mut data_stream, 1);
    process_data_responses(&mut data_stream, &global_data_summary).await;
    assert_none!(stream_listener.select_next_some().now_or_never());

    // Set a response for the first request and verify two notifications
    set_state_value_response_in_queue(&mut data_stream, 0);
    process_data_responses(&mut data_stream, &global_data_summary).await;
    for _ in 0..2 {
        let data_notification = get_data_notification(&mut stream_listener).await.unwrap();
        assert_matches!(
            data_notification.data_payload,
            DataPayload::StateValuesWithProof(_)
        );
    }
    assert_none!(stream_listener.select_next_some().now_or_never());

    // Set the response for the first and third request and verify one notification sent
    set_state_value_response_in_queue(&mut data_stream, 0);
    set_state_value_response_in_queue(&mut data_stream, 2);
    process_data_responses(&mut data_stream, &global_data_summary).await;
    let data_notification = get_data_notification(&mut stream_listener).await.unwrap();
    assert_matches!(
        data_notification.data_payload,
        DataPayload::StateValuesWithProof(_)
    );
    assert_none!(stream_listener.select_next_some().now_or_never());

    // Set the response for the first and third request and verify three notifications sent
    set_state_value_response_in_queue(&mut data_stream, 0);
    set_state_value_response_in_queue(&mut data_stream, 2);
    process_data_responses(&mut data_stream, &global_data_summary).await;
    for _ in 0..3 {
        let data_notification = get_data_notification(&mut stream_listener).await.unwrap();
        assert_matches!(
            data_notification.data_payload,
            DataPayload::StateValuesWithProof(_)
        );
    }
    assert_none!(stream_listener.select_next_some().now_or_never());
}

#[tokio::test]
async fn test_continuous_stream_epoch_change_retry() {
    // Create a test streaming service config
    let max_request_retry = 10;
    let max_concurrent_requests = 3;
    let streaming_service_config = DataStreamingServiceConfig {
        max_concurrent_requests,
        max_request_retry,
        ..Default::default()
    };

    // Test both types of continuous data streams
    let (data_stream_1, _stream_listener_1) = create_continuous_transaction_stream(
        AptosDataClientConfig::default(),
        streaming_service_config,
        MIN_ADVERTISED_TRANSACTION,
        MIN_ADVERTISED_EPOCH_END,
    );
    let (data_stream_2, _stream_listener_2) = create_continuous_transaction_output_stream(
        AptosDataClientConfig::default(),
        streaming_service_config,
        MIN_ADVERTISED_TRANSACTION_OUTPUT,
        MIN_ADVERTISED_EPOCH_END,
    );
    let (data_stream_3, _stream_listener_3) = create_continuous_transaction_or_output_stream(
        AptosDataClientConfig::default(),
        streaming_service_config,
        MIN_ADVERTISED_TRANSACTION_OUTPUT,
        MIN_ADVERTISED_EPOCH_END,
    );
    for mut data_stream in [data_stream_1, data_stream_2, data_stream_3] {
        // Initialize the data stream and drive progress
        let global_data_summary = create_global_data_summary(1);
        initialize_data_requests(&mut data_stream, &global_data_summary);

        // Verify a single request is made
        let (sent_requests, _) = data_stream.get_sent_requests_and_notifications();
        assert_eq!(sent_requests.as_ref().unwrap().len(), 1);

        // Verify the request is for an epoch ending ledger info
        let client_request = get_pending_client_request(&mut data_stream, 0);
        let epoch_ending_request =
            DataClientRequest::EpochEndingLedgerInfos(EpochEndingLedgerInfosRequest {
                start_epoch: MIN_ADVERTISED_EPOCH_END,
                end_epoch: MIN_ADVERTISED_EPOCH_END,
            });
        assert_eq!(client_request, epoch_ending_request);

        // Handle multiple timeouts and retries
        for _ in 0..max_request_retry - 1 {
            // Set a timeout response for the epoch ending ledger info and process it
            set_timeout_response_in_queue(&mut data_stream, 0);
            process_data_responses(&mut data_stream, &global_data_summary).await;

            // Verify the data client request was resent to the network (retried)
            let client_request = get_pending_client_request(&mut data_stream, 0);
            assert_eq!(client_request, epoch_ending_request);
        }

        // Set an epoch ending response in the queue and process it
        set_epoch_ending_response_in_queue(&mut data_stream, 0, MIN_ADVERTISED_TRANSACTION + 100);
        process_data_responses(&mut data_stream, &global_data_summary).await;

        // Verify the correct number of data requests are now pending,
        // i.e., a target has been found and we're fetching data up to it.
        let (sent_requests, _) = data_stream.get_sent_requests_and_notifications();
        assert_eq!(sent_requests.as_ref().unwrap().len(), 3);
    }
}

#[tokio::test]
async fn test_continuous_stream_optimistic_fetch_retry() {
    // Create a test streaming service config
    let max_request_retry = 3;
    let max_concurrent_requests = 3;
    let streaming_service_config = DataStreamingServiceConfig {
        max_concurrent_requests,
        max_request_retry,
        ..Default::default()
    };

    // Test both types of continuous data streams
    let (data_stream_1, stream_listener_1) = create_continuous_transaction_stream(
        AptosDataClientConfig::default(),
        streaming_service_config,
        MAX_ADVERTISED_TRANSACTION,
        MAX_ADVERTISED_EPOCH_END,
    );
    let (data_stream_2, stream_listener_2) = create_continuous_transaction_output_stream(
        AptosDataClientConfig::default(),
        streaming_service_config,
        MAX_ADVERTISED_TRANSACTION_OUTPUT,
        MAX_ADVERTISED_EPOCH_END,
    );
    let (data_stream_3, stream_listener_3) = create_continuous_transaction_or_output_stream(
        AptosDataClientConfig::default(),
        streaming_service_config,
        MAX_ADVERTISED_TRANSACTION_OUTPUT,
        MAX_ADVERTISED_EPOCH_END,
    );
    for (mut data_stream, mut stream_listener, transactions_only, allow_transactions_or_outputs) in [
        (data_stream_1, stream_listener_1, true, false),
        (data_stream_2, stream_listener_2, false, false),
        (data_stream_3, stream_listener_3, false, true),
    ] {
        // Initialize the data stream
        let global_data_summary = create_global_data_summary(1);
        initialize_data_requests(&mut data_stream, &global_data_summary);

        // Verify a single request is made
        let (sent_requests, _) = data_stream.get_sent_requests_and_notifications();
        assert_eq!(sent_requests.as_ref().unwrap().len(), 1);

        // Verify the request is for the correct data
        let client_request = get_pending_client_request(&mut data_stream, 0);
        let expected_request = if allow_transactions_or_outputs {
            DataClientRequest::NewTransactionsOrOutputsWithProof(
                NewTransactionsOrOutputsWithProofRequest {
                    known_version: MAX_ADVERTISED_TRANSACTION_OUTPUT,
                    known_epoch: MAX_ADVERTISED_EPOCH_END,
                    include_events: false,
                },
            )
        } else if transactions_only {
            DataClientRequest::NewTransactionsWithProof(NewTransactionsWithProofRequest {
                known_version: MAX_ADVERTISED_TRANSACTION,
                known_epoch: MAX_ADVERTISED_EPOCH_END,
                include_events: false,
            })
        } else {
            DataClientRequest::NewTransactionOutputsWithProof(
                NewTransactionOutputsWithProofRequest {
                    known_version: MAX_ADVERTISED_TRANSACTION_OUTPUT,
                    known_epoch: MAX_ADVERTISED_EPOCH_END,
                },
            )
        };
        assert_eq!(client_request, expected_request);

        // Set a timeout response for the optimistic fetch request and process it
        set_timeout_response_in_queue(&mut data_stream, 0);
        process_data_responses(&mut data_stream, &global_data_summary).await;
        assert_none!(stream_listener.select_next_some().now_or_never());

        // Handle multiple timeouts and retries because no new data is known
        // about, so the best we can do is send optimistic fetches
        for _ in 0..max_request_retry * 3 {
            // Set a timeout response for the request and process it
            set_timeout_response_in_queue(&mut data_stream, 0);
            process_data_responses(&mut data_stream, &global_data_summary).await;

            // Verify the same optimistic fetch request was resent to the network
            let new_client_request = get_pending_client_request(&mut data_stream, 0);
            assert_eq!(new_client_request, client_request);
        }

        // Set an optimistic fetch response in the queue and process it
        set_optimistic_fetch_response_in_queue(
            &mut data_stream,
            0,
            MAX_ADVERTISED_TRANSACTION + 1,
            transactions_only,
        );
        process_data_responses(&mut data_stream, &global_data_summary).await;

        // Verify another optimistic fetch request is now sent (for data beyond the previous target)
        let (sent_requests, _) = data_stream.get_sent_requests_and_notifications();
        assert_eq!(sent_requests.as_ref().unwrap().len(), 1);
        let client_request = get_pending_client_request(&mut data_stream, 0);
        let expected_request = if allow_transactions_or_outputs {
            DataClientRequest::NewTransactionsOrOutputsWithProof(
                NewTransactionsOrOutputsWithProofRequest {
                    known_version: MAX_ADVERTISED_TRANSACTION_OUTPUT + 1,
                    known_epoch: MAX_ADVERTISED_EPOCH_END,
                    include_events: false,
                },
            )
        } else if transactions_only {
            DataClientRequest::NewTransactionsWithProof(NewTransactionsWithProofRequest {
                known_version: MAX_ADVERTISED_TRANSACTION + 1,
                known_epoch: MAX_ADVERTISED_EPOCH_END,
                include_events: false,
            })
        } else {
            DataClientRequest::NewTransactionOutputsWithProof(
                NewTransactionOutputsWithProofRequest {
                    known_version: MAX_ADVERTISED_TRANSACTION_OUTPUT + 1,
                    known_epoch: MAX_ADVERTISED_EPOCH_END,
                },
            )
        };
        assert_eq!(client_request, expected_request);

        // Set a timeout response for the optimistic fetch request and process it.
        // This will cause the same request to be re-sent.
        set_timeout_response_in_queue(&mut data_stream, 0);
        process_data_responses(&mut data_stream, &global_data_summary).await;

        // Set a timeout response for the optimistic fetch request and process it,
        // but this time the node knows about new data to fetch.
        set_timeout_response_in_queue(&mut data_stream, 0);
        let mut new_global_data_summary = global_data_summary.clone();
        let new_highest_synced_version = MAX_ADVERTISED_TRANSACTION + 1000;
        new_global_data_summary.advertised_data.synced_ledger_infos = vec![create_ledger_info(
            new_highest_synced_version,
            MAX_ADVERTISED_EPOCH_END,
            false,
        )];
        process_data_responses(&mut data_stream, &new_global_data_summary).await;

        // Verify multiple data requests have now been sent to fetch the missing data
        let (sent_requests, _) = data_stream.get_sent_requests_and_notifications();
        assert_eq!(sent_requests.as_ref().unwrap().len(), 3);
        for i in 0..3 {
            let client_request = get_pending_client_request(&mut data_stream, i);
            let expected_version = MAX_ADVERTISED_TRANSACTION + 2 + i as u64;
            let expected_request = if allow_transactions_or_outputs {
                DataClientRequest::TransactionsOrOutputsWithProof(
                    TransactionsOrOutputsWithProofRequest {
                        start_version: expected_version,
                        end_version: expected_version,
                        proof_version: new_highest_synced_version,
                        include_events: false,
                    },
                )
            } else if transactions_only {
                DataClientRequest::TransactionsWithProof(TransactionsWithProofRequest {
                    start_version: expected_version,
                    end_version: expected_version,
                    proof_version: new_highest_synced_version,
                    include_events: false,
                })
            } else {
                DataClientRequest::TransactionOutputsWithProof(TransactionOutputsWithProofRequest {
                    start_version: expected_version,
                    end_version: expected_version,
                    proof_version: new_highest_synced_version,
                })
            };
            assert_eq!(client_request, expected_request);
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_continuous_stream_optimistic_fetch_timeout() {
    // Create a test data client config
    let optimistic_fetch_timeout_ms = 2022;
    let data_client_config = AptosDataClientConfig {
        optimistic_fetch_timeout_ms,
        ..Default::default()
    };

    // Test both types of continuous data streams
    let (data_stream_1, stream_listener_1) = create_continuous_transaction_stream(
        data_client_config,
        DataStreamingServiceConfig::default(),
        MAX_ADVERTISED_TRANSACTION,
        MAX_ADVERTISED_EPOCH_END,
    );
    let (data_stream_2, stream_listener_2) = create_continuous_transaction_output_stream(
        data_client_config,
        DataStreamingServiceConfig::default(),
        MAX_ADVERTISED_TRANSACTION_OUTPUT,
        MAX_ADVERTISED_EPOCH_END,
    );
    let (data_stream_3, stream_listener_3) = create_continuous_transaction_or_output_stream(
        data_client_config,
        DataStreamingServiceConfig::default(),
        MAX_ADVERTISED_TRANSACTION_OUTPUT,
        MAX_ADVERTISED_EPOCH_END,
    );
    for (mut data_stream, mut stream_listener, transactions_only, allow_transactions_or_outputs) in [
        (data_stream_1, stream_listener_1, true, false),
        (data_stream_2, stream_listener_2, false, false),
        (data_stream_3, stream_listener_3, false, true),
    ] {
        // Initialize the data stream
        let global_data_summary = create_global_data_summary(1);
        initialize_data_requests(&mut data_stream, &global_data_summary);

        // Verify a single request is made
        let (sent_requests, _) = data_stream.get_sent_requests_and_notifications();
        assert_eq!(sent_requests.as_ref().unwrap().len(), 1);

        // Wait until a notification is sent. The mock data client
        // will verify the timeout.
        wait_for_notification_and_verify(
            &mut data_stream,
            &mut stream_listener,
            transactions_only,
            allow_transactions_or_outputs,
            true,
            &global_data_summary,
        )
        .await;

        // Handle multiple timeouts and retries because no new data is known
        // about, so the best we can do is send optimistic fetch requests.
        for _ in 0..3 {
            set_timeout_response_in_queue(&mut data_stream, 0);
            process_data_responses(&mut data_stream, &global_data_summary).await;
        }

        // Wait until a notification is sent. The mock data client
        // will verify the timeout.
        wait_for_notification_and_verify(
            &mut data_stream,
            &mut stream_listener,
            transactions_only,
            allow_transactions_or_outputs,
            true,
            &global_data_summary,
        )
        .await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_transactions_and_output_stream_timeout() {
    // Create a test data client config
    let max_response_timeout_ms = 85;
    let response_timeout_ms = 7;
    let data_client_config = AptosDataClientConfig {
        max_response_timeout_ms,
        response_timeout_ms,
        ..Default::default()
    };

    // Create a test streaming service config
    let max_concurrent_requests = 3;
    let max_request_retry = 10;
    let streaming_service_config = DataStreamingServiceConfig {
        max_concurrent_requests,
        max_request_retry,
        ..Default::default()
    };

    // Test all types of data streams
    let (data_stream_1, stream_listener_1) = create_transaction_stream(
        data_client_config,
        streaming_service_config,
        MIN_ADVERTISED_TRANSACTION,
        MAX_ADVERTISED_TRANSACTION,
    );
    let (data_stream_2, stream_listener_2) = create_output_stream(
        data_client_config,
        streaming_service_config,
        MIN_ADVERTISED_TRANSACTION_OUTPUT,
        MAX_ADVERTISED_TRANSACTION_OUTPUT,
    );
    let (data_stream_3, stream_listener_3) = create_transactions_or_output_stream(
        data_client_config,
        streaming_service_config,
        MIN_ADVERTISED_TRANSACTION_OUTPUT,
        MAX_ADVERTISED_TRANSACTION_OUTPUT,
    );
    for (mut data_stream, mut stream_listener, transactions_only, allow_transactions_or_outputs) in [
        (data_stream_1, stream_listener_1, true, false),
        (data_stream_2, stream_listener_2, false, false),
        (data_stream_3, stream_listener_3, false, true),
    ] {
        // Initialize the data stream
        let global_data_summary = create_global_data_summary(1);
        initialize_data_requests(&mut data_stream, &global_data_summary);

        // Verify the correct number of requests are made
        let (sent_requests, _) = data_stream.get_sent_requests_and_notifications();
        assert_eq!(
            sent_requests.as_ref().unwrap().len(),
            max_concurrent_requests as usize
        );

        // Wait for the data client to satisfy all requests
        for i in 0..max_concurrent_requests as usize {
            wait_for_data_client_to_respond(&mut data_stream, i).await;
        }

        // Handle multiple timeouts and retries on the first request
        for _ in 0..max_request_retry / 2 {
            set_timeout_response_in_queue(&mut data_stream, 0);
            process_data_responses(&mut data_stream, &global_data_summary).await;
            wait_for_data_client_to_respond(&mut data_stream, 0).await;
        }

        // Wait until a notification is finally sent along the stream
        wait_for_notification_and_verify(
            &mut data_stream,
            &mut stream_listener,
            transactions_only,
            allow_transactions_or_outputs,
            false,
            &global_data_summary,
        )
        .await;

        // Wait for the data client to satisfy all requests
        for i in 0..max_concurrent_requests as usize {
            wait_for_data_client_to_respond(&mut data_stream, i).await;
        }

        // Set a timeout on the second request
        set_timeout_response_in_queue(&mut data_stream, 1);

        // Handle multiple invalid type responses on the first request
        for _ in 0..max_request_retry / 2 {
            set_state_value_response_in_queue(&mut data_stream, 0);
            process_data_responses(&mut data_stream, &global_data_summary).await;
            wait_for_data_client_to_respond(&mut data_stream, 0).await;
        }

        // Handle multiple invalid type responses on the third request
        for _ in 0..max_request_retry / 2 {
            set_state_value_response_in_queue(&mut data_stream, 2);
            process_data_responses(&mut data_stream, &global_data_summary).await;
            wait_for_data_client_to_respond(&mut data_stream, 2).await;
        }

        // Wait until a notification is finally sent along the stream
        wait_for_notification_and_verify(
            &mut data_stream,
            &mut stream_listener,
            transactions_only,
            allow_transactions_or_outputs,
            false,
            &global_data_summary,
        )
        .await;
    }
}

#[tokio::test]
async fn test_stream_listener_dropped() {
    // Create an epoch ending data stream
    let max_concurrent_requests = 3;
    let streaming_service_config = DataStreamingServiceConfig {
        max_concurrent_requests,
        ..Default::default()
    };
    let (mut data_stream, mut stream_listener) = create_epoch_ending_stream(
        AptosDataClientConfig::default(),
        streaming_service_config,
        MIN_ADVERTISED_EPOCH_END,
    );

    // Initialize the data stream
    let global_data_summary = create_global_data_summary(1);
    initialize_data_requests(&mut data_stream, &global_data_summary);

    // Verify no notifications have been sent yet
    let (sent_requests, sent_notifications) = data_stream.get_sent_requests_and_notifications();
    assert_ge!(
        sent_requests.as_ref().unwrap().len(),
        max_concurrent_requests as usize
    );
    assert_eq!(sent_notifications.len(), 0);

    // Set a response for the first request and verify a notification is sent
    set_epoch_ending_response_in_queue(&mut data_stream, 0, 0);
    process_data_responses(&mut data_stream, &global_data_summary).await;
    verify_epoch_ending_notification(
        &mut stream_listener,
        create_ledger_info(0, MIN_ADVERTISED_EPOCH_END, true),
    )
    .await;

    // Verify a single notification was sent
    let (_, sent_notifications) = data_stream.get_sent_requests_and_notifications();
    assert_eq!(sent_notifications.len(), 1);

    // Drop the listener
    drop(stream_listener);

    // Set a response for the first request and verify an error is returned
    // when the notification is sent.
    set_epoch_ending_response_in_queue(&mut data_stream, 0, 0);
    data_stream
        .process_data_responses(global_data_summary.clone())
        .await
        .unwrap_err();
    let (_, sent_notifications) = data_stream.get_sent_requests_and_notifications();
    assert_eq!(sent_notifications.len(), 2);

    // Set a response for the first request and verify no notifications are sent
    set_epoch_ending_response_in_queue(&mut data_stream, 0, 0);
    process_data_responses(&mut data_stream, &global_data_summary).await;
    let (_, sent_notifications) = data_stream.get_sent_requests_and_notifications();
    assert_eq!(sent_notifications.len(), 2);
}

/// Creates a state value stream for the given `version`.
fn create_state_value_stream(
    data_client_config: AptosDataClientConfig,
    streaming_service_config: DataStreamingServiceConfig,
    version: Version,
) -> (DataStream<MockAptosDataClient>, DataStreamListener) {
    // Create a state value stream request
    let stream_request = StreamRequest::GetAllStates(GetAllStatesRequest {
        version,
        start_index: 0,
    });
    create_data_stream(data_client_config, streaming_service_config, stream_request)
}

/// Creates an epoch ending stream starting at `start_epoch`
fn create_epoch_ending_stream(
    data_client_config: AptosDataClientConfig,
    streaming_service_config: DataStreamingServiceConfig,
    start_epoch: u64,
) -> (DataStream<MockAptosDataClient>, DataStreamListener) {
    // Create an epoch ending stream request
    let stream_request =
        StreamRequest::GetAllEpochEndingLedgerInfos(GetAllEpochEndingLedgerInfosRequest {
            start_epoch,
        });
    create_data_stream(data_client_config, streaming_service_config, stream_request)
}

/// Creates a continuous transaction output stream for the given `version`.
fn create_continuous_transaction_output_stream(
    data_client_config: AptosDataClientConfig,
    streaming_service_config: DataStreamingServiceConfig,
    known_version: Version,
    known_epoch: Version,
) -> (DataStream<MockAptosDataClient>, DataStreamListener) {
    // Create a continuous transaction output stream request
    let stream_request = StreamRequest::ContinuouslyStreamTransactionOutputs(
        ContinuouslyStreamTransactionOutputsRequest {
            known_version,
            known_epoch,
            target: None,
        },
    );
    create_data_stream(data_client_config, streaming_service_config, stream_request)
}

/// Creates a continuous transaction stream for the given `version`.
fn create_continuous_transaction_stream(
    data_client_config: AptosDataClientConfig,
    streaming_service_config: DataStreamingServiceConfig,
    known_version: Version,
    known_epoch: Version,
) -> (DataStream<MockAptosDataClient>, DataStreamListener) {
    // Create a continuous transaction stream request
    let stream_request =
        StreamRequest::ContinuouslyStreamTransactions(ContinuouslyStreamTransactionsRequest {
            known_version,
            known_epoch,
            include_events: false,
            target: None,
        });
    create_data_stream(data_client_config, streaming_service_config, stream_request)
}

/// Creates a continuous transaction or output stream for the given `version`.
fn create_continuous_transaction_or_output_stream(
    data_client_config: AptosDataClientConfig,
    streaming_service_config: DataStreamingServiceConfig,
    known_version: Version,
    known_epoch: Version,
) -> (DataStream<MockAptosDataClient>, DataStreamListener) {
    // Create a continuous transaction stream request
    let stream_request = StreamRequest::ContinuouslyStreamTransactionsOrOutputs(
        ContinuouslyStreamTransactionsOrOutputsRequest {
            known_version,
            known_epoch,
            include_events: false,
            target: None,
        },
    );
    create_data_stream(data_client_config, streaming_service_config, stream_request)
}

/// Creates a transaction stream for the given `version`.
fn create_transaction_stream(
    data_client_config: AptosDataClientConfig,
    streaming_service_config: DataStreamingServiceConfig,
    start_version: Version,
    end_version: Version,
) -> (DataStream<MockAptosDataClient>, DataStreamListener) {
    // Create a transaction stream request
    let stream_request = StreamRequest::GetAllTransactions(GetAllTransactionsRequest {
        start_version,
        end_version,
        proof_version: end_version,
        include_events: false,
    });
    create_data_stream(data_client_config, streaming_service_config, stream_request)
}

/// Creates an output stream for the given `version`.
fn create_output_stream(
    data_client_config: AptosDataClientConfig,
    streaming_service_config: DataStreamingServiceConfig,
    start_version: Version,
    end_version: Version,
) -> (DataStream<MockAptosDataClient>, DataStreamListener) {
    // Create an output stream request
    let stream_request = StreamRequest::GetAllTransactionOutputs(GetAllTransactionOutputsRequest {
        start_version,
        end_version,
        proof_version: end_version,
    });
    create_data_stream(data_client_config, streaming_service_config, stream_request)
}

/// Creates an output stream for the given `version`.
fn create_transactions_or_output_stream(
    data_client_config: AptosDataClientConfig,
    streaming_service_config: DataStreamingServiceConfig,
    start_version: Version,
    end_version: Version,
) -> (DataStream<MockAptosDataClient>, DataStreamListener) {
    // Create a transaction or output stream request
    let stream_request =
        StreamRequest::GetAllTransactionsOrOutputs(GetAllTransactionsOrOutputsRequest {
            start_version,
            end_version,
            proof_version: end_version,
            include_events: false,
        });
    create_data_stream(data_client_config, streaming_service_config, stream_request)
}

fn create_data_stream(
    data_client_config: AptosDataClientConfig,
    streaming_service_config: DataStreamingServiceConfig,
    stream_request: StreamRequest,
) -> (DataStream<MockAptosDataClient>, DataStreamListener) {
    initialize_logger();

    // Create an advertised data
    let advertised_data = create_advertised_data();

    // Create an aptos data client mock and notification generator
    let aptos_data_client = MockAptosDataClient::new(data_client_config, true, false, true, false);
    let notification_generator = Arc::new(U64IdGenerator::new());

    // Return the data stream and listener pair
    DataStream::new(
        data_client_config,
        streaming_service_config,
        create_random_u64(10000),
        &stream_request,
        aptos_data_client,
        notification_generator,
        &advertised_data,
    )
    .unwrap()
}

fn create_advertised_data() -> AdvertisedData {
    AdvertisedData {
        states: vec![CompleteDataRange::new(MIN_ADVERTISED_STATES, MAX_ADVERTISED_STATES).unwrap()],
        epoch_ending_ledger_infos: vec![CompleteDataRange::new(
            MIN_ADVERTISED_EPOCH_END,
            MAX_ADVERTISED_EPOCH_END,
        )
        .unwrap()],
        synced_ledger_infos: vec![create_ledger_info(
            MAX_ADVERTISED_TRANSACTION,
            MAX_ADVERTISED_EPOCH_END,
            true,
        )],
        transactions: vec![CompleteDataRange::new(
            MIN_ADVERTISED_TRANSACTION,
            MAX_ADVERTISED_TRANSACTION,
        )
        .unwrap()],
        transaction_outputs: vec![CompleteDataRange::new(
            MIN_ADVERTISED_TRANSACTION_OUTPUT,
            MAX_ADVERTISED_TRANSACTION_OUTPUT,
        )
        .unwrap()],
    }
}

fn create_global_data_summary(chunk_sizes: u64) -> GlobalDataSummary {
    let mut global_data_summary = GlobalDataSummary::empty();
    global_data_summary.optimal_chunk_sizes = create_optimal_chunk_sizes(chunk_sizes);
    global_data_summary.advertised_data = create_advertised_data();
    global_data_summary
}

fn create_optimal_chunk_sizes(chunk_sizes: u64) -> OptimalChunkSizes {
    OptimalChunkSizes {
        state_chunk_size: chunk_sizes,
        epoch_chunk_size: chunk_sizes,
        transaction_chunk_size: chunk_sizes,
        transaction_output_chunk_size: chunk_sizes,
    }
}

/// Sets the client response at the index in the pending queue to contain an
/// epoch ending data response.
fn set_epoch_ending_response_in_queue(
    data_stream: &mut DataStream<MockAptosDataClient>,
    index: usize,
    version: u64,
) {
    let (sent_requests, _) = data_stream.get_sent_requests_and_notifications();
    let pending_response = sent_requests.as_mut().unwrap().get_mut(index).unwrap();
    let client_response = Some(Ok(create_data_client_response(
        ResponsePayload::EpochEndingLedgerInfos(vec![create_ledger_info(
            version,
            MIN_ADVERTISED_EPOCH_END,
            true,
        )]),
    )));
    pending_response.lock().client_response = client_response;
}

/// Sets the client response at the index in the pending queue to contain a
/// number of state values response.
fn set_num_state_values_response_in_queue(
    data_stream: &mut DataStream<MockAptosDataClient>,
    index: usize,
) {
    let (sent_requests, _) = data_stream.get_sent_requests_and_notifications();
    let pending_response = sent_requests.as_mut().unwrap().get_mut(index).unwrap();
    let client_response = Some(Ok(create_data_client_response(
        ResponsePayload::NumberOfStates(1000000),
    )));
    pending_response.lock().client_response = client_response;
}

/// Sets the client response at the index in the pending queue to contain an
/// state value data response.
fn set_state_value_response_in_queue(
    data_stream: &mut DataStream<MockAptosDataClient>,
    index: usize,
) {
    let (sent_requests, _) = data_stream.get_sent_requests_and_notifications();
    let pending_response = sent_requests.as_mut().unwrap().get_mut(index).unwrap();
    let client_response = Some(Ok(create_data_client_response(
        ResponsePayload::StateValuesWithProof(StateValueChunkWithProof {
            first_index: 0,
            last_index: 0,
            first_key: Default::default(),
            last_key: Default::default(),
            raw_values: vec![],
            proof: SparseMerkleRangeProof::new(vec![]),
            root_hash: Default::default(),
        }),
    )));
    pending_response.lock().client_response = client_response;
}

/// Sets the client response at the index in the pending queue to contain
/// an optimistic fetch response.
fn set_optimistic_fetch_response_in_queue(
    data_stream: &mut DataStream<MockAptosDataClient>,
    index: usize,
    single_data_version: u64,
    transactions_only: bool,
) {
    let (sent_requests, _) = data_stream.get_sent_requests_and_notifications();
    let pending_response = sent_requests.as_mut().unwrap().get_mut(index).unwrap();
    let client_response = if transactions_only {
        Some(Ok(create_data_client_response(
            ResponsePayload::NewTransactionsWithProof((
                create_transaction_list_with_proof(single_data_version, single_data_version, false),
                create_ledger_info(single_data_version, MAX_ADVERTISED_EPOCH_END, false),
            )),
        )))
    } else {
        Some(Ok(create_data_client_response(
            ResponsePayload::NewTransactionOutputsWithProof((
                create_output_list_with_proof(single_data_version, single_data_version),
                create_ledger_info(single_data_version, MAX_ADVERTISED_EPOCH_END, false),
            )),
        )))
    };
    pending_response.lock().client_response = client_response;
}

/// Sets the client response at the index in the pending queue to contain a
/// timeout response.
fn set_timeout_response_in_queue(data_stream: &mut DataStream<MockAptosDataClient>, index: usize) {
    let (sent_requests, _) = data_stream.get_sent_requests_and_notifications();
    let pending_response = sent_requests.as_mut().unwrap().get_mut(index).unwrap();
    let client_response = Some(Err(
        aptos_data_client::error::Error::TimeoutWaitingForResponse("Timed out!".into()),
    ));
    pending_response.lock().client_response = client_response;
}

/// Waits for the data client to set the response at the index in the
/// pending queue.
async fn wait_for_data_client_to_respond(
    data_stream: &mut DataStream<MockAptosDataClient>,
    index: usize,
) {
    let (sent_requests, _) = data_stream.get_sent_requests_and_notifications();
    let pending_response = sent_requests.as_mut().unwrap().get_mut(index).unwrap();

    loop {
        if let Some(client_response) = &pending_response.lock().client_response {
            if !matches!(
                client_response,
                Err(aptos_data_client::error::Error::TimeoutWaitingForResponse(
                    _
                ))
            ) {
                return;
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// Sets the client response at the head of the pending queue to contain an
/// transaction response.
fn set_transaction_response_at_queue_head(data_stream: &mut DataStream<MockAptosDataClient>) {
    let (sent_requests, _) = data_stream.get_sent_requests_and_notifications();
    if !sent_requests.as_mut().unwrap().is_empty() {
        let pending_response = sent_requests.as_mut().unwrap().get_mut(0).unwrap();
        let client_response = Some(Ok(create_data_client_response(
            ResponsePayload::TransactionsWithProof(create_transaction_list_with_proof(0, 0, false)),
        )));
        pending_response.lock().client_response = client_response;
    }
}

/// Clears the pending queue of the given data stream and inserts a single
/// response into the head of the queue.
fn insert_response_into_pending_queue(
    data_stream: &mut DataStream<MockAptosDataClient>,
    pending_response: PendingClientResponse,
) {
    // Clear the queue
    let (sent_requests, _) = data_stream.get_sent_requests_and_notifications();
    sent_requests.as_mut().unwrap().clear();

    // Insert the pending response
    let pending_response = Arc::new(Mutex::new(Box::new(pending_response)));
    sent_requests.as_mut().unwrap().push_front(pending_response);
}

/// Verifies that a client request was resubmitted (i.e., pushed to the head of the
/// sent request queue)
fn verify_client_request_resubmitted(
    data_stream: &mut DataStream<MockAptosDataClient>,
    client_request: DataClientRequest,
) {
    let (sent_requests, _) = data_stream.get_sent_requests_and_notifications();
    let pending_response = sent_requests.as_mut().unwrap().pop_front().unwrap();
    assert_eq!(pending_response.lock().client_request, client_request);
    assert_none!(pending_response.lock().client_response.as_ref());
}

/// Verifies that a single epoch ending notification is received by the
/// data listener and that it contains the `expected_ledger_info`.
async fn verify_epoch_ending_notification(
    stream_listener: &mut DataStreamListener,
    expected_ledger_info: LedgerInfoWithSignatures,
) {
    let data_notification = get_data_notification(stream_listener).await.unwrap();
    if let DataPayload::EpochEndingLedgerInfos(ledger_infos) = data_notification.data_payload {
        assert_eq!(ledger_infos[0], expected_ledger_info);
    } else {
        panic!(
            "Expected an epoch ending ledger info payload, but got: {:?}",
            data_notification
        );
    }
}

/// Helper function to initialize the data requests
fn initialize_data_requests(
    data_stream: &mut DataStream<MockAptosDataClient>,
    global_data_summary: &GlobalDataSummary,
) {
    data_stream
        .initialize_data_requests(global_data_summary.clone())
        .unwrap();
}

/// Helper function to process data responses on the given data stream
async fn process_data_responses(
    data_stream: &mut DataStream<MockAptosDataClient>,
    global_data_summary: &GlobalDataSummary,
) {
    data_stream
        .process_data_responses(global_data_summary.clone())
        .await
        .unwrap();
}

/// Helper function to get the pending client request at
/// the specified index.
fn get_pending_client_request(
    data_stream: &mut DataStream<MockAptosDataClient>,
    index: usize,
) -> DataClientRequest {
    let (sent_requests, _) = data_stream.get_sent_requests_and_notifications();
    let pending_response = sent_requests.as_ref().unwrap().get(index).unwrap();
    let client_request = pending_response.lock().client_request.clone();
    client_request
}

/// Waits for an optimistic fetch notification along the given
/// listener and continues to drive progress until one is received.
/// Verifies the notification when it is received.
async fn wait_for_notification_and_verify(
    data_stream: &mut DataStream<MockAptosDataClient>,
    stream_listener: &mut DataStreamListener,
    transaction_syncing: bool,
    allow_transactions_or_outputs: bool,
    optimistic_fetch_notification: bool,
    global_data_summary: &GlobalDataSummary,
) {
    loop {
        if let Ok(data_notification) =
            timeout(Duration::from_secs(1), stream_listener.select_next_some()).await
        {
            if optimistic_fetch_notification {
                // Verify we got the correct optimistic fetch data
                match data_notification.data_payload {
                    DataPayload::ContinuousTransactionsWithProof(..) => {
                        assert!(allow_transactions_or_outputs || transaction_syncing);
                    },
                    DataPayload::ContinuousTransactionOutputsWithProof(..) => {
                        assert!(allow_transactions_or_outputs || !transaction_syncing);
                    },
                    _ => {
                        panic!(
                            "Invalid data notification found: {:?}",
                            data_notification.data_payload
                        );
                    },
                }
            } else {
                // Verify we got the correct transaction data
                match data_notification.data_payload {
                    DataPayload::TransactionsWithProof(..) => {
                        assert!(allow_transactions_or_outputs || transaction_syncing);
                    },
                    DataPayload::TransactionOutputsWithProof(..) => {
                        assert!(allow_transactions_or_outputs || !transaction_syncing);
                    },
                    _ => {
                        panic!(
                            "Invalid data notification found: {:?}",
                            data_notification.data_payload
                        );
                    },
                }
            }
            break;
        } else {
            process_data_responses(data_stream, global_data_summary).await;
        }
    }
}

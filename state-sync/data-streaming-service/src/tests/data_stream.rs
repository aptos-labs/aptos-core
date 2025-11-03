// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    data_notification::{
        DataClientRequest, DataPayload, EpochEndingLedgerInfosRequest,
        NewTransactionOutputsWithProofRequest, NewTransactionsOrOutputsWithProofRequest,
        NewTransactionsWithProofRequest, PendingClientResponse,
        SubscribeTransactionOutputsWithProofRequest,
        SubscribeTransactionsOrOutputsWithProofRequest, SubscribeTransactionsWithProofRequest,
        TransactionOutputsWithProofRequest, TransactionsOrOutputsWithProofRequest,
        TransactionsWithProofRequest,
    },
    data_stream::{DataStream, DataStreamListener},
    streaming_client::{
        ContinuouslyStreamTransactionOutputsRequest,
        ContinuouslyStreamTransactionsOrOutputsRequest, ContinuouslyStreamTransactionsRequest,
        GetAllEpochEndingLedgerInfosRequest, GetAllStatesRequest, GetAllTransactionOutputsRequest,
        GetAllTransactionsOrOutputsRequest, GetAllTransactionsRequest, NotificationFeedback,
        StreamRequest,
    },
    streaming_service::StreamUpdateNotification,
    tests::utils::{
        create_data_client_response, create_ledger_info, create_output_list_with_proof,
        create_random_u64, create_transaction_list_with_proof, get_data_notification,
        initialize_logger, MockAptosDataClient, NoopResponseCallback, MAX_ADVERTISED_EPOCH_END,
        MAX_ADVERTISED_STATES, MAX_ADVERTISED_TRANSACTION, MAX_ADVERTISED_TRANSACTION_OUTPUT,
        MAX_NOTIFICATION_TIMEOUT_SECS, MIN_ADVERTISED_EPOCH_END, MIN_ADVERTISED_STATES,
        MIN_ADVERTISED_TRANSACTION, MIN_ADVERTISED_TRANSACTION_OUTPUT,
    },
};
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_config::config::{
    AptosDataClientConfig, DataStreamingServiceConfig, DynamicPrefetchingConfig,
};
use aptos_data_client::{
    global_summary::{AdvertisedData, GlobalDataSummary, OptimalChunkSizes},
    interface::{Response, ResponseContext, ResponsePayload},
};
use aptos_id_generator::U64IdGenerator;
use aptos_infallible::Mutex;
use aptos_storage_service_types::responses::CompleteDataRange;
use aptos_time_service::{TimeService, TimeServiceTrait};
use aptos_types::{
    ledger_info::LedgerInfoWithSignatures,
    proof::SparseMerkleRangeProof,
    state_store::{
        state_key::StateKey,
        state_value::{StateValue, StateValueChunkWithProof},
    },
    transaction::Version,
};
use claims::{assert_err, assert_ge, assert_matches, assert_none, assert_ok, assert_some};
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
        let context = ResponseContext::new(0, Box::new(NoopResponseCallback));
        let pending_response = PendingClientResponse::new_with_response(
            client_request.clone(),
            Ok(Response::new(context, ResponsePayload::NumberOfStates(10))),
        );
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
    let pending_response = PendingClientResponse::new_with_response(
        client_request.clone(),
        Err(aptos_data_client::error::Error::DataIsUnavailable(
            "Missing data!".into(),
        )),
    );
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
    let context = ResponseContext::new(0, Box::new(NoopResponseCallback));
    let pending_response = PendingClientResponse::new_with_response(
        client_request.clone(),
        Ok(Response::new(context, ResponsePayload::NumberOfStates(10))),
    );
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

    // Verify that three requests have been made
    verify_num_sent_requests(&mut data_stream, max_concurrent_requests);

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
    // Create a state value data stream with dynamic prefetching disabled
    let max_concurrent_state_requests = 6;
    let dynamic_prefetching_config = DynamicPrefetchingConfig {
        enable_dynamic_prefetching: false,
        ..Default::default()
    };
    let streaming_service_config = DataStreamingServiceConfig {
        dynamic_prefetching: dynamic_prefetching_config,
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
    verify_num_sent_requests(&mut data_stream, 1);

    // Set a response for the number of state values
    set_num_state_values_response_in_queue(&mut data_stream, 0);
    process_data_responses(&mut data_stream, &global_data_summary).await;

    // Verify the number of sent requests
    verify_num_sent_requests(&mut data_stream, max_concurrent_state_requests);

    // Set a response for the second request and verify no notifications
    set_state_value_response_in_queue(&mut data_stream, 1, 1, 1);
    process_data_responses(&mut data_stream, &global_data_summary).await;
    assert_none!(stream_listener.select_next_some().now_or_never());

    // Set a response for the first request and verify two notifications
    set_state_value_response_in_queue(&mut data_stream, 0, 0, 0);
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
    set_state_value_response_in_queue(&mut data_stream, 2, 2, 0);
    set_state_value_response_in_queue(&mut data_stream, 4, 4, 2);
    process_data_responses(&mut data_stream, &global_data_summary).await;
    let data_notification = get_data_notification(&mut stream_listener).await.unwrap();
    assert_matches!(
        data_notification.data_payload,
        DataPayload::StateValuesWithProof(_)
    );
    assert_none!(stream_listener.select_next_some().now_or_never());

    // Set the response for the first and third request and verify three notifications sent
    set_state_value_response_in_queue(&mut data_stream, 3, 3, 0);
    set_state_value_response_in_queue(&mut data_stream, 5, 5, 2);
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
async fn test_state_stream_out_of_order_responses_dynamic() {
    // Create a dynamic prefetching config with prefetching enabled
    let initial_prefetching_value = 3;
    let prefetching_value_increase = 2;
    let dynamic_prefetching_config = DynamicPrefetchingConfig {
        enable_dynamic_prefetching: true,
        initial_prefetching_value,
        prefetching_value_increase,
        ..Default::default()
    };

    // Create a state value data stream
    let streaming_service_config = DataStreamingServiceConfig {
        dynamic_prefetching: dynamic_prefetching_config,
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
    verify_num_sent_requests(&mut data_stream, 1);

    // Set a response for the number of state values
    set_num_state_values_response_in_queue(&mut data_stream, 0);
    process_data_responses(&mut data_stream, &global_data_summary).await;

    // Verify the correct number of requests have been made
    verify_num_sent_requests(
        &mut data_stream,
        initial_prefetching_value + prefetching_value_increase,
    );

    // Set a response for the second request and verify no notifications
    set_state_value_response_in_queue(&mut data_stream, 1, 1, 1);
    process_data_responses(&mut data_stream, &global_data_summary).await;
    assert_none!(stream_listener.select_next_some().now_or_never());

    // Verify the correct number of requests have been made
    verify_num_sent_requests(
        &mut data_stream,
        initial_prefetching_value + prefetching_value_increase + 1,
    );

    // Set a response for the first request and verify two notifications
    set_state_value_response_in_queue(&mut data_stream, 0, 0, 0);
    process_data_responses(&mut data_stream, &global_data_summary).await;
    for _ in 0..2 {
        let data_notification = get_data_notification(&mut stream_listener).await.unwrap();
        assert_matches!(
            data_notification.data_payload,
            DataPayload::StateValuesWithProof(_)
        );
    }
    assert_none!(stream_listener.select_next_some().now_or_never());

    // Verify the correct number of requests have been made
    verify_num_sent_requests(
        &mut data_stream,
        initial_prefetching_value + (prefetching_value_increase * 3),
    );

    // Set the response for the first and third request and verify one notification sent
    set_state_value_response_in_queue(&mut data_stream, 2, 2, 0);
    set_state_value_response_in_queue(&mut data_stream, 4, 4, 2);
    process_data_responses(&mut data_stream, &global_data_summary).await;
    let data_notification = get_data_notification(&mut stream_listener).await.unwrap();
    assert_matches!(
        data_notification.data_payload,
        DataPayload::StateValuesWithProof(_)
    );
    assert_none!(stream_listener.select_next_some().now_or_never());

    // Verify the correct number of requests have been made
    verify_num_sent_requests(
        &mut data_stream,
        initial_prefetching_value + (prefetching_value_increase * 4) + 1,
    );

    // Set the response for the first and third request and verify three notifications sent
    set_state_value_response_in_queue(&mut data_stream, 3, 3, 0);
    set_state_value_response_in_queue(&mut data_stream, 5, 5, 2);
    process_data_responses(&mut data_stream, &global_data_summary).await;
    for _ in 0..3 {
        let data_notification = get_data_notification(&mut stream_listener).await.unwrap();
        assert_matches!(
            data_notification.data_payload,
            DataPayload::StateValuesWithProof(_)
        );
    }
    assert_none!(stream_listener.select_next_some().now_or_never());

    // Verify the correct number of requests have been made
    verify_num_sent_requests(
        &mut data_stream,
        initial_prefetching_value + (prefetching_value_increase * 7),
    );
}

#[tokio::test]
async fn test_stream_max_pending_requests() {
    // Create an epoch ending data stream with dynamic prefetching disabled
    let max_concurrent_requests = 6;
    let max_pending_requests = 19;
    let dynamic_prefetching_config = DynamicPrefetchingConfig {
        enable_dynamic_prefetching: false,
        ..Default::default()
    };
    let streaming_service_config = DataStreamingServiceConfig {
        dynamic_prefetching: dynamic_prefetching_config,
        max_concurrent_requests,
        max_pending_requests,
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

    // Verify that the correct number of client requests have been made
    verify_num_sent_requests(&mut data_stream, max_concurrent_requests);

    // Set a valid response for each request except the first one
    set_epoch_ending_response_for_indices(
        &mut data_stream,
        max_concurrent_requests,
        (1..max_concurrent_requests).collect::<Vec<_>>(),
    );

    // Process the responses and send more client requests
    process_data_responses(&mut data_stream, &global_data_summary).await;
    assert_none!(stream_listener.select_next_some().now_or_never());

    // Verify the correct number of requests have been made
    let num_expected_pending_requests = (max_concurrent_requests * 2) - 1; // The first request failed
    verify_num_sent_requests(&mut data_stream, num_expected_pending_requests);

    // Verify the state of the pending responses
    verify_pending_responses_for_indices(
        &mut data_stream,
        num_expected_pending_requests,
        (1..max_concurrent_requests).collect::<Vec<_>>(),
    );

    // Set a valid response for each request except the first and last ones
    set_epoch_ending_response_for_indices(
        &mut data_stream,
        num_expected_pending_requests,
        (1..num_expected_pending_requests - 1).collect::<Vec<_>>(),
    );

    // Process the responses and send more client requests
    process_data_responses(&mut data_stream, &global_data_summary).await;
    assert_none!(stream_listener.select_next_some().now_or_never());

    // Verify the correct number of requests have been made
    let num_expected_pending_requests = (max_concurrent_requests * 3) - 3;
    verify_num_sent_requests(&mut data_stream, num_expected_pending_requests);

    // Verify the state of the pending responses
    verify_pending_responses_for_indices(
        &mut data_stream,
        num_expected_pending_requests,
        (1..(max_concurrent_requests * 2) - 2).collect::<Vec<_>>(),
    );

    // Set a valid response for each request except the first one
    set_epoch_ending_response_for_indices(
        &mut data_stream,
        num_expected_pending_requests,
        (1..num_expected_pending_requests).collect::<Vec<_>>(),
    );

    // Process the responses and send more client requests
    process_data_responses(&mut data_stream, &global_data_summary).await;
    assert_none!(stream_listener.select_next_some().now_or_never());

    // Verify the correct number of requests have been made
    verify_num_sent_requests(&mut data_stream, max_pending_requests);

    // Verify the state of the pending responses
    verify_pending_responses_for_indices(
        &mut data_stream,
        num_expected_pending_requests,
        (1..(max_concurrent_requests * 3) - 3).collect::<Vec<_>>(),
    );

    // Set a valid response for each request except the first one
    set_epoch_ending_response_for_indices(
        &mut data_stream,
        num_expected_pending_requests,
        (1..num_expected_pending_requests).collect::<Vec<_>>(),
    );

    // Process the responses and send more client requests several times
    for _ in 0..10 {
        // Process the responses and send more client requests
        process_data_responses(&mut data_stream, &global_data_summary).await;
        assert_none!(stream_listener.select_next_some().now_or_never());

        // Verify that no more requests have been made (we're at the max)
        verify_num_sent_requests(&mut data_stream, max_pending_requests);
    }

    // Set a valid response for every request
    set_epoch_ending_response_for_indices(
        &mut data_stream,
        max_pending_requests,
        (0..max_pending_requests).collect::<Vec<_>>(),
    );

    // Process the responses and send more client requests
    process_data_responses(&mut data_stream, &global_data_summary).await;

    // Verify that more requests have been made (and the entire buffer has been flushed)
    verify_num_sent_requests(&mut data_stream, max_concurrent_requests);

    // Verify that we received a notification for each flushed response
    for _ in 0..max_pending_requests {
        let data_notification = get_data_notification(&mut stream_listener).await.unwrap();
        assert_matches!(
            data_notification.data_payload,
            DataPayload::EpochEndingLedgerInfos(_)
        );
    }
}

#[tokio::test]
async fn test_stream_max_pending_requests_dynamic() {
    // Create a dynamic prefetching config with prefetching enabled
    let initial_prefetching_value = 5;
    let min_prefetching_value = 1;
    let prefetching_value_increase = 2;
    let prefetching_value_decrease = 3;
    let dynamic_prefetching_config = DynamicPrefetchingConfig {
        enable_dynamic_prefetching: true,
        initial_prefetching_value,
        min_prefetching_value,
        prefetching_value_increase,
        prefetching_value_decrease,
        timeout_freeze_duration_secs: 0, // Don't freeze the prefetching value
        ..Default::default()
    };

    // Create an epoch ending data stream
    let max_pending_requests = 6;
    let streaming_service_config = DataStreamingServiceConfig {
        dynamic_prefetching: dynamic_prefetching_config,
        max_pending_requests,
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

    // Verify that the correct number of client requests have been made
    verify_num_sent_requests(&mut data_stream, initial_prefetching_value);

    // Set a valid response for each request except the first one
    set_epoch_ending_response_for_indices(
        &mut data_stream,
        initial_prefetching_value,
        (1..initial_prefetching_value).collect::<Vec<_>>(),
    );

    // Process the responses and send more client requests
    process_data_responses(&mut data_stream, &global_data_summary).await;
    assert_none!(stream_listener.select_next_some().now_or_never());

    // Verify the correct number of requests have been made
    let num_expected_pending_requests =
        ((initial_prefetching_value * 2) - prefetching_value_decrease) - 1; // The first request failed
    verify_num_sent_requests(&mut data_stream, num_expected_pending_requests);

    // Verify the state of the pending responses
    verify_pending_responses_for_indices(
        &mut data_stream,
        num_expected_pending_requests,
        (1..initial_prefetching_value).collect::<Vec<_>>(),
    );

    // Set a valid response for each request except the first and last ones
    set_epoch_ending_response_for_indices(
        &mut data_stream,
        num_expected_pending_requests,
        (1..num_expected_pending_requests - 1).collect::<Vec<_>>(),
    );

    // Process the responses and send more client requests
    process_data_responses(&mut data_stream, &global_data_summary).await;
    assert_none!(stream_listener.select_next_some().now_or_never());

    // Verify the correct number of requests have been made
    let num_expected_pending_requests =
        ((initial_prefetching_value * 2) - prefetching_value_decrease) - 1; // The first request failed
    verify_num_sent_requests(&mut data_stream, num_expected_pending_requests);

    // Verify the state of the pending responses
    verify_pending_responses_for_indices(
        &mut data_stream,
        num_expected_pending_requests,
        (1..num_expected_pending_requests - 1).collect::<Vec<_>>(),
    );

    // Set a valid response for each request except the first one
    set_epoch_ending_response_for_indices(
        &mut data_stream,
        num_expected_pending_requests,
        (1..num_expected_pending_requests).collect::<Vec<_>>(),
    );

    // Process the responses and send more client requests
    process_data_responses(&mut data_stream, &global_data_summary).await;
    assert_none!(stream_listener.select_next_some().now_or_never());

    // Verify the correct number of requests have been made
    verify_num_sent_requests(&mut data_stream, max_pending_requests);

    // Verify the state of the pending responses
    verify_pending_responses_for_indices(
        &mut data_stream,
        num_expected_pending_requests,
        (1..num_expected_pending_requests).collect::<Vec<_>>(),
    );

    // Set a valid response for each request except the first one
    set_epoch_ending_response_for_indices(
        &mut data_stream,
        num_expected_pending_requests,
        (1..num_expected_pending_requests).collect::<Vec<_>>(),
    );

    // Process the responses and send more client requests several times
    for _ in 0..10 {
        // Process the responses and send more client requests
        process_data_responses(&mut data_stream, &global_data_summary).await;
        assert_none!(stream_listener.select_next_some().now_or_never());

        // Verify that no more requests have been made (we're at the max)
        verify_num_sent_requests(&mut data_stream, max_pending_requests);
    }

    // Set a valid response for every request
    set_epoch_ending_response_for_indices(
        &mut data_stream,
        max_pending_requests,
        (0..max_pending_requests).collect::<Vec<_>>(),
    );

    // Process the responses and send more client requests
    process_data_responses(&mut data_stream, &global_data_summary).await;

    // Verify that more requests have been made (and the entire buffer has been flushed)
    verify_num_sent_requests(&mut data_stream, max_pending_requests);

    // Verify that we received a notification for each flushed response
    for _ in 0..max_pending_requests {
        let data_notification = get_data_notification(&mut stream_listener).await.unwrap();
        assert_matches!(
            data_notification.data_payload,
            DataPayload::EpochEndingLedgerInfos(_)
        );
    }
}

#[tokio::test]
async fn test_stream_max_pending_requests_flushing() {
    // Create an epoch ending data stream with dynamic prefetching disabled
    let max_concurrent_requests = 2;
    let max_pending_requests = 4;
    let dynamic_prefetching_config = DynamicPrefetchingConfig {
        enable_dynamic_prefetching: false,
        ..Default::default()
    };
    let streaming_service_config = DataStreamingServiceConfig {
        dynamic_prefetching: dynamic_prefetching_config,
        max_concurrent_requests,
        max_pending_requests,
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

    // Verify that the correct number of client requests have been made
    verify_num_sent_requests(&mut data_stream, max_concurrent_requests);

    // Set a valid response for the second request
    set_epoch_ending_response_in_queue(&mut data_stream, 1, 0);

    // Process the responses and verify we get no notifications
    process_data_responses(&mut data_stream, &global_data_summary).await;
    assert_none!(stream_listener.select_next_some().now_or_never());

    // Verify that the correct number of client requests have been made
    verify_num_sent_requests(&mut data_stream, max_concurrent_requests + 1);

    // Set a valid response for the third request
    set_epoch_ending_response_in_queue(&mut data_stream, 2, 0);

    // Process the responses and send more client requests
    process_data_responses(&mut data_stream, &global_data_summary).await;
    assert_none!(stream_listener.select_next_some().now_or_never());

    // Verify that the correct number of client requests have been made
    verify_num_sent_requests(&mut data_stream, max_pending_requests);

    // Set a valid response for the first request
    set_epoch_ending_response_in_queue(&mut data_stream, 0, 0);

    // Process the responses and verify we get three notifications
    process_data_responses(&mut data_stream, &global_data_summary).await;
    for _ in 0..3 {
        assert_some!(stream_listener.select_next_some().now_or_never());
    }
    assert_none!(stream_listener.select_next_some().now_or_never());

    // Verify the correct number of client requests have been made
    verify_num_sent_requests(&mut data_stream, max_concurrent_requests);

    // Set a valid response for the second request
    set_epoch_ending_response_in_queue(&mut data_stream, 1, 0);

    // Process the responses and verify we get no notifications
    process_data_responses(&mut data_stream, &global_data_summary).await;
    assert_none!(stream_listener.select_next_some().now_or_never());

    // Verify the correct number of client requests have been made
    verify_num_sent_requests(&mut data_stream, max_concurrent_requests + 1);

    // Set a valid response for the first request
    set_epoch_ending_response_in_queue(&mut data_stream, 0, 0);

    // Process the responses and verify we get two notifications
    process_data_responses(&mut data_stream, &global_data_summary).await;
    for _ in 0..2 {
        assert_some!(stream_listener.select_next_some().now_or_never());
    }
    assert_none!(stream_listener.select_next_some().now_or_never());

    // Verify the correct number of client requests have been made
    verify_num_sent_requests(&mut data_stream, max_concurrent_requests);

    // Set an error response for all requests
    for index in 0..max_concurrent_requests {
        set_failure_response_in_queue(&mut data_stream, index as usize);
    }

    // Process the responses and (potentially) send more client requests
    process_data_responses(&mut data_stream, &global_data_summary).await;
    assert_none!(stream_listener.select_next_some().now_or_never());

    // Verify no more client requests have been made
    verify_num_sent_requests(&mut data_stream, max_concurrent_requests);

    // Set a valid response for the first request
    set_epoch_ending_response_in_queue(&mut data_stream, 0, 0);

    // Process the responses and verify we get one notification
    process_data_responses(&mut data_stream, &global_data_summary).await;
    assert_some!(stream_listener.select_next_some().now_or_never());
    assert_none!(stream_listener.select_next_some().now_or_never());

    // Verify no more client requests have been made
    verify_num_sent_requests(&mut data_stream, max_concurrent_requests);
}

#[tokio::test]
async fn test_stream_max_pending_requests_flushing_dynamic() {
    // Create a dynamic prefetching config with prefetching enabled
    let initial_prefetching_value = 3;
    let min_prefetching_value = 1;
    let prefetching_value_increase = 2;
    let dynamic_prefetching_config = DynamicPrefetchingConfig {
        enable_dynamic_prefetching: true,
        initial_prefetching_value,
        min_prefetching_value,
        prefetching_value_increase,
        timeout_freeze_duration_secs: 0, // Don't freeze the prefetching value
        ..Default::default()
    };

    // Create an epoch ending data stream
    let max_pending_requests = 7;
    let streaming_service_config = DataStreamingServiceConfig {
        dynamic_prefetching: dynamic_prefetching_config,
        max_pending_requests,
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

    // Verify that the correct number of client requests have been made
    verify_num_sent_requests(&mut data_stream, initial_prefetching_value);

    // Set a valid response for the second request
    set_epoch_ending_response_in_queue(&mut data_stream, 1, 0);

    // Process the responses and verify we get no notifications
    process_data_responses(&mut data_stream, &global_data_summary).await;
    assert_none!(stream_listener.select_next_some().now_or_never());

    // Verify that the correct number of client requests have been made
    verify_num_sent_requests(&mut data_stream, initial_prefetching_value + 1);

    // Set a valid response for the third request
    set_epoch_ending_response_in_queue(&mut data_stream, 2, 0);

    // Process the responses and send more client requests
    process_data_responses(&mut data_stream, &global_data_summary).await;
    assert_none!(stream_listener.select_next_some().now_or_never());

    // Verify that the correct number of client requests have been made
    verify_num_sent_requests(&mut data_stream, initial_prefetching_value + 2);

    // Set a valid response for the first request
    set_epoch_ending_response_in_queue(&mut data_stream, 0, 0);

    // Process the responses and verify we get three notifications
    process_data_responses(&mut data_stream, &global_data_summary).await;
    for _ in 0..3 {
        assert_some!(stream_listener.select_next_some().now_or_never());
    }
    assert_none!(stream_listener.select_next_some().now_or_never());

    // Verify the correct number of client requests have been made
    verify_num_sent_requests(&mut data_stream, max_pending_requests);

    // Set a valid response for the second request
    set_epoch_ending_response_in_queue(&mut data_stream, 1, 0);

    // Process the responses and verify we get no notifications
    process_data_responses(&mut data_stream, &global_data_summary).await;
    assert_none!(stream_listener.select_next_some().now_or_never());

    // Verify the correct number of client requests have been made
    verify_num_sent_requests(&mut data_stream, max_pending_requests);

    // Set a valid response for the first request
    set_epoch_ending_response_in_queue(&mut data_stream, 0, 0);

    // Process the responses and verify we get two notifications
    process_data_responses(&mut data_stream, &global_data_summary).await;
    for _ in 0..2 {
        assert_some!(stream_listener.select_next_some().now_or_never());
    }
    assert_none!(stream_listener.select_next_some().now_or_never());

    // Verify the correct number of client requests have been made
    verify_num_sent_requests(&mut data_stream, max_pending_requests);

    // Set an error response for all requests
    for index in 0..max_pending_requests {
        set_failure_response_in_queue(&mut data_stream, index as usize);
    }

    // Process the responses and (potentially) send more client requests
    process_data_responses(&mut data_stream, &global_data_summary).await;
    assert_none!(stream_listener.select_next_some().now_or_never());

    // Verify no more client requests have been made
    verify_num_sent_requests(&mut data_stream, max_pending_requests);

    // Set a valid response for the first request
    set_epoch_ending_response_in_queue(&mut data_stream, 0, 0);

    // Process the responses and verify we get one notification
    process_data_responses(&mut data_stream, &global_data_summary).await;
    assert_some!(stream_listener.select_next_some().now_or_never());
    assert_none!(stream_listener.select_next_some().now_or_never());

    // Verify more client requests have been made
    verify_num_sent_requests(&mut data_stream, max_pending_requests);
}

#[tokio::test]
async fn test_stream_max_pending_requests_freeze_dynamic() {
    // Create a dynamic prefetching config with prefetching enabled
    let initial_prefetching_value = 10;
    let max_prefetching_value = 12;
    let prefetching_value_increase = 2;
    let prefetching_value_decrease = 2;
    let timeout_freeze_duration_secs = 10;
    let dynamic_prefetching_config = DynamicPrefetchingConfig {
        enable_dynamic_prefetching: true,
        initial_prefetching_value,
        max_prefetching_value,
        prefetching_value_increase,
        prefetching_value_decrease,
        timeout_freeze_duration_secs,
        ..Default::default()
    };

    // Create an data streaming service config with prefetching enabled
    let max_pending_requests = 100;
    let streaming_service_config = DataStreamingServiceConfig {
        dynamic_prefetching: dynamic_prefetching_config,
        max_pending_requests,
        ..Default::default()
    };

    // Create an epoch ending data stream
    let stream_request =
        StreamRequest::GetAllEpochEndingLedgerInfos(GetAllEpochEndingLedgerInfosRequest {
            start_epoch: MIN_ADVERTISED_EPOCH_END,
        });
    let (mut data_stream, mut stream_listener, time_service) = create_data_stream(
        AptosDataClientConfig::default(),
        streaming_service_config,
        stream_request,
    );

    // Initialize the data stream
    let global_data_summary = create_global_data_summary(1);
    initialize_data_requests(&mut data_stream, &global_data_summary);

    // Verify that the correct number of client requests have been made
    verify_num_sent_requests(&mut data_stream, initial_prefetching_value);

    // Set a valid response for each request except the first one
    set_epoch_ending_response_for_indices(
        &mut data_stream,
        initial_prefetching_value,
        (1..initial_prefetching_value).collect::<Vec<_>>(),
    );

    // Set an invalid response for the first request
    set_failure_response_in_queue(&mut data_stream, 0);

    // Process the responses and verify we get no notifications
    process_data_responses(&mut data_stream, &global_data_summary).await;
    assert_none!(stream_listener.select_next_some().now_or_never());

    // Verify that the correct number of client requests have been made
    let mut num_expected_pending_requests =
        ((initial_prefetching_value * 2) - prefetching_value_decrease) - 1; // The first request failed
    verify_num_sent_requests(&mut data_stream, num_expected_pending_requests);

    // Set a valid response for each request except the first one
    set_epoch_ending_response_for_indices(
        &mut data_stream,
        num_expected_pending_requests,
        (1..num_expected_pending_requests).collect::<Vec<_>>(),
    );

    // Set an invalid response for the first request
    set_failure_response_in_queue(&mut data_stream, 0);

    // Elapse some time (but not enough for the prefetching value to be unfrozen)
    let time_service = time_service.into_mock();
    time_service.advance(Duration::from_secs(timeout_freeze_duration_secs / 2));

    // Process the responses and verify we get no notifications
    process_data_responses(&mut data_stream, &global_data_summary).await;
    assert_none!(stream_listener.select_next_some().now_or_never());

    // Verify that the correct number of client requests have been made
    num_expected_pending_requests +=
        initial_prefetching_value - (prefetching_value_decrease * 2) - 1; // The first request failed
    verify_num_sent_requests(&mut data_stream, num_expected_pending_requests);

    // Set a valid response for all requests
    set_epoch_ending_response_for_indices(
        &mut data_stream,
        num_expected_pending_requests,
        (0..num_expected_pending_requests).collect::<Vec<_>>(),
    );

    // Process the responses and verify we get the correct number of notifications
    process_data_responses(&mut data_stream, &global_data_summary).await;
    for _ in 0..num_expected_pending_requests {
        assert_some!(stream_listener.select_next_some().now_or_never());
    }
    assert_none!(stream_listener.select_next_some().now_or_never());

    // Verify the correct number of client requests have been made
    let num_expected_pending_requests =
        initial_prefetching_value - (prefetching_value_decrease * 2);
    verify_num_sent_requests(&mut data_stream, num_expected_pending_requests);

    // Elapse enough time for the prefetching value to be unfrozen
    time_service.advance(Duration::from_secs(timeout_freeze_duration_secs + 1));

    // Set a valid response for all requests
    set_epoch_ending_response_for_indices(
        &mut data_stream,
        num_expected_pending_requests,
        (0..num_expected_pending_requests).collect::<Vec<_>>(),
    );

    // Process the responses and verify we get the correct number of notifications
    process_data_responses(&mut data_stream, &global_data_summary).await;
    for _ in 0..num_expected_pending_requests {
        assert_some!(stream_listener.select_next_some().now_or_never());
    }
    assert_none!(stream_listener.select_next_some().now_or_never());

    // Verify the correct number of client requests have been made
    verify_num_sent_requests(&mut data_stream, max_prefetching_value);
}

#[tokio::test]
async fn test_stream_max_pending_requests_missing_data() {
    // Create an epoch ending data stream with dynamic prefetching disabled
    let max_concurrent_requests = 1;
    let max_pending_requests = 3;
    let dynamic_prefetching_config = DynamicPrefetchingConfig {
        enable_dynamic_prefetching: false,
        ..Default::default()
    };
    let streaming_service_config = DataStreamingServiceConfig {
        dynamic_prefetching: dynamic_prefetching_config,
        max_concurrent_requests,
        max_pending_requests,
        ..Default::default()
    };
    let (mut data_stream, mut stream_listener) = create_epoch_ending_stream(
        AptosDataClientConfig::default(),
        streaming_service_config,
        MIN_ADVERTISED_EPOCH_END,
    );

    // Initialize the data stream
    let optimal_epoch_chunk_sizes = 2;
    let global_data_summary = create_global_data_summary(optimal_epoch_chunk_sizes);
    initialize_data_requests(&mut data_stream, &global_data_summary);

    // Verify that the correct number of client requests have been made
    verify_num_sent_requests(&mut data_stream, max_concurrent_requests);

    // Set a valid (but partial) response for the first request
    set_epoch_ending_response_in_queue(&mut data_stream, 0, 1);

    // Process the responses and verify we get a notification
    process_data_responses(&mut data_stream, &global_data_summary).await;
    assert_some!(stream_listener.select_next_some().now_or_never());

    // Verify that the correct number of client requests have been made
    verify_num_sent_requests(&mut data_stream, max_concurrent_requests);

    // Set a valid (now complete) response for the first request
    set_epoch_ending_response_in_queue(&mut data_stream, 0, 1);

    // Process the responses and verify we get a notification
    process_data_responses(&mut data_stream, &global_data_summary).await;
    assert_some!(stream_listener.select_next_some().now_or_never());

    // Verify that no more client requests have been made
    verify_num_sent_requests(&mut data_stream, max_concurrent_requests);

    // Set a valid (but partial) response for the first request again
    set_epoch_ending_response_in_queue(&mut data_stream, 0, 1);

    // Process the responses and verify we get a notification
    process_data_responses(&mut data_stream, &global_data_summary).await;
    assert_some!(stream_listener.select_next_some().now_or_never());

    // Verify that the correct number of client requests have been made
    verify_num_sent_requests(&mut data_stream, max_concurrent_requests);
}

#[tokio::test]
async fn test_stream_max_pending_requests_missing_data_dynamic() {
    // Create a dynamic prefetching config with prefetching enabled
    let initial_prefetching_value = 3;
    let min_prefetching_value = 1;
    let prefetching_value_increase = 2;
    let dynamic_prefetching_config = DynamicPrefetchingConfig {
        enable_dynamic_prefetching: true,
        initial_prefetching_value,
        min_prefetching_value,
        prefetching_value_increase,
        timeout_freeze_duration_secs: 0, // Don't freeze the prefetching value
        ..Default::default()
    };

    // Create an epoch ending data stream
    let max_pending_requests = 10;
    let streaming_service_config = DataStreamingServiceConfig {
        dynamic_prefetching: dynamic_prefetching_config,
        max_pending_requests,
        ..Default::default()
    };
    let (mut data_stream, mut stream_listener) = create_epoch_ending_stream(
        AptosDataClientConfig::default(),
        streaming_service_config,
        MIN_ADVERTISED_EPOCH_END,
    );

    // Initialize the data stream
    let optimal_epoch_chunk_sizes = 2;
    let global_data_summary = create_global_data_summary(optimal_epoch_chunk_sizes);
    initialize_data_requests(&mut data_stream, &global_data_summary);

    // Verify that the correct number of client requests have been made
    verify_num_sent_requests(&mut data_stream, initial_prefetching_value);

    // Set a valid (but partial) response for the first request
    set_epoch_ending_response_in_queue(&mut data_stream, 0, 1);

    // Process the responses and verify we get a notification
    process_data_responses(&mut data_stream, &global_data_summary).await;
    assert_some!(stream_listener.select_next_some().now_or_never());

    // Verify that the correct number of client requests have been made
    verify_num_sent_requests(
        &mut data_stream,
        initial_prefetching_value + prefetching_value_increase,
    );

    // Set a valid (now complete) response for the first request
    set_epoch_ending_response_in_queue(&mut data_stream, 0, 1);

    // Process the responses and verify we get a notification
    process_data_responses(&mut data_stream, &global_data_summary).await;
    assert_some!(stream_listener.select_next_some().now_or_never());

    // Verify that more client requests have been made
    verify_num_sent_requests(
        &mut data_stream,
        initial_prefetching_value + (2 * prefetching_value_increase),
    );

    // Set a valid (but partial) response for the first request again
    set_epoch_ending_response_in_queue(&mut data_stream, 0, 1);

    // Process the responses and verify we get a notification
    process_data_responses(&mut data_stream, &global_data_summary).await;
    assert_some!(stream_listener.select_next_some().now_or_never());

    // Verify that the correct number of client requests have been made
    verify_num_sent_requests(
        &mut data_stream,
        initial_prefetching_value + (3 * prefetching_value_increase),
    );
}

#[tokio::test]
async fn test_continuous_stream_epoch_change_retry() {
    // Create a test streaming service config with dynamic prefetching disabled
    let max_request_retry = 10;
    let max_concurrent_requests = 3;
    let dynamic_prefetching_config = DynamicPrefetchingConfig {
        enable_dynamic_prefetching: false,
        ..Default::default()
    };
    let streaming_service_config = DataStreamingServiceConfig {
        dynamic_prefetching: dynamic_prefetching_config,
        max_concurrent_requests,
        max_request_retry,
        ..Default::default()
    };

    // Test all types of continuous data streams
    let (data_stream_1, _stream_listener_1, _) = create_continuous_transaction_stream(
        AptosDataClientConfig::default(),
        streaming_service_config,
        MIN_ADVERTISED_TRANSACTION,
        MIN_ADVERTISED_EPOCH_END,
    );
    let (data_stream_2, _stream_listener_2, _) = create_continuous_transaction_output_stream(
        AptosDataClientConfig::default(),
        streaming_service_config,
        MIN_ADVERTISED_TRANSACTION_OUTPUT,
        MIN_ADVERTISED_EPOCH_END,
    );
    let (data_stream_3, _stream_listener_3, _) = create_continuous_transaction_or_output_stream(
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
        verify_num_sent_requests(&mut data_stream, 1);

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
        verify_num_sent_requests(&mut data_stream, 3);
    }
}

#[tokio::test]
async fn test_continuous_stream_epoch_change_retry_dynamic() {
    // Create a dynamic prefetching config with prefetching enabled
    let initial_prefetching_value = 5;
    let min_prefetching_value = 2;
    let dynamic_prefetching_config = DynamicPrefetchingConfig {
        enable_dynamic_prefetching: true,
        initial_prefetching_value,
        min_prefetching_value,
        ..Default::default()
    };

    // Create a test streaming service config
    let max_request_retry = 10;
    let streaming_service_config = DataStreamingServiceConfig {
        dynamic_prefetching: dynamic_prefetching_config,
        max_request_retry,
        ..Default::default()
    };

    // Test all types of continuous data streams
    let (data_stream_1, _stream_listener_1, _) = create_continuous_transaction_stream(
        AptosDataClientConfig::default(),
        streaming_service_config,
        MIN_ADVERTISED_TRANSACTION,
        MIN_ADVERTISED_EPOCH_END,
    );
    let (data_stream_2, _stream_listener_2, _) = create_continuous_transaction_output_stream(
        AptosDataClientConfig::default(),
        streaming_service_config,
        MIN_ADVERTISED_TRANSACTION_OUTPUT,
        MIN_ADVERTISED_EPOCH_END,
    );
    let (data_stream_3, _stream_listener_3, _) = create_continuous_transaction_or_output_stream(
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
        verify_num_sent_requests(&mut data_stream, 1);

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
        verify_num_sent_requests(&mut data_stream, min_prefetching_value);
    }
}

#[tokio::test]
async fn test_continuous_stream_optimistic_fetch_retry() {
    // Create a test streaming service config with subscriptions disabled
    let max_request_retry = 3;
    let max_concurrent_requests = 3;
    let streaming_service_config = DataStreamingServiceConfig {
        enable_subscription_streaming: false,
        max_concurrent_requests,
        max_request_retry,
        ..Default::default()
    };

    // Test all types of continuous data streams
    let continuous_data_streams = enumerate_continuous_data_streams(
        AptosDataClientConfig::default(),
        streaming_service_config,
    );
    for (
        mut data_stream,
        mut stream_listener,
        _,
        transactions_only,
        allow_transactions_or_outputs,
    ) in continuous_data_streams
    {
        // Initialize the data stream
        let global_data_summary = create_global_data_summary(1);
        initialize_data_requests(&mut data_stream, &global_data_summary);

        // Verify a single request is made and that it contains the correct data
        verify_pending_optimistic_fetch(
            &mut data_stream,
            transactions_only,
            allow_transactions_or_outputs,
            0,
        );

        // Set a timeout response for the optimistic fetch request and process it
        set_timeout_response_in_queue(&mut data_stream, 0);
        process_data_responses(&mut data_stream, &global_data_summary).await;
        assert_none!(stream_listener.select_next_some().now_or_never());

        // Handle multiple timeouts and retries (because no new data is known)
        let client_request = get_pending_client_request(&mut data_stream, 0);
        for _ in 0..max_request_retry * 3 {
            // Set a timeout response for the request and process it
            set_timeout_response_in_queue(&mut data_stream, 0);
            process_data_responses(&mut data_stream, &global_data_summary).await;

            // Verify the same optimistic fetch request was resent to the network
            let new_client_request = get_pending_client_request(&mut data_stream, 0);
            assert_eq!(new_client_request, client_request);
        }

        // Set an optimistic fetch response in the queue and process it
        set_new_data_response_in_queue(
            &mut data_stream,
            0,
            MAX_ADVERTISED_TRANSACTION + 1,
            transactions_only,
        );
        process_data_responses(&mut data_stream, &global_data_summary).await;

        // Verify another optimistic fetch request is now sent
        verify_pending_optimistic_fetch(
            &mut data_stream,
            transactions_only,
            allow_transactions_or_outputs,
            1, // Offset by 1 (for data beyond the previous target)
        );

        // Set an error response for the optimistic fetch request and process it.
        // This will cause the same request to be re-sent.
        set_failure_response_in_queue(&mut data_stream, 0);
        process_data_responses(&mut data_stream, &global_data_summary).await;

        // Advertise new data and verify the data is requested
        advertise_new_data_and_verify_requests(
            &mut data_stream,
            global_data_summary,
            transactions_only,
            allow_transactions_or_outputs,
            max_concurrent_requests,
        )
        .await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_continuous_stream_optimistic_fetch_timeout() {
    // Create a test data client config
    let data_client_config = AptosDataClientConfig {
        optimistic_fetch_timeout_ms: 1005,
        ..Default::default()
    };

    // Create a test streaming service config with subscriptions disabled
    let streaming_service_config = DataStreamingServiceConfig {
        enable_subscription_streaming: false,
        ..Default::default()
    };

    // Verify the timeouts of all continuous data streams
    verify_continuous_stream_request_timeouts(
        data_client_config,
        streaming_service_config,
        1, // Optimistic fetch requests are only sent one at a time
    )
    .await;
}

#[tokio::test]
async fn test_continuous_stream_subscription_failures() {
    // Create a dynamic prefetching config with prefetching disabled
    let dynamic_prefetching = DynamicPrefetchingConfig {
        enable_dynamic_prefetching: false,
        ..Default::default()
    };

    // Create a test streaming service config with subscriptions enabled
    let max_request_retry = 3;
    let max_concurrent_requests = 3;
    let streaming_service_config = DataStreamingServiceConfig {
        dynamic_prefetching,
        enable_subscription_streaming: true,
        max_concurrent_requests,
        max_request_retry,
        ..Default::default()
    };

    // Test all types of continuous data streams
    let continuous_data_streams = enumerate_continuous_data_streams(
        AptosDataClientConfig::default(),
        streaming_service_config,
    );
    for (
        mut data_stream,
        mut stream_listener,
        _,
        transactions_only,
        allow_transactions_or_outputs,
    ) in continuous_data_streams
    {
        // Initialize the data stream
        let global_data_summary = create_global_data_summary(1);
        initialize_data_requests(&mut data_stream, &global_data_summary);

        // Fetch the subscription stream ID from the first pending request
        let mut subscription_stream_id = get_subscription_stream_id(&mut data_stream, 0);

        // Verify the pending requests are for the correct data and correctly formed
        verify_pending_subscription_requests(
            &mut data_stream,
            max_concurrent_requests,
            allow_transactions_or_outputs,
            transactions_only,
            0,
            subscription_stream_id,
            0,
        );

        // Set a failure response for the first subscription request and process it
        set_failure_response_in_queue(&mut data_stream, 0);
        process_data_responses(&mut data_stream, &global_data_summary).await;
        assert_none!(stream_listener.select_next_some().now_or_never());

        // Handle multiple timeouts and retries
        for _ in 0..max_request_retry * 3 {
            // Set a timeout response for the first request and process it
            set_timeout_response_in_queue(&mut data_stream, 0);
            process_data_responses(&mut data_stream, &global_data_summary).await;

            // Fetch the subscription stream ID from the first pending request
            let next_subscription_stream_id = get_subscription_stream_id(&mut data_stream, 0);

            // Verify the next stream ID is different from the previous one
            assert_ne!(subscription_stream_id, next_subscription_stream_id);
            subscription_stream_id = next_subscription_stream_id;

            // Verify the pending requests are for the correct data and correctly formed
            verify_pending_subscription_requests(
                &mut data_stream,
                max_concurrent_requests,
                allow_transactions_or_outputs,
                transactions_only,
                0,
                subscription_stream_id,
                0,
            );
        }

        // Set a failure response for the first request and process it
        set_failure_response_in_queue(&mut data_stream, 0);
        process_data_responses(&mut data_stream, &global_data_summary).await;

        // Fetch the next subscription stream ID from the first pending request
        let next_subscription_stream_id = get_subscription_stream_id(&mut data_stream, 0);

        // Verify the next stream ID is different from the previous one
        assert_ne!(subscription_stream_id, next_subscription_stream_id);
        subscription_stream_id = next_subscription_stream_id;

        // Verify the pending requests are for the correct data and correctly formed
        verify_pending_subscription_requests(
            &mut data_stream,
            max_concurrent_requests,
            allow_transactions_or_outputs,
            transactions_only,
            0,
            subscription_stream_id,
            0,
        );

        // Set a subscription response in the queue and process it
        set_new_data_response_in_queue(
            &mut data_stream,
            0,
            MAX_ADVERTISED_TRANSACTION + 1,
            transactions_only,
        );
        process_data_responses(&mut data_stream, &global_data_summary).await;

        // Verify the pending requests are for the correct data and correctly formed
        verify_pending_subscription_requests(
            &mut data_stream,
            max_concurrent_requests,
            allow_transactions_or_outputs,
            transactions_only,
            1,
            subscription_stream_id, // The subscription stream ID should be the same
            0,
        );

        // Set a timeout response for the subscription request and process it
        set_timeout_response_in_queue(&mut data_stream, 0);
        process_data_responses(&mut data_stream, &global_data_summary).await;

        // Advertise new data and verify the data is requested
        advertise_new_data_and_verify_requests(
            &mut data_stream,
            global_data_summary,
            transactions_only,
            allow_transactions_or_outputs,
            max_concurrent_requests,
        )
        .await;
    }
}

#[tokio::test]
async fn test_continuous_stream_subscription_failures_prefetching() {
    // Create a dynamic prefetching config with prefetching enabled
    let max_in_flight_subscription_requests = 5;
    let initial_prefetching_value = 7;
    let dynamic_prefetching = DynamicPrefetchingConfig {
        enable_dynamic_prefetching: true,
        initial_prefetching_value,
        max_in_flight_subscription_requests,
        ..Default::default()
    };

    // Create a test streaming service config with subscriptions enabled
    let max_request_retry = 3;
    let streaming_service_config = DataStreamingServiceConfig {
        dynamic_prefetching,
        enable_subscription_streaming: true,
        max_request_retry,
        ..Default::default()
    };

    // Test all types of continuous data streams
    let continuous_data_streams = enumerate_continuous_data_streams(
        AptosDataClientConfig::default(),
        streaming_service_config,
    );
    for (
        mut data_stream,
        mut stream_listener,
        _,
        transactions_only,
        allow_transactions_or_outputs,
    ) in continuous_data_streams
    {
        // Initialize the data stream
        let global_data_summary = create_global_data_summary(1);
        initialize_data_requests(&mut data_stream, &global_data_summary);

        // Fetch the subscription stream ID from the first pending request
        let mut subscription_stream_id = get_subscription_stream_id(&mut data_stream, 0);

        // Verify the pending requests are for the correct data and correctly formed
        verify_pending_subscription_requests(
            &mut data_stream,
            max_in_flight_subscription_requests,
            allow_transactions_or_outputs,
            transactions_only,
            0,
            subscription_stream_id,
            0,
        );

        // Set a failure response for the first subscription request and process it
        set_failure_response_in_queue(&mut data_stream, 0);
        process_data_responses(&mut data_stream, &global_data_summary).await;
        assert_none!(stream_listener.select_next_some().now_or_never());

        // Handle multiple timeouts and retries
        for _ in 0..max_request_retry * 3 {
            // Set a timeout response for the first request and process it
            set_timeout_response_in_queue(&mut data_stream, 0);
            process_data_responses(&mut data_stream, &global_data_summary).await;

            // Fetch the subscription stream ID from the first pending request
            let next_subscription_stream_id = get_subscription_stream_id(&mut data_stream, 0);

            // Verify the next stream ID is different from the previous one
            assert_ne!(subscription_stream_id, next_subscription_stream_id);
            subscription_stream_id = next_subscription_stream_id;

            // Verify the pending requests are for the correct data and correctly formed
            verify_pending_subscription_requests(
                &mut data_stream,
                max_in_flight_subscription_requests,
                allow_transactions_or_outputs,
                transactions_only,
                0,
                subscription_stream_id,
                0,
            );
        }

        // Set a failure response for the first request and process it
        set_failure_response_in_queue(&mut data_stream, 0);
        process_data_responses(&mut data_stream, &global_data_summary).await;

        // Fetch the next subscription stream ID from the first pending request
        let next_subscription_stream_id = get_subscription_stream_id(&mut data_stream, 0);

        // Verify the next stream ID is different from the previous one
        assert_ne!(subscription_stream_id, next_subscription_stream_id);
        subscription_stream_id = next_subscription_stream_id;

        // Verify the pending requests are for the correct data and correctly formed
        verify_pending_subscription_requests(
            &mut data_stream,
            max_in_flight_subscription_requests,
            allow_transactions_or_outputs,
            transactions_only,
            0,
            subscription_stream_id,
            0,
        );

        // Set a subscription response in the queue and process it
        set_new_data_response_in_queue(
            &mut data_stream,
            0,
            MAX_ADVERTISED_TRANSACTION + 1,
            transactions_only,
        );
        process_data_responses(&mut data_stream, &global_data_summary).await;

        // Verify the pending requests are for the correct data and correctly formed
        verify_pending_subscription_requests(
            &mut data_stream,
            max_in_flight_subscription_requests,
            allow_transactions_or_outputs,
            transactions_only,
            1,
            subscription_stream_id, // The subscription stream ID should be the same
            0,
        );

        // Set a timeout response for the subscription request and process it
        set_timeout_response_in_queue(&mut data_stream, 0);
        process_data_responses(&mut data_stream, &global_data_summary).await;

        // Advertise new data and verify the data is requested
        advertise_new_data_and_verify_requests(
            &mut data_stream,
            global_data_summary,
            transactions_only,
            allow_transactions_or_outputs,
            initial_prefetching_value,
        )
        .await;
    }
}

#[tokio::test]
async fn test_continuous_stream_subscription_lag() {
    // Create a dynamic prefetching config with prefetching disabled
    let dynamic_prefetching = DynamicPrefetchingConfig {
        enable_dynamic_prefetching: false,
        ..Default::default()
    };

    // Create a test streaming service config with subscriptions enabled
    let max_concurrent_requests = 3;
    let max_subscription_stream_lag_secs = 10;
    let streaming_service_config = DataStreamingServiceConfig {
        dynamic_prefetching,
        enable_subscription_streaming: true,
        max_subscription_stream_lag_secs,
        max_concurrent_requests,
        ..Default::default()
    };

    // Test all types of continuous data streams
    let continuous_data_streams = enumerate_continuous_data_streams(
        AptosDataClientConfig::default(),
        streaming_service_config,
    );
    for (
        mut data_stream,
        mut stream_listener,
        time_service,
        transactions_only,
        allow_transactions_or_outputs,
    ) in continuous_data_streams
    {
        // Initialize the data stream
        let mut global_data_summary = create_global_data_summary(1);
        initialize_data_requests(&mut data_stream, &global_data_summary);

        // Fetch the subscription stream ID from the first pending request
        let subscription_stream_id = get_subscription_stream_id(&mut data_stream, 0);

        // Verify the pending subscription requests
        verify_pending_subscription_requests(
            &mut data_stream,
            max_concurrent_requests,
            allow_transactions_or_outputs,
            transactions_only,
            0,
            subscription_stream_id,
            0,
        );

        // Update the global data summary to be ahead of the subscription stream
        let highest_advertised_version = MAX_ADVERTISED_TRANSACTION + 1000;
        global_data_summary.advertised_data.synced_ledger_infos = vec![create_ledger_info(
            highest_advertised_version,
            MAX_ADVERTISED_EPOCH_END,
            false,
        )];

        // Set a valid response for the first subscription request and process it
        let highest_response_version = highest_advertised_version - 900; // Behind the advertised version
        set_new_data_response_in_queue(
            &mut data_stream,
            0,
            highest_response_version,
            transactions_only,
        );
        process_data_responses(&mut data_stream, &global_data_summary).await;
        assert_some!(stream_listener.select_next_some().now_or_never());

        // Update the global data summary to be further ahead of the subscription stream
        let highest_advertised_version = MAX_ADVERTISED_TRANSACTION + 2000;
        global_data_summary.advertised_data.synced_ledger_infos = vec![create_ledger_info(
            highest_advertised_version,
            MAX_ADVERTISED_EPOCH_END,
            false,
        )];

        // Elapse some time (but not enough for the stream to be killed)
        let time_service = time_service.into_mock();
        time_service.advance_secs(max_subscription_stream_lag_secs / 2);

        // Set a valid response for the first subscription request and process it
        let highest_response_version = highest_advertised_version - 1000; // Further behind the advertised
        set_new_data_response_in_queue(
            &mut data_stream,
            0,
            highest_response_version,
            transactions_only,
        );
        process_data_responses(&mut data_stream, &global_data_summary).await;
        assert_some!(stream_listener.select_next_some().now_or_never());

        // Elapse enough time for the stream to be killed
        time_service.advance_secs(max_subscription_stream_lag_secs);

        // Set a valid response for the first subscription request and process it
        let highest_response_version = highest_advertised_version - 901; // Behind the initial lag
        set_new_data_response_in_queue(
            &mut data_stream,
            0,
            highest_response_version,
            transactions_only,
        );
        process_data_responses(&mut data_stream, &global_data_summary).await;
        assert_some!(stream_listener.select_next_some().now_or_never());

        // Verify that we no longer have pending subscription requests (the stream was killed)
        let client_request = get_pending_client_request(&mut data_stream, 0);
        assert!(!client_request.is_subscription_request());

        // Verify that the subscription stream lag has been reset
        assert!(data_stream.get_subscription_stream_lag().is_none());
    }
}

#[tokio::test]
async fn test_continuous_stream_subscription_lag_bounded() {
    // Create a test streaming service config with subscriptions enabled
    let max_subscription_stream_lag_secs = 10;
    let streaming_service_config = DataStreamingServiceConfig {
        enable_subscription_streaming: true,
        max_subscription_stream_lag_secs,
        ..Default::default()
    };

    // Test all types of continuous data streams
    let continuous_data_streams = enumerate_continuous_data_streams(
        AptosDataClientConfig::default(),
        streaming_service_config,
    );
    for (mut data_stream, mut stream_listener, time_service, transactions_only, _) in
        continuous_data_streams
    {
        // Initialize the data stream
        let mut global_data_summary = create_global_data_summary(1);
        initialize_data_requests(&mut data_stream, &global_data_summary);

        // Update the global data summary to be ahead of the subscription stream
        let highest_advertised_version = MAX_ADVERTISED_TRANSACTION + 500;
        global_data_summary.advertised_data.synced_ledger_infos = vec![create_ledger_info(
            highest_advertised_version,
            MAX_ADVERTISED_EPOCH_END,
            false,
        )];

        // Set a valid response for the first subscription request and process it
        let highest_response_version = highest_advertised_version - 300; // Behind the advertised version
        set_new_data_response_in_queue(
            &mut data_stream,
            0,
            highest_response_version,
            transactions_only,
        );
        process_data_responses(&mut data_stream, &global_data_summary).await;
        assert_some!(stream_listener.select_next_some().now_or_never());

        // Verify the stream is now tracking the subscription lag
        let subscription_stream_lag = data_stream.get_subscription_stream_lag().unwrap();
        assert_eq!(
            subscription_stream_lag.version_lag,
            highest_advertised_version - highest_response_version
        );

        // Elapse enough time for the stream to be killed
        let time_service = time_service.into_mock();
        time_service.advance_secs(max_subscription_stream_lag_secs);

        // Update the global data summary to be further ahead (by 1)
        let highest_advertised_version = highest_advertised_version + 1;
        global_data_summary.advertised_data.synced_ledger_infos = vec![create_ledger_info(
            highest_advertised_version,
            MAX_ADVERTISED_EPOCH_END,
            false,
        )];

        // Set a valid response for the first subscription request and process it
        let highest_response_version = highest_response_version + 1; // Still behind, but not worse
        set_new_data_response_in_queue(
            &mut data_stream,
            0,
            highest_response_version,
            transactions_only,
        );
        process_data_responses(&mut data_stream, &global_data_summary).await;
        assert_some!(stream_listener.select_next_some().now_or_never());

        // Elapse enough time for the stream to be killed (again)
        time_service.advance_secs(max_subscription_stream_lag_secs);

        // Update the global data summary to be further ahead (by 10)
        let highest_advertised_version = highest_advertised_version + 10;
        global_data_summary.advertised_data.synced_ledger_infos = vec![create_ledger_info(
            highest_advertised_version,
            MAX_ADVERTISED_EPOCH_END,
            false,
        )];

        // Set a valid response for the first subscription request and process it
        let highest_response_version = highest_response_version + 10; // Still behind, but not worse
        set_new_data_response_in_queue(
            &mut data_stream,
            0,
            highest_response_version,
            transactions_only,
        );
        process_data_responses(&mut data_stream, &global_data_summary).await;
        assert_some!(stream_listener.select_next_some().now_or_never());

        // Elapse enough time for the stream to be killed (again)
        time_service.advance_secs(max_subscription_stream_lag_secs);

        // Update the global data summary to be further ahead (by 100)
        let highest_advertised_version = highest_advertised_version + 100;
        global_data_summary.advertised_data.synced_ledger_infos = vec![create_ledger_info(
            highest_advertised_version,
            MAX_ADVERTISED_EPOCH_END,
            false,
        )];

        // Set a valid response for the first subscription request and process it
        let highest_response_version = highest_response_version + 101; // Still behind, but slightly better
        set_new_data_response_in_queue(
            &mut data_stream,
            0,
            highest_response_version,
            transactions_only,
        );
        process_data_responses(&mut data_stream, &global_data_summary).await;
        assert_some!(stream_listener.select_next_some().now_or_never());

        // Verify the state of the subscription stream lag
        let subscription_stream_lag = data_stream.get_subscription_stream_lag().unwrap();
        assert_eq!(
            subscription_stream_lag.version_lag,
            highest_advertised_version - highest_response_version
        );

        // Update the global data summary to be further ahead (by 100)
        let highest_advertised_version = highest_advertised_version + 100;
        global_data_summary.advertised_data.synced_ledger_infos = vec![create_ledger_info(
            highest_advertised_version,
            MAX_ADVERTISED_EPOCH_END,
            false,
        )];

        // Set a valid response for the first subscription request and process it
        let highest_response_version = highest_response_version + 150; // Still behind, but slightly better
        set_new_data_response_in_queue(
            &mut data_stream,
            0,
            highest_response_version,
            transactions_only,
        );
        process_data_responses(&mut data_stream, &global_data_summary).await;
        assert_some!(stream_listener.select_next_some().now_or_never());

        // Verify the state of the subscription stream lag
        let subscription_stream_lag = data_stream.get_subscription_stream_lag().unwrap();
        assert_eq!(
            subscription_stream_lag.version_lag,
            highest_advertised_version - highest_response_version
        );
    }
}

#[tokio::test]
async fn test_continuous_stream_subscription_lag_catch_up() {
    // Create a dynamic prefetching config with prefetching disabled
    let dynamic_prefetching = DynamicPrefetchingConfig {
        enable_dynamic_prefetching: false,
        ..Default::default()
    };

    // Create a test streaming service config with subscriptions enabled
    let max_concurrent_requests = 3;
    let max_subscription_stream_lag_secs = 10;
    let streaming_service_config = DataStreamingServiceConfig {
        dynamic_prefetching,
        enable_subscription_streaming: true,
        max_subscription_stream_lag_secs,
        max_concurrent_requests,
        ..Default::default()
    };

    // Test all types of continuous data streams
    let continuous_data_streams = enumerate_continuous_data_streams(
        AptosDataClientConfig::default(),
        streaming_service_config,
    );
    for (
        mut data_stream,
        mut stream_listener,
        time_service,
        transactions_only,
        allow_transactions_or_outputs,
    ) in continuous_data_streams
    {
        // Initialize the data stream
        let mut global_data_summary = create_global_data_summary(1);
        initialize_data_requests(&mut data_stream, &global_data_summary);

        // Fetch the subscription stream ID from the first pending request
        let subscription_stream_id = get_subscription_stream_id(&mut data_stream, 0);

        // Update the global data summary to be ahead of the subscription stream
        let highest_advertised_version = MAX_ADVERTISED_TRANSACTION + 1000;
        global_data_summary.advertised_data.synced_ledger_infos = vec![create_ledger_info(
            highest_advertised_version,
            MAX_ADVERTISED_EPOCH_END,
            false,
        )];

        // Set a valid response for the first subscription request and process it
        let highest_response_version = highest_advertised_version - 500; // Behind the advertised version
        set_new_data_response_in_queue(
            &mut data_stream,
            0,
            highest_response_version,
            transactions_only,
        );
        process_data_responses(&mut data_stream, &global_data_summary).await;
        assert_some!(stream_listener.select_next_some().now_or_never());

        // Verify the stream is now tracking the subscription lag
        let subscription_stream_lag = data_stream.get_subscription_stream_lag().unwrap();
        assert_eq!(subscription_stream_lag.start_time, time_service.now());
        assert_eq!(
            subscription_stream_lag.version_lag,
            highest_advertised_version - highest_response_version
        );

        // Elapse enough time for the stream to be killed
        let time_service = time_service.into_mock();
        time_service.advance_secs(max_subscription_stream_lag_secs);

        // Update the global data summary to be further ahead (by 1)
        let highest_advertised_version = highest_advertised_version + 1;
        global_data_summary.advertised_data.synced_ledger_infos = vec![create_ledger_info(
            highest_advertised_version,
            MAX_ADVERTISED_EPOCH_END,
            false,
        )];

        // Set a valid response for the first subscription request and process it
        let highest_response_version = highest_response_version + 1; // Still behind, but not worse
        set_new_data_response_in_queue(
            &mut data_stream,
            0,
            highest_response_version,
            transactions_only,
        );
        process_data_responses(&mut data_stream, &global_data_summary).await;
        assert_some!(stream_listener.select_next_some().now_or_never());

        // Verify that we still have pending subscription requests (the stream hasn't fallen further behind)
        verify_pending_subscription_requests(
            &mut data_stream,
            max_concurrent_requests,
            allow_transactions_or_outputs,
            transactions_only,
            2,
            subscription_stream_id,
            0,
        );

        // Verify the state of the subscription stream lag
        let subscription_stream_lag = data_stream.get_subscription_stream_lag().unwrap();
        assert_eq!(
            subscription_stream_lag.version_lag,
            highest_advertised_version - highest_response_version
        );

        // Set a valid response for the first subscription request and process it
        set_new_data_response_in_queue(
            &mut data_stream,
            0,
            highest_advertised_version, // Catch the stream up to the advertised version
            transactions_only,
        );
        process_data_responses(&mut data_stream, &global_data_summary).await;
        assert_some!(stream_listener.select_next_some().now_or_never());

        // Verify that the subscription stream lag has now been reset (the stream caught up)
        assert!(data_stream.get_subscription_stream_lag().is_none());
    }
}

#[tokio::test]
async fn test_continuous_stream_subscription_lag_catch_up_prefetching() {
    // Create a dynamic prefetching config with prefetching enabled
    let max_in_flight_subscription_requests = 4;
    let dynamic_prefetching = DynamicPrefetchingConfig {
        enable_dynamic_prefetching: true,
        max_in_flight_subscription_requests,
        ..Default::default()
    };

    // Create a test streaming service config with subscriptions enabled
    let max_subscription_stream_lag_secs = 10;
    let streaming_service_config = DataStreamingServiceConfig {
        dynamic_prefetching,
        enable_subscription_streaming: true,
        max_subscription_stream_lag_secs,
        ..Default::default()
    };

    // Test all types of continuous data streams
    let continuous_data_streams = enumerate_continuous_data_streams(
        AptosDataClientConfig::default(),
        streaming_service_config,
    );
    for (
        mut data_stream,
        mut stream_listener,
        time_service,
        transactions_only,
        allow_transactions_or_outputs,
    ) in continuous_data_streams
    {
        // Initialize the data stream
        let mut global_data_summary = create_global_data_summary(1);
        initialize_data_requests(&mut data_stream, &global_data_summary);

        // Fetch the subscription stream ID from the first pending request
        let subscription_stream_id = get_subscription_stream_id(&mut data_stream, 0);

        // Update the global data summary to be ahead of the subscription stream
        let highest_advertised_version = MAX_ADVERTISED_TRANSACTION + 1000;
        global_data_summary.advertised_data.synced_ledger_infos = vec![create_ledger_info(
            highest_advertised_version,
            MAX_ADVERTISED_EPOCH_END,
            false,
        )];

        // Set a valid response for the first subscription request and process it
        let highest_response_version = highest_advertised_version - 500; // Behind the advertised version
        set_new_data_response_in_queue(
            &mut data_stream,
            0,
            highest_response_version,
            transactions_only,
        );
        process_data_responses(&mut data_stream, &global_data_summary).await;
        assert_some!(stream_listener.select_next_some().now_or_never());

        // Verify the stream is now tracking the subscription lag
        let subscription_stream_lag = data_stream.get_subscription_stream_lag().unwrap();
        assert_eq!(subscription_stream_lag.start_time, time_service.now());
        assert_eq!(
            subscription_stream_lag.version_lag,
            highest_advertised_version - highest_response_version
        );

        // Elapse enough time for the stream to be killed
        let time_service = time_service.into_mock();
        time_service.advance_secs(max_subscription_stream_lag_secs);

        // Update the global data summary to be further ahead (by 1)
        let highest_advertised_version = highest_advertised_version + 1;
        global_data_summary.advertised_data.synced_ledger_infos = vec![create_ledger_info(
            highest_advertised_version,
            MAX_ADVERTISED_EPOCH_END,
            false,
        )];

        // Set a valid response for the first subscription request and process it
        let highest_response_version = highest_response_version + 1; // Still behind, but not worse
        set_new_data_response_in_queue(
            &mut data_stream,
            0,
            highest_response_version,
            transactions_only,
        );
        process_data_responses(&mut data_stream, &global_data_summary).await;
        assert_some!(stream_listener.select_next_some().now_or_never());

        // Verify that we still have pending subscription requests (the stream hasn't fallen further behind)
        verify_pending_subscription_requests(
            &mut data_stream,
            max_in_flight_subscription_requests,
            allow_transactions_or_outputs,
            transactions_only,
            2,
            subscription_stream_id,
            0,
        );

        // Verify the state of the subscription stream lag
        let subscription_stream_lag = data_stream.get_subscription_stream_lag().unwrap();
        assert_eq!(
            subscription_stream_lag.version_lag,
            highest_advertised_version - highest_response_version
        );

        // Set a valid response for the first subscription request and process it
        set_new_data_response_in_queue(
            &mut data_stream,
            0,
            highest_advertised_version, // Catch the stream up to the advertised version
            transactions_only,
        );
        process_data_responses(&mut data_stream, &global_data_summary).await;
        assert_some!(stream_listener.select_next_some().now_or_never());

        // Verify that the subscription stream lag has now been reset (the stream caught up)
        assert!(data_stream.get_subscription_stream_lag().is_none());
    }
}

#[tokio::test]
async fn test_continuous_stream_subscription_lag_prefetching() {
    // Create a dynamic prefetching config with prefetching enabled
    let max_in_flight_subscription_requests = 7;
    let dynamic_prefetching = DynamicPrefetchingConfig {
        enable_dynamic_prefetching: true,
        max_in_flight_subscription_requests,
        ..Default::default()
    };

    // Create a test streaming service config with subscriptions enabled
    let max_subscription_stream_lag_secs = 10;
    let streaming_service_config = DataStreamingServiceConfig {
        dynamic_prefetching,
        enable_subscription_streaming: true,
        max_subscription_stream_lag_secs,
        ..Default::default()
    };

    // Test all types of continuous data streams
    let continuous_data_streams = enumerate_continuous_data_streams(
        AptosDataClientConfig::default(),
        streaming_service_config,
    );
    for (
        mut data_stream,
        mut stream_listener,
        time_service,
        transactions_only,
        allow_transactions_or_outputs,
    ) in continuous_data_streams
    {
        // Initialize the data stream
        let mut global_data_summary = create_global_data_summary(1);
        initialize_data_requests(&mut data_stream, &global_data_summary);

        // Fetch the subscription stream ID from the first pending request
        let subscription_stream_id = get_subscription_stream_id(&mut data_stream, 0);

        // Verify the pending subscription requests
        verify_pending_subscription_requests(
            &mut data_stream,
            max_in_flight_subscription_requests,
            allow_transactions_or_outputs,
            transactions_only,
            0,
            subscription_stream_id,
            0,
        );

        // Update the global data summary to be ahead of the subscription stream
        let highest_advertised_version = MAX_ADVERTISED_TRANSACTION + 1000;
        global_data_summary.advertised_data.synced_ledger_infos = vec![create_ledger_info(
            highest_advertised_version,
            MAX_ADVERTISED_EPOCH_END,
            false,
        )];

        // Set a valid response for the first subscription request and process it
        let highest_response_version = highest_advertised_version - 900; // Behind the advertised version
        set_new_data_response_in_queue(
            &mut data_stream,
            0,
            highest_response_version,
            transactions_only,
        );
        process_data_responses(&mut data_stream, &global_data_summary).await;
        assert_some!(stream_listener.select_next_some().now_or_never());

        // Update the global data summary to be further ahead of the subscription stream
        let highest_advertised_version = MAX_ADVERTISED_TRANSACTION + 2000;
        global_data_summary.advertised_data.synced_ledger_infos = vec![create_ledger_info(
            highest_advertised_version,
            MAX_ADVERTISED_EPOCH_END,
            false,
        )];

        // Elapse some time (but not enough for the stream to be killed)
        let time_service = time_service.into_mock();
        time_service.advance_secs(max_subscription_stream_lag_secs / 2);

        // Set a valid response for the first subscription request and process it
        let highest_response_version = highest_advertised_version - 1000; // Further behind the advertised
        set_new_data_response_in_queue(
            &mut data_stream,
            0,
            highest_response_version,
            transactions_only,
        );
        process_data_responses(&mut data_stream, &global_data_summary).await;
        assert_some!(stream_listener.select_next_some().now_or_never());

        // Elapse enough time for the stream to be killed
        time_service.advance_secs(max_subscription_stream_lag_secs);

        // Set a valid response for the first subscription request and process it
        let highest_response_version = highest_advertised_version - 901; // Behind the initial lag
        set_new_data_response_in_queue(
            &mut data_stream,
            0,
            highest_response_version,
            transactions_only,
        );
        process_data_responses(&mut data_stream, &global_data_summary).await;
        assert_some!(stream_listener.select_next_some().now_or_never());

        // Verify that we no longer have pending subscription requests (the stream was killed)
        let client_request = get_pending_client_request(&mut data_stream, 0);
        assert!(!client_request.is_subscription_request());

        // Verify that the subscription stream lag has been reset
        assert!(data_stream.get_subscription_stream_lag().is_none());
    }
}

#[tokio::test]
async fn test_continuous_stream_subscription_lag_time() {
    // Create a test streaming service config with subscriptions enabled
    let max_subscription_stream_lag_secs = 100;
    let streaming_service_config = DataStreamingServiceConfig {
        enable_subscription_streaming: true,
        max_subscription_stream_lag_secs,
        ..Default::default()
    };

    // Test all types of continuous data streams
    let continuous_data_streams = enumerate_continuous_data_streams(
        AptosDataClientConfig::default(),
        streaming_service_config,
    );
    for (mut data_stream, mut stream_listener, time_service, transactions_only, _) in
        continuous_data_streams
    {
        // Initialize the data stream
        let mut global_data_summary = create_global_data_summary(1);
        initialize_data_requests(&mut data_stream, &global_data_summary);

        // Update the global data summary to be ahead of the subscription stream
        let highest_advertised_version = MAX_ADVERTISED_TRANSACTION + 1000;
        global_data_summary.advertised_data.synced_ledger_infos = vec![create_ledger_info(
            highest_advertised_version,
            MAX_ADVERTISED_EPOCH_END,
            false,
        )];

        // Set a valid response for the first subscription request and process it
        let highest_response_version = highest_advertised_version - 200; // Behind the advertised version
        set_new_data_response_in_queue(
            &mut data_stream,
            0,
            highest_response_version,
            transactions_only,
        );
        process_data_responses(&mut data_stream, &global_data_summary).await;
        assert_some!(stream_listener.select_next_some().now_or_never());

        // Verify the stream is now tracking the subscription lag
        let subscription_stream_lag = data_stream.get_subscription_stream_lag().unwrap();
        assert_eq!(subscription_stream_lag.start_time, time_service.now());
        assert_eq!(
            subscription_stream_lag.version_lag,
            highest_advertised_version - highest_response_version
        );

        // Elapse some time (but not enough for the stream to be killed)
        let time_service = time_service.into_mock();
        time_service.advance_secs(max_subscription_stream_lag_secs / 10);

        // Go through several iterations of being behind, but not lagging for too long
        for _ in 0..5 {
            // Update the global data summary to be further ahead (by 1)
            let highest_advertised_version = highest_advertised_version + 1;
            global_data_summary.advertised_data.synced_ledger_infos = vec![create_ledger_info(
                highest_advertised_version,
                MAX_ADVERTISED_EPOCH_END,
                false,
            )];

            // Elapse some time (but not enough for the stream to be killed)
            time_service.advance_secs(max_subscription_stream_lag_secs / 10);

            // Set a valid response for the first subscription request and process it
            let highest_response_version = highest_response_version + 1; // Still behind, but not worse
            set_new_data_response_in_queue(
                &mut data_stream,
                0,
                highest_response_version,
                transactions_only,
            );
            process_data_responses(&mut data_stream, &global_data_summary).await;
            assert_some!(stream_listener.select_next_some().now_or_never());

            // Verify the state of the subscription stream lag
            let subscription_stream_lag = data_stream.get_subscription_stream_lag().unwrap();
            assert_eq!(
                subscription_stream_lag.version_lag,
                highest_advertised_version - highest_response_version
            );
        }

        // Elapse enough time for the stream to be killed
        time_service.advance_secs(max_subscription_stream_lag_secs);

        // Set a valid response for the first subscription request and process it
        set_new_data_response_in_queue(
            &mut data_stream,
            0,
            highest_response_version - 1, // Even further behind the last iteration
            transactions_only,
        );
        process_data_responses(&mut data_stream, &global_data_summary).await;
        assert_some!(stream_listener.select_next_some().now_or_never());

        // Verify that the subscription stream lag has now been reset (the stream was killed)
        assert!(data_stream.get_subscription_stream_lag().is_none());
    }
}

#[tokio::test]
async fn test_continuous_stream_subscription_max() {
    // Create a dynamic prefetching config with prefetching disabled
    let dynamic_prefetching = DynamicPrefetchingConfig {
        enable_dynamic_prefetching: false,
        ..Default::default()
    };

    // Create a test streaming service config with subscriptions enabled
    let max_concurrent_requests = 3;
    let max_num_consecutive_subscriptions = 5;
    let streaming_service_config = DataStreamingServiceConfig {
        dynamic_prefetching,
        enable_subscription_streaming: true,
        max_concurrent_requests,
        max_num_consecutive_subscriptions,
        ..Default::default()
    };

    // Test all types of continuous data streams
    let continuous_data_streams = enumerate_continuous_data_streams(
        AptosDataClientConfig::default(),
        streaming_service_config,
    );
    for (mut data_stream, _stream_listener, _, transactions_only, allow_transactions_or_outputs) in
        continuous_data_streams
    {
        // Initialize the data stream
        let global_data_summary = create_global_data_summary(1);
        initialize_data_requests(&mut data_stream, &global_data_summary);

        // Iterate through several changes in subscription streams
        let num_subscription_stream_changes = 5;
        for stream_number in 0..num_subscription_stream_changes {
            // Fetch the subscription stream ID from the first pending request
            let subscription_stream_id = get_subscription_stream_id(&mut data_stream, 0);

            // Verify the pending requests are for the correct data and correctly formed
            verify_pending_subscription_requests(
                &mut data_stream,
                max_concurrent_requests,
                allow_transactions_or_outputs,
                transactions_only,
                0,
                subscription_stream_id,
                stream_number * max_num_consecutive_subscriptions,
            );

            // Set valid responses for all pending requests and process the responses
            for request_index in 0..max_concurrent_requests {
                set_new_data_response_in_queue(
                    &mut data_stream,
                    request_index as usize,
                    MAX_ADVERTISED_TRANSACTION + request_index,
                    transactions_only,
                );
            }
            process_data_responses(&mut data_stream, &global_data_summary).await;

            // Verify the number of pending requests
            verify_num_sent_requests(
                &mut data_stream,
                max_num_consecutive_subscriptions - max_concurrent_requests,
            );

            // Set valid responses for all pending requests and process the responses
            for request_index in 0..(max_num_consecutive_subscriptions - max_concurrent_requests) {
                set_new_data_response_in_queue(
                    &mut data_stream,
                    request_index as usize,
                    MAX_ADVERTISED_TRANSACTION + request_index + max_concurrent_requests,
                    transactions_only,
                );
            }
            process_data_responses(&mut data_stream, &global_data_summary).await;

            // Fetch the next subscription stream ID from the first pending request
            let next_subscription_stream_id = get_subscription_stream_id(&mut data_stream, 0);

            // Verify the subscription stream ID has changed (because we hit the max number of requests)
            assert_ne!(subscription_stream_id, next_subscription_stream_id);
        }
    }
}

#[tokio::test]
async fn test_continuous_stream_subscription_max_pending() {
    // Create a dynamic prefetching config with prefetching disabled
    let dynamic_prefetching = DynamicPrefetchingConfig {
        enable_dynamic_prefetching: false,
        ..Default::default()
    };

    // Create a test streaming service config with subscriptions enabled
    let max_concurrent_requests = 4;
    let max_num_consecutive_subscriptions = 1000;
    let max_pending_requests = 10;
    let streaming_service_config = DataStreamingServiceConfig {
        dynamic_prefetching,
        enable_subscription_streaming: true,
        max_concurrent_requests,
        max_num_consecutive_subscriptions,
        max_pending_requests,
        ..Default::default()
    };

    // Test all types of continuous data streams
    let continuous_data_streams = enumerate_continuous_data_streams(
        AptosDataClientConfig::default(),
        streaming_service_config,
    );
    for (mut data_stream, _stream_listener, _, transactions_only, allow_transactions_or_outputs) in
        continuous_data_streams
    {
        // Initialize the data stream
        let global_data_summary = create_global_data_summary(1);
        initialize_data_requests(&mut data_stream, &global_data_summary);

        // Fetch the subscription stream ID from the first pending request
        let subscription_stream_id = get_subscription_stream_id(&mut data_stream, 0);

        // Verify the pending requests are for the correct data and correctly formed
        verify_pending_subscription_requests(
            &mut data_stream,
            max_concurrent_requests,
            allow_transactions_or_outputs,
            transactions_only,
            0,
            subscription_stream_id,
            0,
        );

        // Set valid responses for all pending requests except the first
        for request_index in 1..max_concurrent_requests {
            set_new_data_response_in_queue(
                &mut data_stream,
                request_index as usize,
                MAX_ADVERTISED_TRANSACTION + request_index,
                transactions_only,
            );
        }

        // Process the responses
        process_data_responses(&mut data_stream, &global_data_summary).await;

        // Verify more requests are sent
        let num_pending_requests = (max_concurrent_requests * 2) - 1;
        verify_num_sent_requests(&mut data_stream, num_pending_requests);

        // Set valid responses for all pending requests except the first
        for request_index in 1..num_pending_requests {
            set_new_data_response_in_queue(
                &mut data_stream,
                request_index as usize,
                MAX_ADVERTISED_TRANSACTION + request_index,
                transactions_only,
            );
        }

        // Process the responses
        process_data_responses(&mut data_stream, &global_data_summary).await;

        // Verify more requests are sent (but not more than the max pending requests)
        verify_num_sent_requests(&mut data_stream, max_pending_requests);

        // Set responses and process them multiple times
        for _ in 0..10 {
            // Set valid responses for all pending requests except the first
            for request_index in 1..max_pending_requests {
                set_new_data_response_in_queue(
                    &mut data_stream,
                    request_index as usize,
                    MAX_ADVERTISED_TRANSACTION + request_index,
                    transactions_only,
                );
            }

            // Process the responses
            process_data_responses(&mut data_stream, &global_data_summary).await;

            // Verify more requests are sent (but not more than the max pending requests)
            verify_num_sent_requests(&mut data_stream, max_pending_requests);
        }
    }
}

#[tokio::test]
async fn test_continuous_stream_subscription_max_pending_prefetching() {
    // Create a dynamic prefetching config with prefetching enabled
    let max_in_flight_subscription_requests = 5;
    let dynamic_prefetching = DynamicPrefetchingConfig {
        enable_dynamic_prefetching: true,
        max_in_flight_subscription_requests,
        ..Default::default()
    };

    // Create a test streaming service config with subscriptions enabled
    let max_num_consecutive_subscriptions = 1000;
    let max_pending_requests = 11;
    let streaming_service_config = DataStreamingServiceConfig {
        dynamic_prefetching,
        enable_subscription_streaming: true,
        max_num_consecutive_subscriptions,
        max_pending_requests,
        ..Default::default()
    };

    // Test all types of continuous data streams
    let continuous_data_streams = enumerate_continuous_data_streams(
        AptosDataClientConfig::default(),
        streaming_service_config,
    );
    for (mut data_stream, _stream_listener, _, transactions_only, allow_transactions_or_outputs) in
        continuous_data_streams
    {
        // Initialize the data stream
        let global_data_summary = create_global_data_summary(1);
        initialize_data_requests(&mut data_stream, &global_data_summary);

        // Fetch the subscription stream ID from the first pending request
        let subscription_stream_id = get_subscription_stream_id(&mut data_stream, 0);

        // Verify the pending requests are for the correct data and correctly formed
        verify_pending_subscription_requests(
            &mut data_stream,
            max_in_flight_subscription_requests,
            allow_transactions_or_outputs,
            transactions_only,
            0,
            subscription_stream_id,
            0,
        );

        // Set valid responses for all pending requests except the first
        for request_index in 1..max_in_flight_subscription_requests {
            set_new_data_response_in_queue(
                &mut data_stream,
                request_index as usize,
                MAX_ADVERTISED_TRANSACTION + request_index,
                transactions_only,
            );
        }

        // Process the responses
        process_data_responses(&mut data_stream, &global_data_summary).await;

        // Verify more requests are sent
        let num_pending_requests = (max_in_flight_subscription_requests * 2) - 1;
        verify_num_sent_requests(&mut data_stream, num_pending_requests);

        // Set valid responses for all pending requests except the first
        for request_index in 1..num_pending_requests {
            set_new_data_response_in_queue(
                &mut data_stream,
                request_index as usize,
                MAX_ADVERTISED_TRANSACTION + request_index,
                transactions_only,
            );
        }

        // Process the responses
        process_data_responses(&mut data_stream, &global_data_summary).await;

        // Verify more requests are sent (but not more than the max pending requests)
        verify_num_sent_requests(&mut data_stream, max_pending_requests);

        // Set responses and process them multiple times
        for _ in 0..10 {
            // Set valid responses for all pending requests except the first
            for request_index in 1..max_pending_requests {
                set_new_data_response_in_queue(
                    &mut data_stream,
                    request_index as usize,
                    MAX_ADVERTISED_TRANSACTION + request_index,
                    transactions_only,
                );
            }

            // Process the responses
            process_data_responses(&mut data_stream, &global_data_summary).await;

            // Verify more requests are sent (but not more than the max pending requests)
            verify_num_sent_requests(&mut data_stream, max_pending_requests);
        }
    }
}

#[tokio::test]
async fn test_continuous_stream_subscription_max_prefetching() {
    // Create a dynamic prefetching config with prefetching enabled
    let max_in_flight_subscription_requests = 8;
    let dynamic_prefetching = DynamicPrefetchingConfig {
        enable_dynamic_prefetching: true,
        max_in_flight_subscription_requests,
        ..Default::default()
    };

    // Create a test streaming service config with subscriptions enabled
    let max_num_consecutive_subscriptions = 9;
    let streaming_service_config = DataStreamingServiceConfig {
        dynamic_prefetching,
        enable_subscription_streaming: true,
        max_num_consecutive_subscriptions,
        ..Default::default()
    };

    // Test all types of continuous data streams
    let continuous_data_streams = enumerate_continuous_data_streams(
        AptosDataClientConfig::default(),
        streaming_service_config,
    );
    for (mut data_stream, _stream_listener, _, transactions_only, allow_transactions_or_outputs) in
        continuous_data_streams
    {
        // Initialize the data stream
        let global_data_summary = create_global_data_summary(1);
        initialize_data_requests(&mut data_stream, &global_data_summary);

        // Iterate through several changes in subscription streams
        let num_subscription_stream_changes = 5;
        for stream_number in 0..num_subscription_stream_changes {
            // Fetch the subscription stream ID from the first pending request
            let subscription_stream_id = get_subscription_stream_id(&mut data_stream, 0);

            // Verify the pending requests are for the correct data and correctly formed
            verify_pending_subscription_requests(
                &mut data_stream,
                max_in_flight_subscription_requests,
                allow_transactions_or_outputs,
                transactions_only,
                0,
                subscription_stream_id,
                stream_number * max_num_consecutive_subscriptions,
            );

            // Set valid responses for all pending requests and process the responses
            for request_index in 0..max_in_flight_subscription_requests {
                set_new_data_response_in_queue(
                    &mut data_stream,
                    request_index as usize,
                    MAX_ADVERTISED_TRANSACTION + request_index,
                    transactions_only,
                );
            }
            process_data_responses(&mut data_stream, &global_data_summary).await;

            // Verify the number of pending requests
            verify_num_sent_requests(
                &mut data_stream,
                max_num_consecutive_subscriptions - max_in_flight_subscription_requests,
            );

            // Set valid responses for all pending requests and process the responses
            for request_index in
                0..(max_num_consecutive_subscriptions - max_in_flight_subscription_requests)
            {
                set_new_data_response_in_queue(
                    &mut data_stream,
                    request_index as usize,
                    MAX_ADVERTISED_TRANSACTION
                        + request_index
                        + max_in_flight_subscription_requests,
                    transactions_only,
                );
            }
            process_data_responses(&mut data_stream, &global_data_summary).await;

            // Fetch the next subscription stream ID from the first pending request
            let next_subscription_stream_id = get_subscription_stream_id(&mut data_stream, 0);

            // Verify the subscription stream ID has changed (because we hit the max number of requests)
            assert_ne!(subscription_stream_id, next_subscription_stream_id);
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_continuous_stream_subscription_timeout() {
    // Create a test data client config
    let data_client_config = AptosDataClientConfig {
        subscription_response_timeout_ms: 2022,
        ..Default::default()
    };

    // Create a dynamic prefetching config with prefetching disabled
    let dynamic_prefetching = DynamicPrefetchingConfig {
        enable_dynamic_prefetching: false,
        ..Default::default()
    };

    // Create a test streaming service config with subscriptions enabled
    let max_concurrent_requests = 3;
    let streaming_service_config = DataStreamingServiceConfig {
        dynamic_prefetching,
        enable_subscription_streaming: true,
        max_concurrent_requests,
        ..Default::default()
    };

    // Verify the timeouts of all continuous data streams
    verify_continuous_stream_request_timeouts(
        data_client_config,
        streaming_service_config,
        max_concurrent_requests,
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_continuous_stream_subscription_timeout_prefetching() {
    // Create a test data client config
    let data_client_config = AptosDataClientConfig {
        subscription_response_timeout_ms: 500,
        ..Default::default()
    };

    // Create a dynamic prefetching config with prefetching enabled
    let max_in_flight_subscription_requests = 6;
    let dynamic_prefetching = DynamicPrefetchingConfig {
        enable_dynamic_prefetching: true,
        max_in_flight_subscription_requests,
        ..Default::default()
    };

    // Create a test streaming service config with subscriptions enabled
    let streaming_service_config = DataStreamingServiceConfig {
        dynamic_prefetching,
        enable_subscription_streaming: true,
        ..Default::default()
    };

    // Verify the timeouts of all continuous data streams
    verify_continuous_stream_request_timeouts(
        data_client_config,
        streaming_service_config,
        max_in_flight_subscription_requests,
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_stream_timeouts() {
    // Create a test data client config
    let max_response_timeout_ms = 85;
    let response_timeout_ms = 7;
    let data_client_config = AptosDataClientConfig {
        max_response_timeout_ms,
        response_timeout_ms,
        ..Default::default()
    };

    // Create a test streaming service config with dynamic prefetching disabled
    let max_concurrent_requests = 3;
    let max_request_retry = 10;
    let dynamic_prefetching_config = DynamicPrefetchingConfig {
        enable_dynamic_prefetching: false,
        ..Default::default()
    };
    let streaming_service_config = DataStreamingServiceConfig {
        dynamic_prefetching: dynamic_prefetching_config,
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
        verify_num_sent_requests(&mut data_stream, max_concurrent_requests);

        // Wait for the data client to satisfy all requests
        for request_index in 0..max_concurrent_requests as usize {
            wait_for_data_client_to_respond(&mut data_stream, request_index).await;
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
        for request_index in 0..max_concurrent_requests as usize {
            wait_for_data_client_to_respond(&mut data_stream, request_index).await;
        }

        // Set a timeout on the second request
        set_timeout_response_in_queue(&mut data_stream, 1);

        // Handle multiple invalid type responses on the first request
        for _ in 0..max_request_retry / 2 {
            set_state_value_response_in_queue(&mut data_stream, 0, 0, 0);
            process_data_responses(&mut data_stream, &global_data_summary).await;
            wait_for_data_client_to_respond(&mut data_stream, 0).await;
        }

        // Handle multiple invalid type responses on the third request
        for _ in 0..max_request_retry / 2 {
            set_state_value_response_in_queue(&mut data_stream, 2, 2, 2);
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

#[tokio::test(flavor = "multi_thread")]
async fn test_stream_timeouts_dynamic() {
    // Create a test data client config
    let max_response_timeout_ms = 85;
    let response_timeout_ms = 7;
    let data_client_config = AptosDataClientConfig {
        max_response_timeout_ms,
        response_timeout_ms,
        ..Default::default()
    };

    // Create a dynamic prefetching config with prefetching enabled
    let initial_prefetching_value = 5;
    let min_prefetching_value = 3;
    let dynamic_prefetching_config = DynamicPrefetchingConfig {
        enable_dynamic_prefetching: true,
        initial_prefetching_value,
        min_prefetching_value,
        ..Default::default()
    };

    // Create a test streaming service config with dynamic prefetching disabled
    let max_request_retry = 10;
    let streaming_service_config = DataStreamingServiceConfig {
        dynamic_prefetching: dynamic_prefetching_config,
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
        verify_num_sent_requests(&mut data_stream, initial_prefetching_value);

        // Wait for the data client to satisfy all requests
        for request_index in 0..initial_prefetching_value as usize {
            wait_for_data_client_to_respond(&mut data_stream, request_index).await;
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
        for request_index in 0..min_prefetching_value as usize {
            wait_for_data_client_to_respond(&mut data_stream, request_index).await;
        }

        // Set a timeout on the second request
        set_timeout_response_in_queue(&mut data_stream, 1);

        // Handle multiple invalid type responses on the first request
        for _ in 0..max_request_retry / 2 {
            set_state_value_response_in_queue(&mut data_stream, 0, 0, 0);
            process_data_responses(&mut data_stream, &global_data_summary).await;
            wait_for_data_client_to_respond(&mut data_stream, 0).await;
        }

        // Handle multiple invalid type responses on the third request
        for _ in 0..max_request_retry / 2 {
            set_state_value_response_in_queue(&mut data_stream, 2, 2, 2);
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

/// Advertises new data (beyond the highest advertised data) and verifies
/// that data client requests are sent to fetch the missing data.
async fn advertise_new_data_and_verify_requests(
    data_stream: &mut DataStream<MockAptosDataClient>,
    global_data_summary: GlobalDataSummary,
    transactions_only: bool,
    allow_transactions_or_outputs: bool,
    max_concurrent_requests: u64,
) {
    // Advertise new data beyond the currently advertised data
    let mut new_global_data_summary = global_data_summary.clone();
    let new_highest_synced_version = MAX_ADVERTISED_TRANSACTION + 1000;
    new_global_data_summary.advertised_data.synced_ledger_infos = vec![create_ledger_info(
        new_highest_synced_version,
        MAX_ADVERTISED_EPOCH_END,
        false,
    )];

    // Set a timeout response at the head of the queue and process the response
    set_timeout_response_in_queue(data_stream, 0);
    process_data_responses(data_stream, &new_global_data_summary).await;

    // Verify multiple data requests have now been sent to fetch the missing data
    verify_num_sent_requests(data_stream, max_concurrent_requests);

    // Verify the pending requests are for the correct data and correctly formed
    for request_index in 0..max_concurrent_requests {
        let client_request = get_pending_client_request(data_stream, request_index as usize);
        let expected_version = MAX_ADVERTISED_TRANSACTION + 2 + request_index;
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
    let (data_stream, data_stream_listener, _) =
        create_data_stream(data_client_config, streaming_service_config, stream_request);

    (data_stream, data_stream_listener)
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
    let (data_stream, data_stream_listener, _) =
        create_data_stream(data_client_config, streaming_service_config, stream_request);

    (data_stream, data_stream_listener)
}

/// Creates a continuous transaction output stream for the given `version`.
fn create_continuous_transaction_output_stream(
    data_client_config: AptosDataClientConfig,
    streaming_service_config: DataStreamingServiceConfig,
    known_version: Version,
    known_epoch: Version,
) -> (
    DataStream<MockAptosDataClient>,
    DataStreamListener,
    TimeService,
) {
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
) -> (
    DataStream<MockAptosDataClient>,
    DataStreamListener,
    TimeService,
) {
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
) -> (
    DataStream<MockAptosDataClient>,
    DataStreamListener,
    TimeService,
) {
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
    let (data_stream, data_stream_listener, _) =
        create_data_stream(data_client_config, streaming_service_config, stream_request);

    (data_stream, data_stream_listener)
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
    let (data_stream, data_stream_listener, _) =
        create_data_stream(data_client_config, streaming_service_config, stream_request);

    (data_stream, data_stream_listener)
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
    let (data_stream, data_stream_listener, _) =
        create_data_stream(data_client_config, streaming_service_config, stream_request);

    (data_stream, data_stream_listener)
}

fn create_data_stream(
    data_client_config: AptosDataClientConfig,
    streaming_service_config: DataStreamingServiceConfig,
    stream_request: StreamRequest,
) -> (
    DataStream<MockAptosDataClient>,
    DataStreamListener,
    TimeService,
) {
    initialize_logger();

    // Create an advertised data
    let advertised_data = create_advertised_data();

    // Create an aptos data client mock and notification generator
    let aptos_data_client = MockAptosDataClient::new(data_client_config, true, false, true, false);
    let notification_generator = Arc::new(U64IdGenerator::new());

    // Create the data stream and listener pair
    let time_service = TimeService::mock();
    let (data_stream, data_stream_listener) = DataStream::new(
        data_client_config,
        streaming_service_config,
        create_random_u64(10000),
        &stream_request,
        create_stream_update_notifier(),
        aptos_data_client,
        notification_generator,
        &advertised_data,
        time_service.clone(),
    )
    .unwrap();

    (data_stream, data_stream_listener, time_service)
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

/// Creates a returns a new stream update notifier (dropping the listener)
fn create_stream_update_notifier() -> aptos_channel::Sender<(), StreamUpdateNotification> {
    let (stream_update_notifier, _) = aptos_channel::new(QueueStyle::LIFO, 1, None);
    stream_update_notifier
}

/// A utility function that creates and returns all types of
/// continuous data streams. This is useful for tests that verify
/// all stream types.
fn enumerate_continuous_data_streams(
    data_client_config: AptosDataClientConfig,
    streaming_service_config: DataStreamingServiceConfig,
) -> Vec<(
    DataStream<MockAptosDataClient>,
    DataStreamListener,
    TimeService,
    bool,
    bool,
)> {
    let mut continuous_data_streams = vec![];

    // Create a continuous transaction stream
    let transactions_only = true;
    let allow_transactions_or_outputs = false;
    let (data_stream, stream_listener, time_service) = create_continuous_transaction_stream(
        data_client_config,
        streaming_service_config,
        MAX_ADVERTISED_TRANSACTION,
        MAX_ADVERTISED_EPOCH_END,
    );
    continuous_data_streams.push((
        data_stream,
        stream_listener,
        time_service,
        transactions_only,
        allow_transactions_or_outputs,
    ));

    // Create a continuous transaction output stream
    let transactions_only = false;
    let allow_transactions_or_outputs = false;
    let (data_stream, stream_listener, time_service) = create_continuous_transaction_output_stream(
        data_client_config,
        streaming_service_config,
        MAX_ADVERTISED_TRANSACTION_OUTPUT,
        MAX_ADVERTISED_EPOCH_END,
    );
    continuous_data_streams.push((
        data_stream,
        stream_listener,
        time_service,
        transactions_only,
        allow_transactions_or_outputs,
    ));

    // Create a continuous transaction or output stream
    let transactions_only = false;
    let allow_transactions_or_outputs = true;
    let (data_stream, stream_listener, time_service) =
        create_continuous_transaction_or_output_stream(
            data_client_config,
            streaming_service_config,
            MAX_ADVERTISED_TRANSACTION_OUTPUT,
            MAX_ADVERTISED_EPOCH_END,
        );
    continuous_data_streams.push((
        data_stream,
        stream_listener,
        time_service,
        transactions_only,
        allow_transactions_or_outputs,
    ));

    continuous_data_streams
}

/// Sets an epoch ending response in the queue at the given set of indices.
/// For indices that are not specified, either an error response is set,
/// or nothing (to emulate the request failing or still being in-flight).
fn set_epoch_ending_response_for_indices(
    data_stream: &mut DataStream<MockAptosDataClient>,
    max_queue_length: u64,
    indices: Vec<u64>,
) {
    for index in 0..max_queue_length {
        let random_number = create_random_u64(100);
        if indices.contains(&index) {
            set_epoch_ending_response_in_queue(data_stream, index as usize, 0); // Set a valid response
        } else if random_number % 3 == 0 {
            set_timeout_response_in_queue(data_stream, index as usize); // Set a timeout response
        } else if random_number % 3 == 1 {
            set_failure_response_in_queue(data_stream, index as usize); // Set a failure response
        } else {
            set_pending_response_in_queue(data_stream, index as usize); // Set a pending response
        }
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
    first_state_value_index: u64,
    last_state_value_index: u64,
    request_index: usize,
) {
    let (sent_requests, _) = data_stream.get_sent_requests_and_notifications();
    let pending_response = sent_requests
        .as_mut()
        .unwrap()
        .get_mut(request_index)
        .unwrap();
    let client_response = Some(Ok(create_data_client_response(
        ResponsePayload::StateValuesWithProof(StateValueChunkWithProof {
            first_index: first_state_value_index,
            last_index: last_state_value_index,
            first_key: Default::default(),
            last_key: Default::default(),
            raw_values: vec![(StateKey::raw(&[]), StateValue::new_legacy(vec![].into()))],
            proof: SparseMerkleRangeProof::new(vec![]),
            root_hash: Default::default(),
        }),
    )));
    pending_response.lock().client_response = client_response;
}

/// Sets the client response at the index in the pending
/// queue to contain new data.
fn set_new_data_response_in_queue(
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

/// Sets the client response at the index in the pending queue to contain a failure
fn set_failure_response_in_queue(data_stream: &mut DataStream<MockAptosDataClient>, index: usize) {
    set_response_in_queue(
        data_stream,
        index,
        aptos_data_client::error::Error::UnexpectedErrorEncountered("Oops!".into()),
    );
}

/// Sets the client response at the index in the pending
/// queue to contain a pending response.
fn set_pending_response_in_queue(data_stream: &mut DataStream<MockAptosDataClient>, index: usize) {
    // Get the pending response at the specified index
    let (sent_requests, _) = data_stream.get_sent_requests_and_notifications();
    let pending_response = sent_requests.as_mut().unwrap().get_mut(index).unwrap();

    // Set the response to still be pending
    pending_response.lock().client_response = None;
}

/// Sets the client response at the index in the pending
/// queue to contain a timeout response.
fn set_timeout_response_in_queue(data_stream: &mut DataStream<MockAptosDataClient>, index: usize) {
    set_response_in_queue(
        data_stream,
        index,
        aptos_data_client::error::Error::TimeoutWaitingForResponse("Timed out!".into()),
    );
}

/// Sets the given error response at the index in the pending queue
fn set_response_in_queue(
    data_stream: &mut DataStream<MockAptosDataClient>,
    index: usize,
    error_response: aptos_data_client::error::Error,
) {
    // Get the pending response at the specified index
    let (sent_requests, _) = data_stream.get_sent_requests_and_notifications();
    let pending_response = sent_requests.as_mut().unwrap().get_mut(index).unwrap();

    // Set the response
    pending_response.lock().client_response = Some(Err(error_response));
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
    data_stream.clear_sent_data_requests_queue();

    // Insert the pending response
    let (sent_requests, _) = data_stream.get_sent_requests_and_notifications();
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

/// Verifies the timeouts of all continuous data stream requests
/// in the presence of RPC timeouts and failures.
async fn verify_continuous_stream_request_timeouts(
    data_client_config: AptosDataClientConfig,
    streaming_service_config: DataStreamingServiceConfig,
    num_expected_requests: u64,
) {
    // Test all types of continuous data streams
    let continuous_data_streams =
        enumerate_continuous_data_streams(data_client_config, streaming_service_config);
    for (
        mut data_stream,
        mut stream_listener,
        _,
        transactions_only,
        allow_transactions_or_outputs,
    ) in continuous_data_streams
    {
        // Initialize the data stream
        let global_data_summary = create_global_data_summary(1);
        initialize_data_requests(&mut data_stream, &global_data_summary);

        // Verify that the expected number of requests are made
        verify_num_sent_requests(&mut data_stream, num_expected_requests);

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

        // Handle multiple timeouts and retries because no new data is known,
        // so the best we can do is resend the same requests.
        for _ in 0..3 {
            // Set a timeout response for the subscription request and process it
            set_timeout_response_in_queue(&mut data_stream, 0);
            process_data_responses(&mut data_stream, &global_data_summary).await;

            // Verify more requests are made
            verify_num_sent_requests(&mut data_stream, num_expected_requests);
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
    pending_response.lock().client_request.clone()
}

/// Returns the subscription stream ID from the pending client request at the given index
fn get_subscription_stream_id(
    data_stream: &mut DataStream<MockAptosDataClient>,
    index: usize,
) -> u64 {
    // Get the pending client request
    let client_request = get_pending_client_request(data_stream, index);

    // Extract the subscription stream ID from the request
    match client_request {
        DataClientRequest::SubscribeTransactionsOrOutputsWithProof(request) => {
            request.subscription_stream_id
        },
        DataClientRequest::SubscribeTransactionsWithProof(request) => {
            request.subscription_stream_id
        },
        DataClientRequest::SubscribeTransactionOutputsWithProof(request) => {
            request.subscription_stream_id
        },
        _ => panic!("Unexpected client request type found! {:?}", client_request),
    }
}

/// Verifies that the length of the pending requests queue is
/// equal to the expected length.
fn verify_num_sent_requests(
    data_stream: &mut DataStream<MockAptosDataClient>,
    expected_length: u64,
) {
    // Get the number of sent requests
    let (sent_requests, _) = data_stream.get_sent_requests_and_notifications();
    let num_sent_requests = sent_requests.as_ref().unwrap().len() as u64;

    // Verify the number of sent requests
    assert_eq!(num_sent_requests, expected_length);
}

/// Verifies that a single pending optimistic fetch exists and
/// that it is for the correct data.
fn verify_pending_optimistic_fetch(
    data_stream: &mut DataStream<MockAptosDataClient>,
    transactions_only: bool,
    allow_transactions_or_outputs: bool,
    known_version_offset: u64,
) {
    // Verify a single request is pending
    verify_num_sent_requests(data_stream, 1);

    // Verify the request is for the correct data
    let client_request = get_pending_client_request(data_stream, 0);
    let expected_request = if allow_transactions_or_outputs {
        DataClientRequest::NewTransactionsOrOutputsWithProof(
            NewTransactionsOrOutputsWithProofRequest {
                known_version: MAX_ADVERTISED_TRANSACTION_OUTPUT + known_version_offset,
                known_epoch: MAX_ADVERTISED_EPOCH_END,
                include_events: false,
            },
        )
    } else if transactions_only {
        DataClientRequest::NewTransactionsWithProof(NewTransactionsWithProofRequest {
            known_version: MAX_ADVERTISED_TRANSACTION + known_version_offset,
            known_epoch: MAX_ADVERTISED_EPOCH_END,
            include_events: false,
        })
    } else {
        DataClientRequest::NewTransactionOutputsWithProof(NewTransactionOutputsWithProofRequest {
            known_version: MAX_ADVERTISED_TRANSACTION_OUTPUT + known_version_offset,
            known_epoch: MAX_ADVERTISED_EPOCH_END,
        })
    };
    assert_eq!(client_request, expected_request);
}

/// Verifies that the pending requests are fulfilled for the specified indices
fn verify_pending_responses_for_indices(
    data_stream: &mut DataStream<MockAptosDataClient>,
    max_queue_length: u64,
    indices: Vec<u64>,
) {
    // Get the sent requests queue
    let (sent_requests, _) = data_stream.get_sent_requests_and_notifications();

    // Verify the client responses for the specified indices
    for index in 0..max_queue_length {
        // Get the client response
        let sent_request = sent_requests.as_ref().unwrap().get(index as usize);
        let pending_client_response = sent_request.unwrap().lock();
        let client_response = pending_client_response.client_response.as_ref();

        // Verify the client response
        if indices.contains(&index) {
            // Verify the response is present
            assert_some!(client_response);
        } else {
            // Otherwise, if a response is present, it must be an error
            if let Some(client_response) = client_response {
                assert_err!(client_response);
            }
        }
    }
}

/// Verifies that the pending subscription requests are well formed
/// and for the correct data.
fn verify_pending_subscription_requests(
    data_stream: &mut DataStream<MockAptosDataClient>,
    max_concurrent_requests: u64,
    allow_transactions_or_outputs: bool,
    transactions_only: bool,
    starting_stream_index: u64,
    subscription_stream_id: u64,
    known_version_offset: u64,
) {
    // Verify the correct number of pending requests
    verify_num_sent_requests(data_stream, max_concurrent_requests);

    // Verify the pending requests are for the correct data and correctly formed
    for request_index in 0..max_concurrent_requests {
        let client_request = get_pending_client_request(data_stream, request_index as usize);
        let expected_request = if allow_transactions_or_outputs {
            DataClientRequest::SubscribeTransactionsOrOutputsWithProof(
                SubscribeTransactionsOrOutputsWithProofRequest {
                    known_version: MAX_ADVERTISED_TRANSACTION_OUTPUT + known_version_offset,
                    known_epoch: MAX_ADVERTISED_EPOCH_END,
                    subscription_stream_index: starting_stream_index + request_index,
                    include_events: false,
                    subscription_stream_id,
                },
            )
        } else if transactions_only {
            DataClientRequest::SubscribeTransactionsWithProof(
                SubscribeTransactionsWithProofRequest {
                    known_version: MAX_ADVERTISED_TRANSACTION + known_version_offset,
                    known_epoch: MAX_ADVERTISED_EPOCH_END,
                    subscription_stream_index: starting_stream_index + request_index,
                    include_events: false,
                    subscription_stream_id,
                },
            )
        } else {
            DataClientRequest::SubscribeTransactionOutputsWithProof(
                SubscribeTransactionOutputsWithProofRequest {
                    known_version: MAX_ADVERTISED_TRANSACTION_OUTPUT + known_version_offset,
                    known_epoch: MAX_ADVERTISED_EPOCH_END,
                    subscription_stream_index: starting_stream_index + request_index,
                    subscription_stream_id,
                },
            )
        };
        assert_eq!(client_request, expected_request);
    }
}

/// Verifies a notification along the given listener and
/// continues to drive progress until one is received.
async fn wait_for_notification_and_verify(
    data_stream: &mut DataStream<MockAptosDataClient>,
    stream_listener: &mut DataStreamListener,
    transaction_syncing: bool,
    allow_transactions_or_outputs: bool,
    new_data_notification: bool,
    global_data_summary: &GlobalDataSummary,
) {
    loop {
        if let Ok(data_notification) =
            timeout(Duration::from_secs(1), stream_listener.select_next_some()).await
        {
            if new_data_notification {
                // Verify we got the correct new data
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

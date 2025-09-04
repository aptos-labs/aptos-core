// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    stream_engine::{DataStreamEngine, StreamEngine},
    streaming_client::{
        ContinuouslyStreamTransactionOutputsRequest, GetAllEpochEndingLedgerInfosRequest,
        GetAllStatesRequest, GetAllTransactionsRequest, StreamRequest,
    },
    tests::utils::create_ledger_info,
};
use velor_config::config::{DataStreamingServiceConfig, DynamicPrefetchingConfig};
use velor_data_client::global_summary::GlobalDataSummary;
use velor_id_generator::U64IdGenerator;
use velor_storage_service_types::responses::CompleteDataRange;
use std::sync::Arc;

#[test]
fn create_client_requests_epoch_ending_stream() {
    // Create an epoch ending stream request
    let start_epoch = 100;
    let stream_request =
        StreamRequest::GetAllEpochEndingLedgerInfos(GetAllEpochEndingLedgerInfosRequest {
            start_epoch,
        });

    // Create a global data summary with a single epoch range
    let end_epoch = 1000;
    let mut global_data_summary = GlobalDataSummary::empty();
    global_data_summary
        .advertised_data
        .epoch_ending_ledger_infos = vec![CompleteDataRange::new(start_epoch, end_epoch).unwrap()];
    global_data_summary.optimal_chunk_sizes.epoch_chunk_size = 1;

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

    // Verify that client requests are bound by the appropriate limits
    verify_data_client_requests(&mut global_data_summary, &mut stream_engine);
}

#[test]
fn create_client_requests_state_values_stream() {
    // Create a state values stream request
    let version = 100;
    let start_index = 0;
    let stream_request = StreamRequest::GetAllStates(GetAllStatesRequest {
        version,
        start_index,
    });

    // Create a global data summary with a single state range
    let mut global_data_summary = GlobalDataSummary::empty();
    global_data_summary.advertised_data.states =
        vec![CompleteDataRange::new(start_index, start_index).unwrap()];
    global_data_summary.optimal_chunk_sizes.state_chunk_size = 1;

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
    stream_engine.number_of_states = Some(1_000_000);

    // Verify that client requests are bound by the appropriate limits
    verify_data_client_requests(&mut global_data_summary, &mut stream_engine);
}

#[test]
fn create_client_requests_transaction_stream() {
    // Create a transactions stream request
    let start_version = 0;
    let end_version = 1_000_000;
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
        .transaction_chunk_size = 1;

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

    // Verify that client requests are bound by the appropriate limits
    verify_data_client_requests(&mut global_data_summary, &mut stream_engine);
}

#[test]
fn create_client_requests_continuous_output_stream() {
    // Create a continuous outputs stream request
    let known_version = 1;
    let known_epoch = 1;
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
        vec![CompleteDataRange::new(0, 1_000_000).unwrap()];
    global_data_summary
        .optimal_chunk_sizes
        .transaction_output_chunk_size = 1;

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
    stream_engine.current_target_ledger_info = Some(create_ledger_info(
        known_version + 500_000,
        known_epoch,
        false,
    ));

    // Verify that client requests are bound by the appropriate limits
    verify_data_client_requests(&mut global_data_summary, &mut stream_engine)
}

#[test]
fn create_client_requests_continuous_output_stream_optimistic_fetch() {
    // Create a data streaming service with subscriptions disabled
    let streaming_service_config = DataStreamingServiceConfig {
        enable_subscription_streaming: false,
        ..Default::default()
    };

    // Create a continuous outputs stream request
    let known_version = 100;
    let known_epoch = 10;
    let stream_request = StreamRequest::ContinuouslyStreamTransactionOutputs(
        ContinuouslyStreamTransactionOutputsRequest {
            known_version,
            known_epoch,
            target: None,
        },
    );

    // Create a global data summary at the highest known version
    let mut global_data_summary = GlobalDataSummary::empty();
    global_data_summary.advertised_data.transaction_outputs =
        vec![CompleteDataRange::new(0, known_version).unwrap()];
    global_data_summary.advertised_data.synced_ledger_infos =
        vec![create_ledger_info(known_version, known_epoch, false)];

    // Create a new continuous outputs stream engine
    let mut stream_engine = match StreamEngine::new(
        streaming_service_config,
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

    // Set the next request version and epoch to the known version and epoch
    stream_engine.next_request_version_and_epoch = (known_version + 1, known_epoch);

    // Create client requests and verify that a single request is always returned
    for max_number_of_requests in 0..10 {
        // Create and verify the client requests
        let client_requests = stream_engine
            .create_data_client_requests(
                max_number_of_requests,
                1_000_000, // Allow a large number of in-flight requests
                0,
                &global_data_summary,
                create_notification_id_generator(),
            )
            .unwrap();
        assert_eq!(client_requests.len() as u64, 1);

        // Reset the pending optimistic fetch request flag
        stream_engine.optimistic_fetch_requested = false;
    }
}

#[test]
fn create_client_requests_continuous_output_stream_subscriptions() {
    // Create a dynamic prefetching config with prefetching disabled
    let dynamic_prefetching = DynamicPrefetchingConfig {
        enable_dynamic_prefetching: false,
        ..Default::default()
    };

    // Create a data streaming service with subscriptions enabled
    let streaming_service_config = DataStreamingServiceConfig {
        dynamic_prefetching,
        enable_subscription_streaming: true,
        max_num_consecutive_subscriptions: 1_000_000, // Allow a large number of subscriptions
        max_concurrent_state_requests: 1_000_000,     // Allow a large number of in-flight requests
        max_pending_requests: 1_000_000,              // Allow a large number of pending requests
        ..Default::default()
    };

    // Create a continuous outputs stream request
    let known_version = 100;
    let known_epoch = 10;
    let stream_request = StreamRequest::ContinuouslyStreamTransactionOutputs(
        ContinuouslyStreamTransactionOutputsRequest {
            known_version,
            known_epoch,
            target: None,
        },
    );

    // Create a global data summary at the highest known version
    let mut global_data_summary = GlobalDataSummary::empty();
    global_data_summary.advertised_data.transaction_outputs =
        vec![CompleteDataRange::new(0, known_version).unwrap()];
    global_data_summary.advertised_data.synced_ledger_infos =
        vec![create_ledger_info(known_version, known_epoch, false)];

    // Create a new continuous outputs stream engine
    let mut stream_engine = match StreamEngine::new(
        streaming_service_config,
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

    // Set the next request version and epoch to the known version and epoch
    stream_engine.next_request_version_and_epoch = (known_version + 1, known_epoch);

    // Verify that client requests are bound by the appropriate limits
    verify_data_client_requests(&mut global_data_summary, &mut stream_engine)
}

#[test]
fn create_client_requests_continuous_output_stream_prefetching() {
    // Create a dynamic prefetching config with prefetching enabled
    let max_in_flight_subscription_requests = 20;
    let dynamic_prefetching = DynamicPrefetchingConfig {
        enable_dynamic_prefetching: true,
        max_in_flight_subscription_requests,
        ..Default::default()
    };

    // Create a data streaming service with subscriptions enabled
    let streaming_service_config = DataStreamingServiceConfig {
        dynamic_prefetching,
        enable_subscription_streaming: true,
        max_num_consecutive_subscriptions: 1_000_000, // Allow a large number of subscriptions
        max_pending_requests: 1_000_000,              // Allow a large number of pending requests
        ..Default::default()
    };

    // Create a continuous outputs stream request
    let known_version = 100;
    let known_epoch = 10;
    let stream_request = StreamRequest::ContinuouslyStreamTransactionOutputs(
        ContinuouslyStreamTransactionOutputsRequest {
            known_version,
            known_epoch,
            target: None,
        },
    );

    // Create a global data summary at the highest known version
    let mut global_data_summary = GlobalDataSummary::empty();
    global_data_summary.advertised_data.transaction_outputs =
        vec![CompleteDataRange::new(0, known_version).unwrap()];
    global_data_summary.advertised_data.synced_ledger_infos =
        vec![create_ledger_info(known_version, known_epoch, false)];

    // Create a new continuous outputs stream engine
    let mut stream_engine = match StreamEngine::new(
        streaming_service_config,
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

    // Set the next request version and epoch to the known version and epoch
    stream_engine.next_request_version_and_epoch = (known_version + 1, known_epoch);

    // Create client requests and verify they are bound by the maximum number of requests
    for max_number_of_requests in 0..15 {
        let client_requests = stream_engine
            .create_data_client_requests(
                max_number_of_requests,
                1_000_000, // Allow a large number of in-flight requests
                0,
                &global_data_summary,
                create_notification_id_generator(),
            )
            .unwrap();
        assert_eq!(client_requests.len(), max_number_of_requests as usize);
    }

    // Create client requests and verify that the number of in-flight requests is
    // bound by the maximum defined in the dynamic prefetching config.
    for max_in_flight_requests in 0..15 {
        for num_in_flight_requests in 0..15 {
            let client_requests = stream_engine
                .create_data_client_requests(
                    1_000_000, // Allow a large number of maximum requests
                    max_in_flight_requests,
                    num_in_flight_requests,
                    &global_data_summary,
                    create_notification_id_generator(),
                )
                .unwrap();
            println!(
                "max_in_flight_requests: {}, num_in_flight_requests: {}",
                max_in_flight_requests, num_in_flight_requests
            );
            assert_eq!(
                client_requests.len() as u64,
                max_in_flight_subscription_requests.saturating_sub(num_in_flight_requests)
            );
        }
    }
}

/// Returns a simple notification ID generator for testing purposes
fn create_notification_id_generator() -> Arc<U64IdGenerator> {
    Arc::new(U64IdGenerator::new())
}

/// Verifies that the created client requests are bound by the maximum number of
/// requests and the maximum number of in-flight requests (when specified).
fn verify_data_client_requests<T: DataStreamEngine>(
    global_data_summary: &mut GlobalDataSummary,
    stream_engine: &mut T,
) {
    // Create client requests and verify they are bound by the maximum number of requests
    for max_number_of_requests in 0..10 {
        let client_requests = stream_engine
            .create_data_client_requests(
                max_number_of_requests,
                1_000_000, // Allow a large number of in-flight requests
                0,
                global_data_summary,
                create_notification_id_generator(),
            )
            .unwrap();
        assert_eq!(client_requests.len(), max_number_of_requests as usize);
    }

    // Create client requests and verify they are bound by the maximum number of in-flight requests
    for max_in_flight_requests in 0..10 {
        for num_in_flight_requests in 0..15 {
            let client_requests = stream_engine
                .create_data_client_requests(
                    1_000_000, // Allow a large number of maximum requests
                    max_in_flight_requests,
                    num_in_flight_requests,
                    global_data_summary,
                    create_notification_id_generator(),
                )
                .unwrap();
            assert_eq!(
                client_requests.len() as u64,
                max_in_flight_requests.saturating_sub(num_in_flight_requests)
            );
        }
    }
}

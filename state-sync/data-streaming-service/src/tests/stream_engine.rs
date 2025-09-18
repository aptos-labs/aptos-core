// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    data_notification::{DataClientRequest, EpochEndingLedgerInfosRequest},
    error::Error,
    stream_engine::{DataStreamEngine, EpochEndingStreamEngine, StreamEngine},
    streaming_client::{
        ContinuouslyStreamTransactionOutputsRequest,
        ContinuouslyStreamTransactionsOrOutputsRequest, ContinuouslyStreamTransactionsRequest,
        GetAllEpochEndingLedgerInfosRequest, StreamRequest,
    },
    tests::{utils, utils::initialize_logger},
};
use aptos_config::config::DataStreamingServiceConfig;
use aptos_data_client::{
    global_summary::{GlobalDataSummary, OptimalChunkSizes},
    interface::ResponsePayload,
};
use aptos_id_generator::U64IdGenerator;
use aptos_storage_service_types::responses::CompleteDataRange;
use aptos_types::transaction::Version;
use claims::{assert_matches, assert_ok};
use std::{cmp, sync::Arc};

#[test]
fn test_create_epoch_ending_requests() {
    // Create a batch of large client requests and verify the result
    let highest_ending_epoch = 900;
    let mut stream_engine = create_epoch_ending_stream_engine(0, highest_ending_epoch);
    let client_requests = stream_engine
        .create_data_client_requests(
            5,
            5,
            0,
            &create_epoch_ending_chunk_sizes(10000),
            create_notification_id_generator(),
        )
        .unwrap();
    let expected_requests = vec![DataClientRequest::EpochEndingLedgerInfos(
        EpochEndingLedgerInfosRequest {
            start_epoch: 0,
            end_epoch: highest_ending_epoch,
        },
    )];
    assert_eq!(client_requests, expected_requests);

    // Create a batch of regular client requests and verify the result
    let mut stream_engine = create_epoch_ending_stream_engine(0, highest_ending_epoch);
    let chunk_size = 50;
    let client_requests = stream_engine
        .create_data_client_requests(
            3,
            3,
            0,
            &create_epoch_ending_chunk_sizes(chunk_size),
            create_notification_id_generator(),
        )
        .unwrap();
    for (i, client_request) in client_requests.iter().enumerate() {
        let i = i as u64;
        let expected_request =
            DataClientRequest::EpochEndingLedgerInfos(EpochEndingLedgerInfosRequest {
                start_epoch: i * chunk_size,
                end_epoch: ((i + 1) * chunk_size) - 1,
            });
        assert_eq!(*client_request, expected_request);
    }

    // Create a batch of small client requests and verify the result
    let mut stream_engine = create_epoch_ending_stream_engine(0, highest_ending_epoch);
    let chunk_size = 14;
    let client_requests = stream_engine
        .create_data_client_requests(
            100,
            100,
            0,
            &create_epoch_ending_chunk_sizes(chunk_size),
            create_notification_id_generator(),
        )
        .unwrap();
    for (i, client_request) in client_requests.iter().enumerate() {
        let i = i as u64;
        let expected_request =
            DataClientRequest::EpochEndingLedgerInfos(EpochEndingLedgerInfosRequest {
                start_epoch: i * chunk_size,
                end_epoch: cmp::min(((i + 1) * chunk_size) - 1, highest_ending_epoch),
            });
        assert_eq!(*client_request, expected_request);
    }
}

#[test]
fn test_create_epoch_ending_requests_dynamic() {
    // Create a new data stream engine
    let highest_ending_epoch = 1000;
    let mut stream_engine = create_epoch_ending_stream_engine(0, highest_ending_epoch);

    // Update the engine with a new next request epoch
    stream_engine.next_request_epoch = 150;

    // Create a batch of client requests and verify the result
    let client_requests = stream_engine
        .create_data_client_requests(
            5,
            5,
            0,
            &create_epoch_ending_chunk_sizes(700),
            create_notification_id_generator(),
        )
        .unwrap();
    let expected_requests = vec![
        DataClientRequest::EpochEndingLedgerInfos(EpochEndingLedgerInfosRequest {
            start_epoch: 150,
            end_epoch: 849,
        }),
        DataClientRequest::EpochEndingLedgerInfos(EpochEndingLedgerInfosRequest {
            start_epoch: 850,
            end_epoch: highest_ending_epoch,
        }),
    ];
    assert_eq!(client_requests, expected_requests);

    // Update the engine with a new next request epoch
    stream_engine.next_request_epoch = 700;

    // Create a batch of client requests and verify the result
    let chunk_size = 50;
    let client_requests = stream_engine
        .create_data_client_requests(
            10,
            10,
            0,
            &create_epoch_ending_chunk_sizes(chunk_size),
            create_notification_id_generator(),
        )
        .unwrap();
    for (i, client_request) in client_requests.iter().enumerate() {
        let i = i as u64;
        let expected_request =
            DataClientRequest::EpochEndingLedgerInfos(EpochEndingLedgerInfosRequest {
                start_epoch: 700 + (i * chunk_size),
                end_epoch: cmp::min(700 + ((i + 1) * chunk_size) - 1, highest_ending_epoch),
            });
        assert_eq!(*client_request, expected_request);
    }

    // Update the engine with a new next request epoch that matches the stream end
    stream_engine.next_request_epoch = 999;

    // Create a batch of client requests and verify the result
    let client_requests = stream_engine
        .create_data_client_requests(
            5,
            5,
            0,
            &create_epoch_ending_chunk_sizes(700),
            create_notification_id_generator(),
        )
        .unwrap();
    let expected_requests = vec![DataClientRequest::EpochEndingLedgerInfos(
        EpochEndingLedgerInfosRequest {
            start_epoch: 999,
            end_epoch: highest_ending_epoch,
        },
    )];
    assert_eq!(client_requests, expected_requests);

    // Update the engine with a new next request epoch that is at the end
    stream_engine.next_request_epoch = highest_ending_epoch;

    // Create a batch of client requests and verify no error
    let client_requests = stream_engine.create_data_client_requests(
        10,
        10,
        0,
        &create_epoch_ending_chunk_sizes(50),
        create_notification_id_generator(),
    );
    assert_ok!(client_requests);
}

#[test]
fn test_epoch_ending_stream_engine() {
    // Create an epoch ending stream request
    let stream_request =
        StreamRequest::GetAllEpochEndingLedgerInfos(GetAllEpochEndingLedgerInfosRequest {
            start_epoch: 0,
        });

    // Try to create a stream engine where there is no advertised data
    // and verify an error is returned.
    let data_streaming_config = DataStreamingServiceConfig::default();
    let result = StreamEngine::new(
        data_streaming_config,
        &stream_request,
        &GlobalDataSummary::empty().advertised_data,
    );
    assert_matches!(result, Err(Error::DataIsUnavailable(_)));

    // Create a data summary with various advertised epoch ranges (highest is one)
    let mut global_data_summary = GlobalDataSummary::empty();
    global_data_summary
        .advertised_data
        .epoch_ending_ledger_infos = vec![
        CompleteDataRange::new(0, 0).unwrap(),
        CompleteDataRange::new(0, 0).unwrap(),
        CompleteDataRange::new(0, 1).unwrap(),
    ];

    // Try to create a stream engine where the highest epoch is one
    let result = StreamEngine::new(
        data_streaming_config,
        &stream_request,
        &global_data_summary.advertised_data,
    );
    assert_ok!(result);

    // Create a global data summary with non-zero advertised epoch ranges
    let mut global_data_summary = GlobalDataSummary::empty();
    global_data_summary
        .advertised_data
        .epoch_ending_ledger_infos = vec![
        CompleteDataRange::new(0, 1).unwrap(),
        CompleteDataRange::new(0, 100).unwrap(),
        CompleteDataRange::new(0, 1000).unwrap(),
        CompleteDataRange::new(0, 100).unwrap(),
    ];

    // Create a new data stream engine and verify the highest epoch is chosen
    match StreamEngine::new(
        data_streaming_config,
        &stream_request,
        &global_data_summary.advertised_data,
    )
    .unwrap()
    {
        StreamEngine::EpochEndingStreamEngine(stream_engine) => {
            assert_eq!(stream_engine.end_epoch, 1000);
        },
        unexpected_engine => {
            panic!(
                "Expected epoch ending stream engine but got {:?}",
                unexpected_engine
            );
        },
    }
}

#[test]
fn test_invalid_continuous_transactions_stream_engine() {
    // Create a continuous transaction stream request with an invalid target
    let known_version = 100;
    let known_epoch = 5;
    let target = utils::create_ledger_info(known_version - 1, known_epoch, false);
    let stream_request =
        StreamRequest::ContinuouslyStreamTransactions(ContinuouslyStreamTransactionsRequest {
            known_version,
            known_epoch,
            include_events: false,
            target: Some(target),
        });

    // Create a global data summary with non-zero advertised data
    let global_data_summary = create_global_data_summary_for_transactions(known_version);

    // Try to create a stream engine and verify an error is returned (the
    // target version is already known, so there is no new data to fetch).
    let data_streaming_config = DataStreamingServiceConfig::default();
    let result = StreamEngine::new(
        data_streaming_config,
        &stream_request,
        &global_data_summary.advertised_data,
    );
    assert_matches!(result, Err(Error::UnexpectedErrorEncountered(_)));
}

#[test]
fn test_invalid_continuous_transactions_or_outputs_stream_engine() {
    // Create a continuous transactions or outputs stream request with an invalid target
    let known_version = 100;
    let known_epoch = 5;
    let target = utils::create_ledger_info(known_version - 2, known_epoch, false);
    let stream_request = StreamRequest::ContinuouslyStreamTransactionsOrOutputs(
        ContinuouslyStreamTransactionsOrOutputsRequest {
            known_version,
            known_epoch,
            include_events: false,
            target: Some(target),
        },
    );

    // Create a global data summary with non-zero advertised data
    let global_data_summary = create_global_data_summary_for_transactions(known_version);

    // Try to create a stream engine and verify an error is returned (the
    // target version is already known, so there is no new data to fetch).
    let data_streaming_config = DataStreamingServiceConfig::default();
    let result = StreamEngine::new(
        data_streaming_config,
        &stream_request,
        &global_data_summary.advertised_data,
    );
    assert_matches!(result, Err(Error::UnexpectedErrorEncountered(_)));
}

#[test]
fn test_invalid_continuous_outputs_stream_engine() {
    // Create a continuous outputs stream request with an invalid target
    let known_version = 100;
    let known_epoch = 5;
    let target = utils::create_ledger_info(known_version, known_epoch, false);
    let stream_request = StreamRequest::ContinuouslyStreamTransactionOutputs(
        ContinuouslyStreamTransactionOutputsRequest {
            known_version,
            known_epoch,
            target: Some(target),
        },
    );

    // Create a global data summary with non-zero advertised data
    let global_data_summary = create_global_data_summary_for_transactions(known_version);

    // Try to create a stream engine and verify an error is returned (the
    // target version is already known, so there is no new data to fetch).
    let data_streaming_config = DataStreamingServiceConfig::default();
    let result = StreamEngine::new(
        data_streaming_config,
        &stream_request,
        &global_data_summary.advertised_data,
    );
    assert_matches!(result, Err(Error::UnexpectedErrorEncountered(_)));
}

#[test]
fn test_update_epoch_ending_stream_progress() {
    // Create a new data stream engine
    let mut stream_engine = create_epoch_ending_stream_engine(0, 1000);

    // Update the stream engine using valid sent data notifications
    for i in 0..10 {
        let start_epoch = i * 100;
        let end_epoch = (i * 100) + 99;
        let _ = stream_engine
            .transform_client_response_into_notification(
                &DataClientRequest::EpochEndingLedgerInfos(EpochEndingLedgerInfosRequest {
                    start_epoch,
                    end_epoch,
                }),
                create_epoch_ending_ledger_info_payload(start_epoch, end_epoch),
                create_notification_id_generator(),
            )
            .unwrap();

        // Verify internal state
        assert_eq!(stream_engine.next_stream_epoch, end_epoch + 1);
    }
}

#[test]
#[should_panic(expected = "The start index did not match the expected next index!")]
fn test_update_epoch_ending_stream_panic() {
    // Create a new data stream engine
    let mut stream_engine = create_epoch_ending_stream_engine(0, 1000);

    // Update the engine with a valid notification
    let client_response_payload =
        ResponsePayload::EpochEndingLedgerInfos(vec![utils::create_ledger_info(0, 0, true)]);
    let _ = stream_engine
        .transform_client_response_into_notification(
            &DataClientRequest::EpochEndingLedgerInfos(EpochEndingLedgerInfosRequest {
                start_epoch: 0,
                end_epoch: 100,
            }),
            client_response_payload,
            create_notification_id_generator(),
        )
        .unwrap();

    // Update the engine with an old notification and verify a panic
    let _ = stream_engine
        .transform_client_response_into_notification(
            &DataClientRequest::EpochEndingLedgerInfos(EpochEndingLedgerInfosRequest {
                start_epoch: 50,
                end_epoch: 1100,
            }),
            create_empty_client_response_payload(),
            create_notification_id_generator(),
        )
        .unwrap();
}

fn create_epoch_ending_stream_engine(start_epoch: u64, end_epoch: u64) -> EpochEndingStreamEngine {
    initialize_logger();

    // Create an epoch ending stream request
    let stream_request =
        StreamRequest::GetAllEpochEndingLedgerInfos(GetAllEpochEndingLedgerInfosRequest {
            start_epoch,
        });

    // Create a global data summary with a single epoch range
    let mut global_data_summary = GlobalDataSummary::empty();
    global_data_summary
        .advertised_data
        .epoch_ending_ledger_infos = vec![CompleteDataRange::new(start_epoch, end_epoch).unwrap()];

    // Create a new epoch ending stream engine
    let data_streaming_config = DataStreamingServiceConfig::default();
    match StreamEngine::new(
        data_streaming_config,
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
    }
}

fn create_epoch_ending_chunk_sizes(epoch_chunk_size: u64) -> GlobalDataSummary {
    let mut optimal_chunk_sizes = OptimalChunkSizes::empty();
    optimal_chunk_sizes.epoch_chunk_size = epoch_chunk_size;

    let mut global_data_summary = GlobalDataSummary::empty();
    global_data_summary.optimal_chunk_sizes = optimal_chunk_sizes;

    global_data_summary
}

fn create_notification_id_generator() -> Arc<U64IdGenerator> {
    Arc::new(U64IdGenerator::new())
}

fn create_empty_client_response_payload() -> ResponsePayload {
    ResponsePayload::EpochEndingLedgerInfos(vec![])
}

/// Creates a response payload with the given number of epoch ending ledger infos
fn create_epoch_ending_ledger_info_payload(start_epoch: u64, end_epoch: u64) -> ResponsePayload {
    let epoch_ending_ledger_infos = (start_epoch..end_epoch + 1)
        .map(|i| utils::create_ledger_info(i, i, true))
        .collect();
    ResponsePayload::EpochEndingLedgerInfos(epoch_ending_ledger_infos)
}

/// Creates a global data summary with advertised transactions and transaction outputs
fn create_global_data_summary_for_transactions(known_version: Version) -> GlobalDataSummary {
    let mut global_data_summary = GlobalDataSummary::empty();
    let complete_data_range = CompleteDataRange::new(0, known_version * 2).unwrap();
    global_data_summary.advertised_data.transactions = vec![complete_data_range];
    global_data_summary.advertised_data.transaction_outputs = vec![complete_data_range];
    global_data_summary
}

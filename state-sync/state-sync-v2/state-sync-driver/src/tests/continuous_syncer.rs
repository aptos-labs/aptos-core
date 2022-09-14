// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    continuous_syncer::ContinuousSyncer,
    driver::DriverConfiguration,
    error::Error,
    notification_handlers::ConsensusSyncRequest,
    tests::{
        mocks::{
            create_mock_db_reader, create_mock_streaming_client, create_ready_storage_synchronizer,
            MockStorageSynchronizer, MockStreamingClient,
        },
        utils::{
            create_data_stream_listener, create_epoch_ending_ledger_info, create_epoch_state,
            create_full_node_driver_configuration, create_transaction_info,
        },
    },
};
use aptos_config::config::ContinuousSyncingMode;
use aptos_infallible::Mutex;
use aptos_types::transaction::{TransactionOutputListWithProof, Version};
use claims::assert_matches;
use consensus_notifications::ConsensusSyncNotification;
use data_streaming_service::{
    data_notification::{DataNotification, DataPayload},
    streaming_client::{NotificationAndFeedback, NotificationFeedback},
};
use futures::SinkExt;
use mockall::{predicate::eq, Sequence};
use std::sync::Arc;
use storage_service_types::Epoch;

#[tokio::test]
async fn test_critical_timeout() {
    // Create test data
    let current_synced_epoch = 54;
    let current_synced_version = 904345;

    // Create a driver configuration
    let mut driver_configuration = create_full_node_driver_configuration();
    driver_configuration.config.continuous_syncing_mode =
        ContinuousSyncingMode::ApplyTransactionOutputs;
    driver_configuration.config.max_stream_wait_time_ms = 1000;

    // Create the mock streaming client
    let mut mock_streaming_client = create_mock_streaming_client();
    let mut expectation_sequence = Sequence::new();
    let (_notification_sender_1, data_stream_listener_1) = create_data_stream_listener();
    let (_notification_sender_2, data_stream_listener_2) = create_data_stream_listener();
    let data_stream_id_1 = data_stream_listener_1.data_stream_id;
    for data_stream_listener in [data_stream_listener_1, data_stream_listener_2] {
        mock_streaming_client
            .expect_continuously_stream_transaction_outputs()
            .times(1)
            .with(
                eq(current_synced_version),
                eq(current_synced_epoch),
                eq(None),
            )
            .return_once(move |_, _, _| Ok(data_stream_listener))
            .in_sequence(&mut expectation_sequence);
    }
    mock_streaming_client
        .expect_terminate_stream_with_feedback()
        .with(eq(data_stream_id_1), eq(None))
        .return_const(Ok(()));

    // Create the continuous syncer
    let mut continuous_syncer = create_continuous_syncer(
        driver_configuration,
        mock_streaming_client,
        true,
        current_synced_version,
        current_synced_epoch,
    );

    // Drive progress to initialize the transaction output stream
    let no_sync_request = Arc::new(Mutex::new(None));
    continuous_syncer
        .drive_progress(no_sync_request.clone())
        .await
        .unwrap();

    // Drive progress twice and verify we get non-critical timeouts
    for _ in 0..2 {
        let error = continuous_syncer
            .drive_progress(no_sync_request.clone())
            .await
            .unwrap_err();
        assert_matches!(error, Error::DataStreamNotificationTimeout(_));
    }

    // Drive progress again and verify we get a critical timeout
    let error = continuous_syncer
        .drive_progress(no_sync_request.clone())
        .await
        .unwrap_err();
    assert_matches!(error, Error::CriticalDataStreamTimeout(_));

    // Drive progress to initialize the transaction output stream again
    continuous_syncer
        .drive_progress(no_sync_request.clone())
        .await
        .unwrap();

    // Drive progress again and verify we get a non-critical timeout
    let error = continuous_syncer
        .drive_progress(no_sync_request.clone())
        .await
        .unwrap_err();
    assert_matches!(error, Error::DataStreamNotificationTimeout(_));
}

#[tokio::test]
async fn test_data_stream_transactions_with_target() {
    // Create test data
    let current_synced_epoch = 5;
    let current_synced_version = 234;
    let notification_id = 435345;
    let target_ledger_info = create_epoch_ending_ledger_info();

    // Create a driver configuration
    let mut driver_configuration = create_full_node_driver_configuration();
    driver_configuration.config.continuous_syncing_mode =
        ContinuousSyncingMode::ExecuteTransactions;

    // Create the mock streaming client
    let mut mock_streaming_client = create_mock_streaming_client();
    let mut expectation_sequence = Sequence::new();
    let (mut notification_sender_1, data_stream_listener_1) = create_data_stream_listener();
    let (_notification_sender_2, data_stream_listener_2) = create_data_stream_listener();
    let data_stream_id_1 = data_stream_listener_1.data_stream_id;
    for data_stream_listener in [data_stream_listener_1, data_stream_listener_2] {
        mock_streaming_client
            .expect_continuously_stream_transactions()
            .times(1)
            .with(
                eq(current_synced_version),
                eq(current_synced_epoch),
                eq(false),
                eq(Some(target_ledger_info.clone())),
            )
            .return_once(move |_, _, _, _| Ok(data_stream_listener))
            .in_sequence(&mut expectation_sequence);
    }
    mock_streaming_client
        .expect_terminate_stream_with_feedback()
        .with(
            eq(data_stream_id_1),
            eq(Some(NotificationAndFeedback::new(
                notification_id,
                NotificationFeedback::EmptyPayloadData,
            ))),
        )
        .return_const(Ok(()));

    // Create the continuous syncer
    let mut continuous_syncer = create_continuous_syncer(
        driver_configuration,
        mock_streaming_client,
        true,
        current_synced_version,
        current_synced_epoch,
    );

    // Drive progress to initialize the transaction output stream
    let (consensus_sync_notification, _) = ConsensusSyncNotification::new(target_ledger_info);
    let sync_request = Arc::new(Mutex::new(Some(ConsensusSyncRequest::new(
        consensus_sync_notification,
    ))));
    continuous_syncer
        .drive_progress(sync_request.clone())
        .await
        .unwrap();

    // Send an invalid output along the stream
    let data_notification = DataNotification {
        notification_id,
        data_payload: DataPayload::ContinuousTransactionOutputsWithProof(
            create_epoch_ending_ledger_info(),
            TransactionOutputListWithProof::new_empty(),
        ),
    };
    notification_sender_1.send(data_notification).await.unwrap();

    // Drive progress again and ensure we get a verification error
    let error = continuous_syncer
        .drive_progress(sync_request.clone())
        .await
        .unwrap_err();
    assert_matches!(error, Error::VerificationError(_));

    // Drive progress to initialize the transaction output stream
    continuous_syncer
        .drive_progress(sync_request.clone())
        .await
        .unwrap();
}

#[tokio::test]
async fn test_data_stream_transaction_outputs() {
    // Create test data
    let current_synced_epoch = 100;
    let current_synced_version = 5;
    let notification_id = 1235;

    // Create a driver configuration
    let mut driver_configuration = create_full_node_driver_configuration();
    driver_configuration.config.continuous_syncing_mode =
        ContinuousSyncingMode::ApplyTransactionOutputs;

    // Create the mock streaming client
    let mut mock_streaming_client = create_mock_streaming_client();
    let mut expectation_sequence = Sequence::new();
    let (mut notification_sender_1, data_stream_listener_1) = create_data_stream_listener();
    let (_notification_sender_2, data_stream_listener_2) = create_data_stream_listener();
    let data_stream_id_1 = data_stream_listener_1.data_stream_id;
    for data_stream_listener in [data_stream_listener_1, data_stream_listener_2] {
        mock_streaming_client
            .expect_continuously_stream_transaction_outputs()
            .times(1)
            .with(
                eq(current_synced_version),
                eq(current_synced_epoch),
                eq(None),
            )
            .return_once(move |_, _, _| Ok(data_stream_listener))
            .in_sequence(&mut expectation_sequence);
    }
    mock_streaming_client
        .expect_terminate_stream_with_feedback()
        .with(
            eq(data_stream_id_1),
            eq(Some(NotificationAndFeedback::new(
                notification_id,
                NotificationFeedback::InvalidPayloadData,
            ))),
        )
        .return_const(Ok(()));

    // Create the continuous syncer
    let mut continuous_syncer = create_continuous_syncer(
        driver_configuration,
        mock_streaming_client,
        true,
        current_synced_version,
        current_synced_epoch,
    );

    // Drive progress to initialize the transaction output stream
    let no_sync_request = Arc::new(Mutex::new(None));
    continuous_syncer
        .drive_progress(no_sync_request.clone())
        .await
        .unwrap();

    // Send an invalid output along the stream
    let mut transaction_output_with_proof = TransactionOutputListWithProof::new_empty();
    transaction_output_with_proof.first_transaction_output_version =
        Some(current_synced_version - 1);
    let data_notification = DataNotification {
        notification_id,
        data_payload: DataPayload::ContinuousTransactionOutputsWithProof(
            create_epoch_ending_ledger_info(),
            transaction_output_with_proof,
        ),
    };
    notification_sender_1.send(data_notification).await.unwrap();

    // Drive progress again and ensure we get a verification error
    let error = continuous_syncer
        .drive_progress(no_sync_request.clone())
        .await
        .unwrap_err();
    assert_matches!(error, Error::VerificationError(_));

    // Drive progress to initialize the transaction output stream
    continuous_syncer
        .drive_progress(no_sync_request.clone())
        .await
        .unwrap();
}

/// Creates a continuous syncer for testing
fn create_continuous_syncer(
    driver_configuration: DriverConfiguration,
    mock_streaming_client: MockStreamingClient,
    expect_reset_executor: bool,
    synced_version: Version,
    current_epoch: Epoch,
) -> ContinuousSyncer<MockStorageSynchronizer, MockStreamingClient> {
    // Initialize the logger for tests
    aptos_logger::Logger::init_for_testing();

    // Create the mock storage synchronizer
    let mock_storage_synchronizer = create_ready_storage_synchronizer(expect_reset_executor);

    // Create the mock db reader with the given synced version
    let mut mock_database_reader = create_mock_db_reader();
    mock_database_reader
        .expect_get_latest_transaction_info_option()
        .returning(move || Ok(Some((synced_version, create_transaction_info()))));
    mock_database_reader
        .expect_get_latest_epoch_state()
        .returning(move || Ok(create_epoch_state(current_epoch)));

    ContinuousSyncer::new(
        driver_configuration,
        mock_streaming_client,
        Arc::new(mock_database_reader),
        mock_storage_synchronizer,
    )
}

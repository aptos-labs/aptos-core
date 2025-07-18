// Copyright © Aptos Foundation
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
            create_full_node_driver_configuration,
        },
    },
    utils::OutputFallbackHandler,
};
use aptos_config::config::ContinuousSyncingMode;
use aptos_consensus_notifications::{
    ConsensusSyncDurationNotification, ConsensusSyncTargetNotification,
};
use aptos_data_streaming_service::{
    data_notification::{DataNotification, DataPayload, NotificationId},
    streaming_client::{NotificationAndFeedback, NotificationFeedback},
};
use aptos_infallible::Mutex;
use aptos_storage_service_types::Epoch;
use aptos_time_service::TimeService;
use aptos_types::transaction::{
    TransactionOutputListWithProof, TransactionOutputListWithProofV2, Version,
};
use claims::assert_matches;
use futures::SinkExt;
use mockall::{predicate::eq, Sequence};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

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
    driver_configuration.config.max_num_stream_timeouts = 4;

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
    let (mut continuous_syncer, _) = create_continuous_syncer(
        driver_configuration,
        mock_streaming_client,
        None,
        true,
        current_synced_version,
        current_synced_epoch,
    );

    // Drive progress to initialize the transaction output stream
    let no_sync_request = Arc::new(Mutex::new(None));
    drive_progress(&mut continuous_syncer, &no_sync_request).await;

    // Drive progress and verify we get non-critical timeouts
    for _ in 0..3 {
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
    drive_progress(&mut continuous_syncer, &no_sync_request).await;

    // Drive progress again and verify we get a non-critical timeout
    let error = continuous_syncer
        .drive_progress(no_sync_request.clone())
        .await
        .unwrap_err();
    assert_matches!(error, Error::DataStreamNotificationTimeout(_));
}

#[tokio::test]
async fn test_data_stream_transactions_with_sync_duration() {
    // Create test data
    let current_synced_epoch = 10;
    let current_synced_version = 1000;
    let notification_id = 900;

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
                eq(None),
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
    let (mut continuous_syncer, _) = create_continuous_syncer(
        driver_configuration,
        mock_streaming_client,
        None,
        true,
        current_synced_version,
        current_synced_epoch,
    );

    // Drive progress to initialize the transaction output stream for the sync duration
    let (sync_duration_notification, _) =
        ConsensusSyncDurationNotification::new(Duration::from_secs(1));
    let sync_request = Arc::new(Mutex::new(Some(ConsensusSyncRequest::new_with_duration(
        Instant::now(),
        sync_duration_notification,
    ))));
    drive_progress(&mut continuous_syncer, &sync_request).await;

    // Send an invalid output along the stream
    let data_notification = DataNotification::new(
        notification_id,
        DataPayload::ContinuousTransactionOutputsWithProof(
            create_epoch_ending_ledger_info(),
            TransactionOutputListWithProofV2::new_empty(),
        ),
    );
    notification_sender_1.send(data_notification).await.unwrap();

    // Drive progress again and ensure we get a verification error
    let error = continuous_syncer
        .drive_progress(sync_request.clone())
        .await
        .unwrap_err();
    assert_matches!(error, Error::VerificationError(_));

    // Drive progress to initialize the transaction output stream.
    drive_progress(&mut continuous_syncer, &sync_request).await;
}

#[tokio::test]
async fn test_data_stream_transactions_with_sync_target() {
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
    let (mut continuous_syncer, _) = create_continuous_syncer(
        driver_configuration,
        mock_streaming_client,
        None,
        true,
        current_synced_version,
        current_synced_epoch,
    );

    // Drive progress to initialize the transaction output stream for the sync target
    let (sync_target_notification, _) = ConsensusSyncTargetNotification::new(target_ledger_info);
    let sync_request = Arc::new(Mutex::new(Some(ConsensusSyncRequest::new_with_target(
        sync_target_notification,
    ))));
    drive_progress(&mut continuous_syncer, &sync_request).await;

    // Send an invalid output along the stream
    let data_notification = DataNotification::new(
        notification_id,
        DataPayload::ContinuousTransactionOutputsWithProof(
            create_epoch_ending_ledger_info(),
            TransactionOutputListWithProofV2::new_empty(),
        ),
    );
    notification_sender_1.send(data_notification).await.unwrap();

    // Drive progress again and ensure we get a verification error
    let error = continuous_syncer
        .drive_progress(sync_request.clone())
        .await
        .unwrap_err();
    assert_matches!(error, Error::VerificationError(_));

    // Drive progress to initialize the transaction output stream
    drive_progress(&mut continuous_syncer, &sync_request).await;
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
    let (mut continuous_syncer, _) = create_continuous_syncer(
        driver_configuration,
        mock_streaming_client,
        None,
        true,
        current_synced_version,
        current_synced_epoch,
    );

    // Drive progress to initialize the transaction output stream (without a sync target)
    let no_sync_request = Arc::new(Mutex::new(None));
    drive_progress(&mut continuous_syncer, &no_sync_request).await;

    // Send an invalid output along the stream
    let mut transaction_output_with_proof = TransactionOutputListWithProof::new_empty();
    transaction_output_with_proof.first_transaction_output_version =
        Some(current_synced_version - 1);
    let data_notification = DataNotification::new(
        notification_id,
        DataPayload::ContinuousTransactionOutputsWithProof(
            create_epoch_ending_ledger_info(),
            TransactionOutputListWithProofV2::new_from_v1(transaction_output_with_proof),
        ),
    );
    notification_sender_1.send(data_notification).await.unwrap();

    // Drive progress again and ensure we get a verification error
    let error = continuous_syncer
        .drive_progress(no_sync_request.clone())
        .await
        .unwrap_err();
    assert_matches!(error, Error::VerificationError(_));

    // Drive progress to initialize the transaction output stream
    drive_progress(&mut continuous_syncer, &no_sync_request).await;
}

#[tokio::test]
async fn test_data_stream_transactions_or_outputs_with_sync_duration() {
    // Create test data
    let current_synced_epoch = 100;
    let current_synced_version = 1000;
    let notification_id = 100;

    // Create a driver configuration with a genesis waypoint and transactions or output syncing
    let mut driver_configuration = create_full_node_driver_configuration();
    driver_configuration.config.continuous_syncing_mode =
        ContinuousSyncingMode::ExecuteTransactionsOrApplyOutputs;

    // Create the mock streaming client
    let mut mock_streaming_client = create_mock_streaming_client();
    let mut expectation_sequence = Sequence::new();
    let (mut notification_sender_1, data_stream_listener_1) = create_data_stream_listener();
    let (_notification_sender_2, data_stream_listener_2) = create_data_stream_listener();
    let data_stream_id_1 = data_stream_listener_1.data_stream_id;
    for data_stream_listener in [data_stream_listener_1, data_stream_listener_2] {
        mock_streaming_client
            .expect_continuously_stream_transactions_or_outputs()
            .times(1)
            .with(
                eq(current_synced_version),
                eq(current_synced_epoch),
                eq(false),
                eq(None),
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
    let (mut continuous_syncer, _) = create_continuous_syncer(
        driver_configuration,
        mock_streaming_client,
        None,
        true,
        current_synced_version,
        current_synced_epoch,
    );

    // Drive progress to initialize the transaction output stream for the sync duration
    let (sync_duration_notification, _) =
        ConsensusSyncDurationNotification::new(Duration::from_secs(1));
    let sync_request = Arc::new(Mutex::new(Some(ConsensusSyncRequest::new_with_duration(
        Instant::now(),
        sync_duration_notification,
    ))));
    drive_progress(&mut continuous_syncer, &sync_request).await;

    // Send an invalid output along the stream
    let data_notification = DataNotification::new(
        notification_id,
        DataPayload::ContinuousTransactionOutputsWithProof(
            create_epoch_ending_ledger_info(),
            TransactionOutputListWithProofV2::new_empty(),
        ),
    );
    notification_sender_1.send(data_notification).await.unwrap();

    // Drive progress again and ensure we get a verification error
    let error = continuous_syncer
        .drive_progress(sync_request.clone())
        .await
        .unwrap_err();
    assert_matches!(error, Error::VerificationError(_));

    // Drive progress to initialize the transaction output stream
    drive_progress(&mut continuous_syncer, &sync_request).await;
}

#[tokio::test]
async fn test_data_stream_transactions_or_outputs_with_sync_target() {
    // Create test data
    let current_synced_epoch = 5;
    let current_synced_version = 234;
    let notification_id = 435345;
    let target_ledger_info = create_epoch_ending_ledger_info();

    // Create a driver configuration with a genesis waypoint and transactions or output syncing
    let mut driver_configuration = create_full_node_driver_configuration();
    driver_configuration.config.continuous_syncing_mode =
        ContinuousSyncingMode::ExecuteTransactionsOrApplyOutputs;

    // Create the mock streaming client
    let mut mock_streaming_client = create_mock_streaming_client();
    let mut expectation_sequence = Sequence::new();
    let (mut notification_sender_1, data_stream_listener_1) = create_data_stream_listener();
    let (_notification_sender_2, data_stream_listener_2) = create_data_stream_listener();
    let data_stream_id_1 = data_stream_listener_1.data_stream_id;
    for data_stream_listener in [data_stream_listener_1, data_stream_listener_2] {
        mock_streaming_client
            .expect_continuously_stream_transactions_or_outputs()
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
    let (mut continuous_syncer, _) = create_continuous_syncer(
        driver_configuration,
        mock_streaming_client,
        None,
        true,
        current_synced_version,
        current_synced_epoch,
    );

    // Drive progress to initialize the transaction output stream for the sync target
    let (sync_target_notification, _) = ConsensusSyncTargetNotification::new(target_ledger_info);
    let sync_request = Arc::new(Mutex::new(Some(ConsensusSyncRequest::new_with_target(
        sync_target_notification,
    ))));
    drive_progress(&mut continuous_syncer, &sync_request).await;

    // Send an invalid output along the stream
    let data_notification = DataNotification::new(
        notification_id,
        DataPayload::ContinuousTransactionOutputsWithProof(
            create_epoch_ending_ledger_info(),
            TransactionOutputListWithProofV2::new_empty(),
        ),
    );
    notification_sender_1.send(data_notification).await.unwrap();

    // Drive progress again and ensure we get a verification error
    let error = continuous_syncer
        .drive_progress(sync_request.clone())
        .await
        .unwrap_err();
    assert_matches!(error, Error::VerificationError(_));

    // Drive progress to initialize the transaction output stream
    drive_progress(&mut continuous_syncer, &sync_request).await;
}

#[tokio::test]
async fn test_data_stream_transactions_or_outputs_with_sync_duration_fallback() {
    // Create test data
    let current_synced_epoch = 50;
    let current_synced_version = 1234;
    let notification_id = 101;

    // Create a driver configuration with a genesis waypoint and transactions or output syncing
    let mut driver_configuration = create_full_node_driver_configuration();
    driver_configuration.config.continuous_syncing_mode =
        ContinuousSyncingMode::ExecuteTransactionsOrApplyOutputs;

    // Create the mock streaming client
    let mut mock_streaming_client = create_mock_streaming_client();

    // Set expectations for stream creations and terminations
    let mut expectation_sequence = Sequence::new();
    let (_notification_sender_1, data_stream_listener_1) = create_data_stream_listener();
    let data_stream_id_1 = data_stream_listener_1.data_stream_id;
    let (_notification_sender_2, data_stream_listener_2) = create_data_stream_listener();
    let data_stream_id_2 = data_stream_listener_2.data_stream_id;
    let (_notification_sender_3, data_stream_listener_3) = create_data_stream_listener();
    mock_streaming_client
        .expect_continuously_stream_transactions_or_outputs()
        .times(1)
        .with(
            eq(current_synced_version),
            eq(current_synced_epoch),
            eq(false),
            eq(None),
        )
        .return_once(move |_, _, _, _| Ok(data_stream_listener_1))
        .in_sequence(&mut expectation_sequence);
    mock_streaming_client
        .expect_terminate_stream_with_feedback()
        .times(1)
        .with(
            eq(data_stream_id_1),
            eq(Some(NotificationAndFeedback::new(
                notification_id,
                NotificationFeedback::PayloadProofFailed,
            ))),
        )
        .return_const(Ok(()))
        .in_sequence(&mut expectation_sequence);
    mock_streaming_client
        .expect_continuously_stream_transaction_outputs()
        .times(1)
        .with(
            eq(current_synced_version),
            eq(current_synced_epoch),
            eq(None),
        )
        .return_once(move |_, _, _| Ok(data_stream_listener_2))
        .in_sequence(&mut expectation_sequence);
    mock_streaming_client
        .expect_terminate_stream_with_feedback()
        .times(1)
        .with(
            eq(data_stream_id_2),
            eq(Some(NotificationAndFeedback::new(
                notification_id,
                NotificationFeedback::InvalidPayloadData,
            ))),
        )
        .return_const(Ok(()))
        .in_sequence(&mut expectation_sequence);
    mock_streaming_client
        .expect_continuously_stream_transactions_or_outputs()
        .times(1)
        .with(
            eq(current_synced_version),
            eq(current_synced_epoch),
            eq(false),
            eq(None),
        )
        .return_once(move |_, _, _, _| Ok(data_stream_listener_3))
        .in_sequence(&mut expectation_sequence);

    // Create the continuous syncer
    let time_service = TimeService::mock();
    let (mut continuous_syncer, mut output_fallback_handler) = create_continuous_syncer(
        driver_configuration.clone(),
        mock_streaming_client,
        Some(time_service.clone()),
        true,
        current_synced_version,
        current_synced_epoch,
    );
    assert!(!output_fallback_handler.in_fallback_mode());

    // Drive progress to initialize the transactions or output stream for the sync duration
    let (sync_duration_notification, _) =
        ConsensusSyncDurationNotification::new(Duration::from_secs(1));
    let sync_request = Arc::new(Mutex::new(Some(ConsensusSyncRequest::new_with_duration(
        Instant::now(),
        sync_duration_notification,
    ))));
    drive_progress(&mut continuous_syncer, &sync_request).await;

    // Send a storage synchronizer error to the continuous syncer so that it falls back
    // to output syncing and drive progress for the new stream type.
    handle_storage_synchronizer_error(
        &mut continuous_syncer,
        notification_id,
        NotificationFeedback::PayloadProofFailed,
    )
    .await;
    drive_progress(&mut continuous_syncer, &sync_request).await;
    assert!(output_fallback_handler.in_fallback_mode());

    // Elapse enough time so that fallback mode is now disabled
    time_service
        .into_mock()
        .advance_async(Duration::from_secs(
            driver_configuration.config.fallback_to_output_syncing_secs,
        ))
        .await;

    // Send another storage synchronizer error to the bootstrapper and drive progress
    // so that a regular stream is created.
    handle_storage_synchronizer_error(
        &mut continuous_syncer,
        notification_id,
        NotificationFeedback::InvalidPayloadData,
    )
    .await;
    drive_progress(&mut continuous_syncer, &sync_request).await;
    assert!(!output_fallback_handler.in_fallback_mode());
}

#[tokio::test]
async fn test_data_stream_transactions_or_outputs_with_sync_target_fallback() {
    // Create test data
    let current_synced_epoch = 5;
    let current_synced_version = 234;
    let notification_id = 435345;
    let target_ledger_info = create_epoch_ending_ledger_info();

    // Create a driver configuration with a genesis waypoint and transactions or output syncing
    let mut driver_configuration = create_full_node_driver_configuration();
    driver_configuration.config.continuous_syncing_mode =
        ContinuousSyncingMode::ExecuteTransactionsOrApplyOutputs;

    // Create the mock streaming client
    let mut mock_streaming_client = create_mock_streaming_client();

    // Set expectations for stream creations and terminations
    let mut expectation_sequence = Sequence::new();
    let (_notification_sender_1, data_stream_listener_1) = create_data_stream_listener();
    let data_stream_id_1 = data_stream_listener_1.data_stream_id;
    let (_notification_sender_2, data_stream_listener_2) = create_data_stream_listener();
    let data_stream_id_2 = data_stream_listener_2.data_stream_id;
    let (_notification_sender_3, data_stream_listener_3) = create_data_stream_listener();
    mock_streaming_client
        .expect_continuously_stream_transactions_or_outputs()
        .times(1)
        .with(
            eq(current_synced_version),
            eq(current_synced_epoch),
            eq(false),
            eq(Some(target_ledger_info.clone())),
        )
        .return_once(move |_, _, _, _| Ok(data_stream_listener_1))
        .in_sequence(&mut expectation_sequence);
    mock_streaming_client
        .expect_terminate_stream_with_feedback()
        .times(1)
        .with(
            eq(data_stream_id_1),
            eq(Some(NotificationAndFeedback::new(
                notification_id,
                NotificationFeedback::PayloadProofFailed,
            ))),
        )
        .return_const(Ok(()))
        .in_sequence(&mut expectation_sequence);
    mock_streaming_client
        .expect_continuously_stream_transaction_outputs()
        .times(1)
        .with(
            eq(current_synced_version),
            eq(current_synced_epoch),
            eq(Some(target_ledger_info.clone())),
        )
        .return_once(move |_, _, _| Ok(data_stream_listener_2))
        .in_sequence(&mut expectation_sequence);
    mock_streaming_client
        .expect_terminate_stream_with_feedback()
        .times(1)
        .with(
            eq(data_stream_id_2),
            eq(Some(NotificationAndFeedback::new(
                notification_id,
                NotificationFeedback::InvalidPayloadData,
            ))),
        )
        .return_const(Ok(()))
        .in_sequence(&mut expectation_sequence);
    mock_streaming_client
        .expect_continuously_stream_transactions_or_outputs()
        .times(1)
        .with(
            eq(current_synced_version),
            eq(current_synced_epoch),
            eq(false),
            eq(Some(target_ledger_info.clone())),
        )
        .return_once(move |_, _, _, _| Ok(data_stream_listener_3))
        .in_sequence(&mut expectation_sequence);

    // Create the continuous syncer
    let time_service = TimeService::mock();
    let (mut continuous_syncer, mut output_fallback_handler) = create_continuous_syncer(
        driver_configuration.clone(),
        mock_streaming_client,
        Some(time_service.clone()),
        true,
        current_synced_version,
        current_synced_epoch,
    );
    assert!(!output_fallback_handler.in_fallback_mode());

    // Drive progress to initialize the transactions or output stream
    let (sync_target_notification, _) = ConsensusSyncTargetNotification::new(target_ledger_info);
    let sync_request = Arc::new(Mutex::new(Some(ConsensusSyncRequest::new_with_target(
        sync_target_notification,
    ))));
    drive_progress(&mut continuous_syncer, &sync_request).await;

    // Send a storage synchronizer error to the continuous syncer so that it falls back
    // to output syncing and drive progress for the new stream type.
    handle_storage_synchronizer_error(
        &mut continuous_syncer,
        notification_id,
        NotificationFeedback::PayloadProofFailed,
    )
    .await;
    drive_progress(&mut continuous_syncer, &sync_request).await;
    assert!(output_fallback_handler.in_fallback_mode());

    // Elapse enough time so that fallback mode is now disabled
    time_service
        .into_mock()
        .advance_async(Duration::from_secs(
            driver_configuration.config.fallback_to_output_syncing_secs,
        ))
        .await;

    // Send another storage synchronizer error to the bootstrapper and drive progress
    // so that a regular stream is created.
    handle_storage_synchronizer_error(
        &mut continuous_syncer,
        notification_id,
        NotificationFeedback::InvalidPayloadData,
    )
    .await;
    drive_progress(&mut continuous_syncer, &sync_request).await;
    assert!(!output_fallback_handler.in_fallback_mode());
}

/// Creates a continuous syncer for testing
fn create_continuous_syncer(
    driver_configuration: DriverConfiguration,
    mock_streaming_client: MockStreamingClient,
    time_service: Option<TimeService>,
    expect_reset_executor: bool,
    synced_version: Version,
    current_epoch: Epoch,
) -> (
    ContinuousSyncer<MockStorageSynchronizer, MockStreamingClient>,
    OutputFallbackHandler,
) {
    // Initialize the logger for tests
    aptos_logger::Logger::init_for_testing();

    // Create the mock storage synchronizer
    let mock_storage_synchronizer = create_ready_storage_synchronizer(expect_reset_executor);

    // Create the mock db reader with the given synced version
    let mut mock_database_reader = create_mock_db_reader();
    mock_database_reader
        .expect_get_synced_version()
        .returning(move || Ok(Some(synced_version)));
    mock_database_reader
        .expect_get_pre_committed_version()
        .returning(move || Ok(Some(synced_version)));
    mock_database_reader
        .expect_get_latest_epoch_state()
        .returning(move || Ok(create_epoch_state(current_epoch)));

    // Create the output fallback handler
    let time_service = time_service.unwrap_or_else(TimeService::mock);
    let output_fallback_handler =
        OutputFallbackHandler::new(driver_configuration.clone(), time_service);

    // Create the continuous syncer
    let continuous_syncer = ContinuousSyncer::new(
        driver_configuration,
        mock_streaming_client,
        output_fallback_handler.clone(),
        Arc::new(mock_database_reader),
        mock_storage_synchronizer,
    );

    (continuous_syncer, output_fallback_handler)
}

/// Drives progress on the given syncer
async fn drive_progress(
    continuous_syncer: &mut ContinuousSyncer<MockStorageSynchronizer, MockStreamingClient>,
    no_sync_request: &Arc<Mutex<Option<ConsensusSyncRequest>>>,
) {
    continuous_syncer
        .drive_progress(no_sync_request.clone())
        .await
        .unwrap();
}

/// Handles the given storage synchronizer error for the bootstrapper
async fn handle_storage_synchronizer_error(
    continuous_syncer: &mut ContinuousSyncer<MockStorageSynchronizer, MockStreamingClient>,
    notification_id: NotificationId,
    notification_feedback: NotificationFeedback,
) {
    continuous_syncer
        .handle_storage_synchronizer_error(NotificationAndFeedback::new(
            notification_id,
            notification_feedback,
        ))
        .await
        .unwrap();
}

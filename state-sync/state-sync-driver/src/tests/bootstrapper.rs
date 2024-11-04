// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    bootstrapper::{Bootstrapper, GENESIS_TRANSACTION_VERSION},
    driver::DriverConfiguration,
    error::Error,
    tests::{
        mocks::{
            create_mock_db_reader, create_mock_streaming_client, create_ready_storage_synchronizer,
            MockMetadataStorage, MockStorageSynchronizer, MockStreamingClient,
        },
        utils::{
            create_data_stream_listener, create_empty_epoch_state, create_epoch_ending_ledger_info,
            create_epoch_ending_ledger_info_for_epoch, create_full_node_driver_configuration,
            create_global_summary, create_global_summary_with_version,
            create_output_list_with_proof, create_random_epoch_ending_ledger_info,
            create_transaction_list_with_proof,
        },
    },
    utils::OutputFallbackHandler,
};
use aptos_config::config::BootstrappingMode;
use aptos_data_client::global_summary::GlobalDataSummary;
use aptos_data_streaming_service::{
    data_notification::{DataNotification, DataPayload, NotificationId},
    streaming_client::{NotificationAndFeedback, NotificationFeedback},
};
use aptos_time_service::TimeService;
use aptos_types::{
    transaction::{TransactionOutputListWithProof, Version},
    waypoint::Waypoint,
};
use claims::{assert_matches, assert_none, assert_ok};
use futures::{channel::oneshot, FutureExt, SinkExt};
use mockall::{predicate::eq, Sequence};
use std::{sync::Arc, time::Duration};

#[tokio::test]
async fn test_bootstrap_genesis_waypoint() {
    // Create a driver configuration with a genesis waypoint
    let driver_configuration = create_full_node_driver_configuration();

    // Create the mock streaming client
    let mock_streaming_client = create_mock_streaming_client();

    // Create the bootstrapper and verify it's not yet bootstrapped
    let (mut bootstrapper, _) =
        create_bootstrapper(driver_configuration, mock_streaming_client, None, true);
    assert!(!bootstrapper.is_bootstrapped());

    // Subscribe to a bootstrapped notification
    let (bootstrap_notification_sender, bootstrap_notification_receiver) = oneshot::channel();
    bootstrapper
        .subscribe_to_bootstrap_notifications(bootstrap_notification_sender)
        .await
        .unwrap();

    // Create a global data summary where only epoch 0 has ended
    let global_data_summary = create_global_summary(0);

    // Drive progress and verify we're now bootstrapped
    drive_progress(&mut bootstrapper, &global_data_summary, true)
        .await
        .unwrap();
    assert!(bootstrapper.is_bootstrapped());
    verify_bootstrap_notification(bootstrap_notification_receiver);

    // Drive progress again and verify we get an error (we're already bootstrapped!)
    let error = drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap_err();
    assert_matches!(error, Error::AlreadyBootstrapped(_));
}

#[tokio::test]
async fn test_bootstrap_genesis_waypoint_ahead_of_peers() {
    // Create a driver configuration with a genesis waypoint
    let driver_configuration = create_full_node_driver_configuration();

    // Create the mock streaming client and metadata storage
    let mock_streaming_client = create_mock_streaming_client();
    let metadata_storage = MockMetadataStorage::new();

    // Create the test data for the bootstrapper
    let synced_version = 100;
    let synced_epoch = 10;

    // Create the bootstrapper at the specified synced epoch and version
    let mut bootstrapper = create_bootstrapper_with_storage(
        driver_configuration,
        mock_streaming_client,
        metadata_storage,
        Some(synced_epoch),
        synced_version,
        true,
    );

    // Verify the bootstrapper is not yet bootstrapped
    assert!(!bootstrapper.is_bootstrapped());

    // Subscribe to a bootstrapped notification
    let (bootstrap_notification_sender, bootstrap_notification_receiver) = oneshot::channel();
    bootstrapper
        .subscribe_to_bootstrap_notifications(bootstrap_notification_sender)
        .await
        .unwrap();

    // Create a global data summary where only epoch 0 has ended
    let global_data_summary = create_global_summary(0);

    // Drive progress and verify we're now bootstrapped
    drive_progress(&mut bootstrapper, &global_data_summary, true)
        .await
        .unwrap();
    assert!(bootstrapper.is_bootstrapped());
    verify_bootstrap_notification(bootstrap_notification_receiver);

    // Drive progress again and verify we get an error (we're already bootstrapped!)
    let error = drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap_err();
    assert_matches!(error, Error::AlreadyBootstrapped(_));
}

#[tokio::test]
async fn test_bootstrap_waypoint_satisfiability_check_failed() {
    // Create a driver configuration
    let mut driver_configuration = create_full_node_driver_configuration();

    // Update the driver configuration to use a waypoint in the future
    let waypoint_version = 1000;
    let waypoint_epoch = 100;
    let waypoint = create_random_epoch_ending_ledger_info(waypoint_version, waypoint_epoch);
    driver_configuration.waypoint = Waypoint::new_any(waypoint.ledger_info());

    // Create the mock streaming client and metadata storage
    let mock_streaming_client = create_mock_streaming_client();
    let metadata_storage = MockMetadataStorage::new();

    // Create the test data for the bootstrapper
    let synced_version = 100;
    let synced_epoch = 10;

    // Create the bootstrapper at the specified synced epoch and version
    let mut bootstrapper = create_bootstrapper_with_storage(
        driver_configuration,
        mock_streaming_client,
        metadata_storage,
        Some(synced_epoch),
        synced_version,
        true,
    );

    // Verify the bootstrapper is not yet bootstrapped
    assert!(!bootstrapper.is_bootstrapped());

    // Create a global data summary that is missing version information
    let advertised_epoch = 5;
    let global_data_summary = create_global_summary(advertised_epoch);

    // Drive progress and verify that an error is returned (waypoint
    // satisfiability cannot be checked).
    let error = drive_progress(&mut bootstrapper, &global_data_summary, true)
        .await
        .unwrap_err();
    assert_matches!(error, Error::UnsatisfiableWaypoint(_));
}

#[tokio::test]
async fn test_bootstrap_waypoint_unsatisfiable() {
    // Create a driver configuration
    let mut driver_configuration = create_full_node_driver_configuration();

    // Update the driver configuration to use a waypoint in the future
    let waypoint_version = 1000;
    let waypoint_epoch = 100;
    let waypoint = create_random_epoch_ending_ledger_info(waypoint_version, waypoint_epoch);
    driver_configuration.waypoint = Waypoint::new_any(waypoint.ledger_info());

    // Create the mock streaming client and metadata storage
    let mock_streaming_client = create_mock_streaming_client();
    let metadata_storage = MockMetadataStorage::new();

    // Create the test data for the bootstrapper
    let synced_version = 100;
    let synced_epoch = 10;

    // Create the bootstrapper at the specified synced epoch and version
    let mut bootstrapper = create_bootstrapper_with_storage(
        driver_configuration,
        mock_streaming_client,
        metadata_storage,
        Some(synced_epoch),
        synced_version,
        true,
    );

    // Verify the bootstrapper is not yet bootstrapped
    assert!(!bootstrapper.is_bootstrapped());

    // Create a global data summary where the latest data is less than the waypoint
    let advertised_version = waypoint_version - 1;
    let advertised_epoch = 5;
    let global_data_summary =
        create_global_summary_with_version(advertised_version, advertised_epoch);

    // Drive progress and verify that an error is returned (as the waypoint is not satisfiable)
    let error = drive_progress(&mut bootstrapper, &global_data_summary, true)
        .await
        .unwrap_err();
    assert_matches!(error, Error::UnsatisfiableWaypoint(_));
}

#[tokio::test]
async fn test_bootstrap_immediate_notification() {
    // Create a driver configuration with a genesis waypoint
    let driver_configuration = create_full_node_driver_configuration();

    // Create the mock streaming client
    let mock_streaming_client = create_mock_streaming_client();

    // Create the bootstrapper
    let (mut bootstrapper, _) =
        create_bootstrapper(driver_configuration, mock_streaming_client, None, true);

    // Create a global data summary where only epoch 0 has ended
    let global_data_summary = create_global_summary(0);

    // Drive progress and verify we're now bootstrapped
    drive_progress(&mut bootstrapper, &global_data_summary, true)
        .await
        .unwrap();
    assert!(bootstrapper.is_bootstrapped());

    // Subscribe to a bootstrapped notification and verify immediate notification
    let (bootstrap_notification_sender, bootstrap_notification_receiver) = oneshot::channel();
    bootstrapper
        .subscribe_to_bootstrap_notifications(bootstrap_notification_sender)
        .await
        .unwrap();
    verify_bootstrap_notification(bootstrap_notification_receiver);
}

#[tokio::test]
async fn test_bootstrap_no_notification() {
    // Create a driver configuration with a genesis waypoint
    let driver_configuration = create_full_node_driver_configuration();

    // Create the mock streaming client
    let mut mock_streaming_client = create_mock_streaming_client();
    let (_notification_sender, data_stream_listener) = create_data_stream_listener();
    mock_streaming_client
        .expect_get_all_epoch_ending_ledger_infos()
        .with(eq(1))
        .return_once(move |_| Ok(data_stream_listener));

    // Create the bootstrapper
    let (mut bootstrapper, _) =
        create_bootstrapper(driver_configuration, mock_streaming_client, None, true);

    // Create a global data summary where epoch 0 and 1 have ended
    let global_data_summary = create_global_summary(1);

    // Subscribe to a bootstrapped notification
    let (bootstrap_notification_sender, bootstrap_notification_receiver) = oneshot::channel();
    bootstrapper
        .subscribe_to_bootstrap_notifications(bootstrap_notification_sender)
        .await
        .unwrap();

    // Drive progress
    drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap();

    // Verify no notification
    assert_none!(bootstrap_notification_receiver.now_or_never());
}

#[tokio::test]
async fn test_critical_timeout() {
    // Create a driver configuration with a genesis waypoint and a stream timeout of 1 second
    let mut driver_configuration = create_full_node_driver_configuration();
    driver_configuration.config.max_stream_wait_time_ms = 1000;
    driver_configuration.config.max_num_stream_timeouts = 6;

    // Create the mock streaming client
    let mut mock_streaming_client = create_mock_streaming_client();
    let mut expectation_sequence = Sequence::new();
    let (_notification_sender_1, data_stream_listener_1) = create_data_stream_listener();
    let (_notification_sender_2, data_stream_listener_2) = create_data_stream_listener();
    let data_stream_id_1 = data_stream_listener_1.data_stream_id;
    for data_stream_listener in [data_stream_listener_1, data_stream_listener_2] {
        mock_streaming_client
            .expect_get_all_epoch_ending_ledger_infos()
            .times(1)
            .with(eq(1))
            .return_once(move |_| Ok(data_stream_listener))
            .in_sequence(&mut expectation_sequence);
    }
    mock_streaming_client
        .expect_terminate_stream_with_feedback()
        .with(eq(data_stream_id_1), eq(None))
        .return_const(Ok(()));

    // Create the bootstrapper
    let (mut bootstrapper, _) =
        create_bootstrapper(driver_configuration, mock_streaming_client, None, true);

    // Create a global data summary where epoch 0 and 1 have ended
    let global_data_summary = create_global_summary(1);

    // Drive progress to initialize the epoch ending data stream
    drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap();

    // Drive progress and verify we get non-critical timeouts
    for _ in 0..5 {
        let error = drive_progress(&mut bootstrapper, &global_data_summary, false)
            .await
            .unwrap_err();
        assert_matches!(error, Error::DataStreamNotificationTimeout(_));
    }

    // Drive progress again and verify we get a critical timeout
    let error = drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap_err();
    assert_matches!(error, Error::CriticalDataStreamTimeout(_));

    // Drive progress to initialize the epoch ending data stream again
    drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap();

    // Drive progress again and verify we get a non-critical timeout
    let error = drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap_err();
    assert_matches!(error, Error::DataStreamNotificationTimeout(_));
}

#[tokio::test]
async fn test_data_stream_state_values() {
    // Create test data
    let notification_id = 50043;
    let highest_version = 10000;
    let highest_ledger_info = create_random_epoch_ending_ledger_info(highest_version, 1);

    // Create a driver configuration with a genesis waypoint and state syncing
    let mut driver_configuration = create_full_node_driver_configuration();
    driver_configuration.config.bootstrapping_mode = BootstrappingMode::DownloadLatestStates;

    // Create the mock streaming client
    let mut mock_streaming_client = create_mock_streaming_client();
    let mut expectation_sequence = Sequence::new();
    let (mut notification_sender_1, data_stream_listener_1) = create_data_stream_listener();
    let (_notification_sender_2, data_stream_listener_2) = create_data_stream_listener();
    let data_stream_id_1 = data_stream_listener_1.data_stream_id;
    for data_stream_listener in [data_stream_listener_1, data_stream_listener_2] {
        mock_streaming_client
            .expect_get_all_transaction_outputs()
            .times(1)
            .with(
                eq(highest_version),
                eq(highest_version),
                eq(highest_version),
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

    // Create the bootstrapper
    let (mut bootstrapper, _) =
        create_bootstrapper(driver_configuration, mock_streaming_client, None, true);

    // Insert an epoch ending ledger info into the verified states of the bootstrapper
    manipulate_verified_epoch_states(&mut bootstrapper, true, true, Some(highest_version));

    // Create a global data summary
    let mut global_data_summary = create_global_summary(1);
    global_data_summary.advertised_data.synced_ledger_infos = vec![highest_ledger_info.clone()];

    // Drive progress to initialize the state values stream
    drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap();

    // Send an invalid output along the stream
    let data_notification = DataNotification::new(
        notification_id,
        DataPayload::TransactionOutputsWithProof(create_output_list_with_proof()),
    );
    notification_sender_1.send(data_notification).await.unwrap();

    // Drive progress again and ensure we get a verification error
    let error = drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap_err();
    assert_matches!(error, Error::VerificationError(_));

    // Drive progress to initialize the state value stream
    drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_data_stream_transactions() {
    // Create test data
    let notification_id = 0;
    let highest_version = 9998765;
    let highest_ledger_info = create_random_epoch_ending_ledger_info(highest_version, 1);

    // Create a driver configuration with a genesis waypoint and transaction syncing
    let mut driver_configuration = create_full_node_driver_configuration();
    driver_configuration.config.bootstrapping_mode =
        BootstrappingMode::ExecuteTransactionsFromGenesis;

    // Create the mock streaming client
    let mut mock_streaming_client = create_mock_streaming_client();
    let mut expectation_sequence = Sequence::new();
    let (mut notification_sender_1, data_stream_listener_1) = create_data_stream_listener();
    let (_notification_sender_2, data_stream_listener_2) = create_data_stream_listener();
    let data_stream_id_1 = data_stream_listener_1.data_stream_id;
    for data_stream_listener in [data_stream_listener_1, data_stream_listener_2] {
        mock_streaming_client
            .expect_get_all_transactions()
            .times(1)
            .with(eq(1), eq(highest_version), eq(highest_version), eq(false))
            .return_once(move |_, _, _, _| Ok(data_stream_listener))
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

    // Create the bootstrapper
    let (mut bootstrapper, _) =
        create_bootstrapper(driver_configuration, mock_streaming_client, None, true);

    // Insert an epoch ending ledger info into the verified states of the bootstrapper
    manipulate_verified_epoch_states(&mut bootstrapper, true, true, Some(highest_version));

    // Create a global data summary
    let mut global_data_summary = create_global_summary(1);
    global_data_summary.advertised_data.synced_ledger_infos = vec![highest_ledger_info.clone()];

    // Drive progress to initialize the transaction output stream
    drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap();

    // Send an invalid output along the stream
    let data_notification = DataNotification::new(
        notification_id,
        DataPayload::TransactionsWithProof(create_transaction_list_with_proof()),
    );
    notification_sender_1.send(data_notification).await.unwrap();

    // Drive progress again and ensure we get a verification error
    let error = drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap_err();
    assert_matches!(error, Error::VerificationError(_));

    // Drive progress to initialize the transaction output stream
    drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_data_stream_transaction_outputs() {
    // Create test data
    let notification_id = 1235;
    let highest_version = 45;
    let highest_ledger_info = create_random_epoch_ending_ledger_info(highest_version, 1);

    // Create a driver configuration with a genesis waypoint and output syncing
    let mut driver_configuration = create_full_node_driver_configuration();
    driver_configuration.config.bootstrapping_mode =
        BootstrappingMode::ApplyTransactionOutputsFromGenesis;

    // Create the mock streaming client
    let mut mock_streaming_client = create_mock_streaming_client();
    let mut expectation_sequence = Sequence::new();
    let (mut notification_sender_1, data_stream_listener_1) = create_data_stream_listener();
    let (_notification_sender_2, data_stream_listener_2) = create_data_stream_listener();
    let data_stream_id_1 = data_stream_listener_1.data_stream_id;
    for data_stream_listener in [data_stream_listener_1, data_stream_listener_2] {
        mock_streaming_client
            .expect_get_all_transaction_outputs()
            .times(1)
            .with(eq(1), eq(highest_version), eq(highest_version))
            .return_once(move |_, _, _| Ok(data_stream_listener))
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

    // Create the bootstrapper
    let (mut bootstrapper, _) =
        create_bootstrapper(driver_configuration, mock_streaming_client, None, true);

    // Insert an epoch ending ledger info into the verified states of the bootstrapper
    manipulate_verified_epoch_states(&mut bootstrapper, true, true, Some(highest_version));

    // Create a global data summary
    let mut global_data_summary = create_global_summary(1);
    global_data_summary.advertised_data.synced_ledger_infos = vec![highest_ledger_info.clone()];

    // Drive progress to initialize the transaction output stream
    drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap();

    // Send an invalid output along the stream
    let data_notification = DataNotification::new(
        notification_id,
        DataPayload::TransactionOutputsWithProof(TransactionOutputListWithProof::new_empty()),
    );
    notification_sender_1.send(data_notification).await.unwrap();

    // Drive progress again and ensure we get a verification error
    let error = drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap_err();
    assert_matches!(error, Error::VerificationError(_));

    // Drive progress to initialize the transaction output stream
    drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_data_stream_transactions_or_outputs() {
    // Create test data
    let notification_id = 0;
    let highest_version = 9998765;
    let highest_ledger_info = create_random_epoch_ending_ledger_info(highest_version, 1);

    // Create a driver configuration with a genesis waypoint and transaction or output syncing
    let mut driver_configuration = create_full_node_driver_configuration();
    driver_configuration.config.bootstrapping_mode = BootstrappingMode::ExecuteOrApplyFromGenesis;

    // Create the mock streaming client
    let mut mock_streaming_client = create_mock_streaming_client();
    let mut expectation_sequence = Sequence::new();
    let (mut notification_sender_1, data_stream_listener_1) = create_data_stream_listener();
    let (_notification_sender_2, data_stream_listener_2) = create_data_stream_listener();
    let data_stream_id_1 = data_stream_listener_1.data_stream_id;
    for data_stream_listener in [data_stream_listener_1, data_stream_listener_2] {
        mock_streaming_client
            .expect_get_all_transactions_or_outputs()
            .times(1)
            .with(eq(1), eq(highest_version), eq(highest_version), eq(false))
            .return_once(move |_, _, _, _| Ok(data_stream_listener))
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

    // Create the bootstrapper
    let (mut bootstrapper, _) =
        create_bootstrapper(driver_configuration, mock_streaming_client, None, true);

    // Insert an epoch ending ledger info into the verified states of the bootstrapper
    manipulate_verified_epoch_states(&mut bootstrapper, true, true, Some(highest_version));

    // Create a global data summary
    let mut global_data_summary = create_global_summary(1);
    global_data_summary.advertised_data.synced_ledger_infos = vec![highest_ledger_info.clone()];

    // Drive progress to initialize the transaction output stream
    drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap();

    // Send an invalid output along the stream
    let data_notification = DataNotification::new(
        notification_id,
        DataPayload::EpochEndingLedgerInfos(vec![create_epoch_ending_ledger_info()]),
    );
    notification_sender_1.send(data_notification).await.unwrap();

    // Drive progress again and ensure we get an invalid payload error
    let error = drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap_err();
    assert_matches!(error, Error::InvalidPayload(_));

    // Drive progress to initialize the transaction output stream
    drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_data_stream_transactions_or_outputs_fallback() {
    // Create test data
    let notification_id = 1;
    let highest_version = 9998765;
    let highest_ledger_info = create_random_epoch_ending_ledger_info(highest_version, 1);

    // Create a driver configuration with a genesis waypoint and transaction or output syncing
    let mut driver_configuration = create_full_node_driver_configuration();
    driver_configuration.config.bootstrapping_mode = BootstrappingMode::ExecuteOrApplyFromGenesis;

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
        .expect_get_all_transactions_or_outputs()
        .times(1)
        .with(eq(1), eq(highest_version), eq(highest_version), eq(false))
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
        .expect_get_all_transaction_outputs()
        .times(1)
        .with(eq(1), eq(highest_version), eq(highest_version))
        .return_once(move |_, _, _| Ok(data_stream_listener_2));
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
        .expect_get_all_transactions_or_outputs()
        .with(eq(1), eq(highest_version), eq(highest_version), eq(false))
        .return_once(move |_, _, _, _| Ok(data_stream_listener_3));

    // Create the bootstrapper
    let time_service = TimeService::mock();
    let (mut bootstrapper, mut output_fallback_handler) = create_bootstrapper(
        driver_configuration.clone(),
        mock_streaming_client,
        Some(time_service.clone()),
        true,
    );
    assert!(!output_fallback_handler.in_fallback_mode());

    // Insert an epoch ending ledger info into the verified states of the bootstrapper
    manipulate_verified_epoch_states(&mut bootstrapper, true, true, Some(highest_version));

    // Create a global data summary
    let mut global_data_summary = create_global_summary(1);
    global_data_summary.advertised_data.synced_ledger_infos = vec![highest_ledger_info.clone()];

    // Drive progress to initialize the first transaction output stream
    drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap();

    // Send a storage synchronizer error to the bootstrapper so that it falls back
    // to output syncing and drive progress for the new stream type.
    handle_storage_synchronizer_error(
        &mut bootstrapper,
        notification_id,
        NotificationFeedback::PayloadProofFailed,
    )
    .await;
    drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap();
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
        &mut bootstrapper,
        notification_id,
        NotificationFeedback::InvalidPayloadData,
    )
    .await;
    drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap();
    assert!(!output_fallback_handler.in_fallback_mode());
}

#[tokio::test]
async fn test_fetch_epoch_ending_ledger_infos() {
    // Create a driver configuration
    let mut driver_configuration = create_full_node_driver_configuration();

    // Update the driver configuration to use a waypoint in the future
    let waypoint_version = 100;
    let waypoint_epoch = 100;
    let waypoint = create_random_epoch_ending_ledger_info(waypoint_version, waypoint_epoch);
    driver_configuration.waypoint = Waypoint::new_any(waypoint.ledger_info());

    // Create the mock streaming client
    let mut mock_streaming_client = create_mock_streaming_client();
    let (mut notification_sender, data_stream_listener) = create_data_stream_listener();
    mock_streaming_client
        .expect_get_all_epoch_ending_ledger_infos()
        .with(eq(1))
        .return_once(move |_| Ok(data_stream_listener));
    mock_streaming_client
        .expect_terminate_stream_with_feedback()
        .return_const(Ok(()));

    // Create the bootstrapper
    let (mut bootstrapper, _) =
        create_bootstrapper(driver_configuration, mock_streaming_client, None, true);

    // Create a global data summary where epoch 100 has ended
    let global_data_summary =
        create_global_summary_with_version(waypoint_epoch, waypoint_version + 1);

    // Drive progress to initialize the epoch ending data stream
    drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap();

    // Create the first set of epoch ending ledger infos and send them across the stream
    let num_ledger_infos_to_send = waypoint_epoch / 2;
    let mut epoch_ending_ledger_infos = vec![];
    for index in 0..num_ledger_infos_to_send {
        epoch_ending_ledger_infos.push(create_random_epoch_ending_ledger_info(index, index));
    }
    let data_notification = DataNotification::new(
        0,
        DataPayload::EpochEndingLedgerInfos(epoch_ending_ledger_infos.clone()),
    );
    notification_sender.send(data_notification).await.unwrap();

    // Drive progress to process the first set of epoch ending ledger infos
    let error = drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap_err();
    assert_matches!(error, Error::DataStreamNotificationTimeout(_));

    // Verify we're not bootstrapped yet
    assert!(!bootstrapper.is_bootstrapped());

    // Verify the bootstrapper has not fetched all ledger infos or verified the waypoint
    let verified_epoch_states = bootstrapper.get_verified_epoch_states().clone();
    assert!(!verified_epoch_states.fetched_epoch_ending_ledger_infos());
    assert!(!verified_epoch_states.verified_waypoint());

    // Verify the epoch states contains the first set of epoch ending ledger infos
    let verified_ledger_infos = verified_epoch_states.all_epoch_ending_ledger_infos();
    assert_eq!(verified_ledger_infos.len() as u64, num_ledger_infos_to_send);
    for epoch_ending_ledger_info in epoch_ending_ledger_infos {
        assert!(verified_ledger_infos.contains(&epoch_ending_ledger_info));
    }

    // Create the second set of epoch ending ledger infos and send them across the stream
    let mut epoch_ending_ledger_infos = vec![];
    for index in num_ledger_infos_to_send..waypoint_epoch + 1 {
        epoch_ending_ledger_infos.push(create_random_epoch_ending_ledger_info(index, index));
    }
    let data_notification = DataNotification::new(
        1,
        DataPayload::EpochEndingLedgerInfos(epoch_ending_ledger_infos.clone()),
    );
    notification_sender.send(data_notification).await.unwrap();

    // Artificially overwrite the waypoint hash so that verification passes
    let last_ledger_info = epoch_ending_ledger_infos.last().unwrap().ledger_info();
    bootstrapper.set_waypoint(Waypoint::new_any(last_ledger_info));

    // Drive progress to process the second set of epoch ending ledger infos
    let error = drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap_err();
    assert_matches!(error, Error::DataStreamNotificationTimeout(_));

    // Ensure the bootstrapper verified the waypoint
    let verified_epoch_states = bootstrapper.get_verified_epoch_states().clone();
    assert!(verified_epoch_states.verified_waypoint());

    // Verify the epoch states contains all epoch ending ledger infos
    let verified_ledger_infos = verified_epoch_states.all_epoch_ending_ledger_infos();
    assert_eq!(verified_ledger_infos.len() as u64, waypoint_epoch + 1);
    for epoch_ending_ledger_info in epoch_ending_ledger_infos {
        assert!(verified_ledger_infos.contains(&epoch_ending_ledger_info));
    }

    // Send the end of stream notification
    let data_notification = DataNotification::new(2, DataPayload::EndOfStream);
    notification_sender.send(data_notification).await.unwrap();

    // Drive progress to process the end of stream notification
    for _ in 0..2 {
        drive_progress(&mut bootstrapper, &global_data_summary, false)
            .await
            .unwrap();
    }

    // Verify the bootstrapper has fetched all ledger infos
    let verified_epoch_states = bootstrapper.get_verified_epoch_states().clone();
    assert!(verified_epoch_states.fetched_epoch_ending_ledger_infos());
}

#[tokio::test]
async fn test_fetch_epoch_ending_ledger_infos_timeout() {
    // Create a driver configuration with a genesis waypoint and a stream timeout of 1 second
    let mut driver_configuration = create_full_node_driver_configuration();
    driver_configuration.config.max_stream_wait_time_ms = 1000;

    // Create the mock streaming client
    let mut mock_streaming_client = create_mock_streaming_client();
    let (_notification_sender, data_stream_listener) = create_data_stream_listener();
    mock_streaming_client
        .expect_get_all_epoch_ending_ledger_infos()
        .with(eq(1))
        .return_once(move |_| Ok(data_stream_listener));

    // Create the bootstrapper
    let (mut bootstrapper, _) =
        create_bootstrapper(driver_configuration, mock_streaming_client, None, true);

    // Set the waypoint as already having been verified (but no fetched ledger infos)
    manipulate_verified_epoch_states(&mut bootstrapper, false, true, None);

    // Create a global data summary where epoch 0 and 1 have ended
    let global_data_summary = create_global_summary(1);

    // Drive progress to initialize the epoch ending data stream
    drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap();

    // Drive progress and verify we get a timeout error as we're still waiting
    // for epoch ending ledger infos to epoch skip.
    let error = drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap_err();
    assert_matches!(error, Error::DataStreamNotificationTimeout(_));
}

#[tokio::test]
#[should_panic(expected = "Failed to verify the waypoint: Waypoint value mismatch")]
async fn test_fetch_epoch_ending_ledger_infos_waypoint_mismatch() {
    // Create a driver configuration
    let mut driver_configuration = create_full_node_driver_configuration();

    // Update the driver configuration to use a waypoint in the future
    let waypoint_version = 100;
    let waypoint_epoch = 100;
    let waypoint = create_random_epoch_ending_ledger_info(waypoint_version, waypoint_epoch);
    driver_configuration.waypoint = Waypoint::new_any(waypoint.ledger_info());

    // Create the mock streaming client
    let mut mock_streaming_client = create_mock_streaming_client();
    let (mut notification_sender, data_stream_listener) = create_data_stream_listener();
    mock_streaming_client
        .expect_get_all_epoch_ending_ledger_infos()
        .with(eq(1))
        .return_once(move |_| Ok(data_stream_listener));

    // Create the bootstrapper
    let (mut bootstrapper, _) =
        create_bootstrapper(driver_configuration, mock_streaming_client, None, true);

    // Create a global data summary where epoch 100 has ended
    let global_data_summary =
        create_global_summary_with_version(waypoint_epoch, waypoint_version + 1);

    // Drive progress to initialize the epoch ending data stream
    drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap();

    // Create a full set of epoch ending ledger infos and send them across the stream
    let mut epoch_ending_ledger_infos = vec![];
    for index in 0..waypoint_epoch + 1 {
        epoch_ending_ledger_infos.push(create_random_epoch_ending_ledger_info(index, index));
    }
    let data_notification = DataNotification::new(
        0,
        DataPayload::EpochEndingLedgerInfos(epoch_ending_ledger_infos.clone()),
    );
    notification_sender.send(data_notification).await.unwrap();

    // Drive progress to process the set of epoch ending ledger infos and panic at the waypoint mismatch
    drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_snapshot_sync_epoch_change() {
    // Create test data
    let synced_version = GENESIS_TRANSACTION_VERSION; // Genesis is the highest synced
    let target_version = 1000;
    let highest_version = 5000;
    let last_persisted_index = 1030405;
    let target_ledger_info = create_random_epoch_ending_ledger_info(target_version, 1);
    let highest_ledger_info = create_random_epoch_ending_ledger_info(highest_version, 2);

    // Create a driver configuration with a genesis waypoint and state syncing
    let mut driver_configuration = create_full_node_driver_configuration();
    driver_configuration.config.bootstrapping_mode = BootstrappingMode::DownloadLatestStates;

    // Create the mock streaming client
    let mut mock_streaming_client = create_mock_streaming_client();
    let (_notification_sender_1, data_stream_listener_1) = create_data_stream_listener();
    mock_streaming_client
        .expect_get_all_state_values()
        .times(1)
        .with(eq(target_version), eq(Some(last_persisted_index)))
        .return_once(move |_, _| Ok(data_stream_listener_1));

    // Create the mock metadata storage
    let mut metadata_storage = MockMetadataStorage::new();
    let target_ledger_info_clone = target_ledger_info.clone();
    let last_persisted_index_clone = last_persisted_index;
    metadata_storage
        .expect_previous_snapshot_sync_target()
        .returning(move || Ok(Some(target_ledger_info_clone.clone())));
    metadata_storage
        .expect_is_snapshot_sync_complete()
        .returning(|_| Ok(false));
    metadata_storage
        .expect_get_last_persisted_state_value_index()
        .returning(move |_| Ok(last_persisted_index_clone));

    // Create the bootstrapper
    let mut bootstrapper = create_bootstrapper_with_storage(
        driver_configuration,
        mock_streaming_client,
        metadata_storage,
        None,
        synced_version,
        true,
    );

    // Insert an epoch ending ledger info into the verified states of the bootstrapper
    manipulate_verified_epoch_states(&mut bootstrapper, true, true, Some(highest_version));

    // Manually insert a transaction output to sync
    bootstrapper
        .get_state_value_syncer()
        .set_transaction_output_to_sync(create_output_list_with_proof());

    // Create a global data summary
    let mut global_data_summary = create_global_summary(1);
    global_data_summary.advertised_data.synced_ledger_infos = vec![highest_ledger_info.clone()];

    // Drive progress to start the state value stream
    drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_snapshot_sync_epoch_change_genesis() {
    // Create test data
    let synced_version = GENESIS_TRANSACTION_VERSION; // Genesis is the highest synced version
    let target_version = GENESIS_TRANSACTION_VERSION; // Genesis should be the target
    let highest_version = 9999;
    let highest_ledger_info = create_random_epoch_ending_ledger_info(highest_version, 0);

    // Create a driver configuration with a genesis waypoint and fast syncing
    let mut driver_configuration = create_full_node_driver_configuration();
    driver_configuration.config.bootstrapping_mode = BootstrappingMode::DownloadLatestStates;

    // Create the mock streaming client
    let mut mock_streaming_client = create_mock_streaming_client();
    let (_notification_sender_1, data_stream_listener_1) = create_data_stream_listener();
    mock_streaming_client
        .expect_get_all_state_values()
        .times(1)
        .with(eq(target_version), eq(Some(0)))
        .return_once(move |_, _| Ok(data_stream_listener_1));

    // Create the mock metadata storage
    let mut metadata_storage = MockMetadataStorage::new();
    metadata_storage
        .expect_previous_snapshot_sync_target()
        .returning(move || Ok(None));

    // Create the bootstrapper
    let mut bootstrapper = create_bootstrapper_with_storage(
        driver_configuration,
        mock_streaming_client,
        metadata_storage,
        None,
        synced_version,
        true,
    );

    // Manually insert a transaction output to sync
    bootstrapper
        .get_state_value_syncer()
        .set_transaction_output_to_sync(create_output_list_with_proof());

    // Create a global data summary
    let mut global_data_summary = create_global_summary(0);
    global_data_summary.advertised_data.synced_ledger_infos = vec![highest_ledger_info.clone()];

    // Drive progress to verify the waypoint
    drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap();

    // Drive progress again to start the state value stream
    drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_snapshot_sync_epoch_change_genesis_restart() {
    // Create test data
    let synced_version = GENESIS_TRANSACTION_VERSION; // Genesis is the highest synced version
    let target_version = GENESIS_TRANSACTION_VERSION; // Genesis should be the target
    let highest_version = 5000;
    let last_persisted_index = 9999; // Fast syncing has already started in a previous run
    let target_ledger_info = create_random_epoch_ending_ledger_info(target_version, 0);
    let highest_ledger_info = create_random_epoch_ending_ledger_info(highest_version, 0);

    // Create a driver configuration with a genesis waypoint and fast syncing
    let mut driver_configuration = create_full_node_driver_configuration();
    driver_configuration.config.bootstrapping_mode = BootstrappingMode::DownloadLatestStates;

    // Create the mock streaming client
    let mut mock_streaming_client = create_mock_streaming_client();
    let (_notification_sender_1, data_stream_listener_1) = create_data_stream_listener();
    mock_streaming_client
        .expect_get_all_state_values()
        .times(1)
        .with(eq(target_version), eq(Some(last_persisted_index)))
        .return_once(move |_, _| Ok(data_stream_listener_1));

    // Create the mock metadata storage
    let mut metadata_storage = MockMetadataStorage::new();
    let target_ledger_info_clone = target_ledger_info.clone();
    let last_persisted_index_clone = last_persisted_index;
    metadata_storage
        .expect_previous_snapshot_sync_target()
        .returning(move || Ok(Some(target_ledger_info_clone.clone())));
    metadata_storage
        .expect_is_snapshot_sync_complete()
        .returning(|_| Ok(false));
    metadata_storage
        .expect_get_last_persisted_state_value_index()
        .returning(move |_| Ok(last_persisted_index_clone));

    // Create the bootstrapper
    let mut bootstrapper = create_bootstrapper_with_storage(
        driver_configuration,
        mock_streaming_client,
        metadata_storage,
        None,
        synced_version,
        true,
    );

    // Manually insert a transaction output to sync
    bootstrapper
        .get_state_value_syncer()
        .set_transaction_output_to_sync(create_output_list_with_proof());

    // Create a global data summary
    let mut global_data_summary = create_global_summary(0);
    global_data_summary.advertised_data.synced_ledger_infos = vec![highest_ledger_info.clone()];

    // Drive progress to verify the waypoint
    drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap();

    // Drive progress again to start the state value stream
    drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_snapshot_sync_existing_state() {
    // Create test data
    let synced_version = GENESIS_TRANSACTION_VERSION; // Genesis is the highest synced
    let highest_version = 1000000;
    let highest_ledger_info = create_random_epoch_ending_ledger_info(highest_version, 1);
    let last_persisted_index = 4567;

    // Create a driver configuration with a genesis waypoint and state syncing
    let mut driver_configuration = create_full_node_driver_configuration();
    driver_configuration.config.bootstrapping_mode = BootstrappingMode::DownloadLatestStates;

    // Create the mock streaming client
    let mut mock_streaming_client = create_mock_streaming_client();
    let mut expectation_sequence = Sequence::new();
    let (mut notification_sender_1, data_stream_listener_1) = create_data_stream_listener();
    let (_notification_sender_2, data_stream_listener_2) = create_data_stream_listener();
    let data_stream_id_1 = data_stream_listener_1.data_stream_id;
    mock_streaming_client
        .expect_get_all_state_values()
        .times(1)
        .with(eq(highest_version), eq(Some(last_persisted_index)))
        .return_once(move |_, _| Ok(data_stream_listener_1))
        .in_sequence(&mut expectation_sequence);
    let notification_id = 100;
    mock_streaming_client
        .expect_terminate_stream_with_feedback()
        .times(1)
        .with(
            eq(data_stream_id_1),
            eq(Some(NotificationAndFeedback::new(
                notification_id,
                NotificationFeedback::InvalidPayloadData,
            ))),
        )
        .return_const(Ok(()))
        .in_sequence(&mut expectation_sequence);
    mock_streaming_client
        .expect_get_all_state_values()
        .times(1)
        .with(eq(highest_version), eq(Some(last_persisted_index)))
        .return_once(move |_, _| Ok(data_stream_listener_2))
        .in_sequence(&mut expectation_sequence);

    // Create the mock metadata storage
    let mut metadata_storage = MockMetadataStorage::new();
    let highest_ledger_info_clone = highest_ledger_info.clone();
    let last_persisted_index_clone = last_persisted_index;
    metadata_storage
        .expect_previous_snapshot_sync_target()
        .returning(move || Ok(Some(highest_ledger_info_clone.clone())));
    metadata_storage
        .expect_is_snapshot_sync_complete()
        .returning(|_| Ok(false));
    metadata_storage
        .expect_get_last_persisted_state_value_index()
        .returning(move |_| Ok(last_persisted_index_clone));

    // Create the bootstrapper
    let mut bootstrapper = create_bootstrapper_with_storage(
        driver_configuration,
        mock_streaming_client,
        metadata_storage,
        None,
        synced_version,
        true,
    );

    // Insert an epoch ending ledger info into the verified states of the bootstrapper
    manipulate_verified_epoch_states(&mut bootstrapper, true, true, Some(highest_version));

    // Manually insert a transaction output to sync
    bootstrapper
        .get_state_value_syncer()
        .set_transaction_output_to_sync(create_output_list_with_proof());

    // Create a global data summary
    let mut global_data_summary = create_global_summary(1);
    global_data_summary.advertised_data.synced_ledger_infos = vec![highest_ledger_info.clone()];

    // Drive progress to start the state value stream
    drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap();

    // Send an invalid notification (incorrect data type)
    let data_notification = DataNotification::new(
        notification_id,
        DataPayload::TransactionOutputsWithProof(create_output_list_with_proof()),
    );
    notification_sender_1.send(data_notification).await.unwrap();

    // Drive progress again and ensure we get an invalid payload error
    let error = drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap_err();
    assert_matches!(error, Error::InvalidPayload(_));

    // Drive progress to start the state value stream again
    drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_snapshot_sync_fresh_state() {
    // Create test data
    let synced_version = GENESIS_TRANSACTION_VERSION; // Genesis is the highest synced
    let highest_version = 1000;
    let highest_ledger_info = create_random_epoch_ending_ledger_info(highest_version, 1);

    // Create a driver configuration with a genesis waypoint and state syncing
    let mut driver_configuration = create_full_node_driver_configuration();
    driver_configuration.config.bootstrapping_mode = BootstrappingMode::DownloadLatestStates;

    // Create the mock streaming client
    let mut mock_streaming_client = create_mock_streaming_client();
    let (_notification_sender_1, data_stream_listener_1) = create_data_stream_listener();
    mock_streaming_client
        .expect_get_all_state_values()
        .times(1)
        .with(eq(highest_version), eq(Some(0)))
        .return_once(move |_, _| Ok(data_stream_listener_1));

    // Create the mock metadata storage
    let mut metadata_storage = MockMetadataStorage::new();
    metadata_storage
        .expect_previous_snapshot_sync_target()
        .returning(move || Ok(None));

    // Create the bootstrapper
    let mut bootstrapper = create_bootstrapper_with_storage(
        driver_configuration,
        mock_streaming_client,
        metadata_storage,
        None,
        synced_version,
        true,
    );

    // Insert an epoch ending ledger info into the verified states of the bootstrapper
    manipulate_verified_epoch_states(&mut bootstrapper, true, true, Some(highest_version));

    // Manually insert a transaction output to sync
    bootstrapper
        .get_state_value_syncer()
        .set_transaction_output_to_sync(create_output_list_with_proof());

    // Create a global data summary
    let mut global_data_summary = create_global_summary(1);
    global_data_summary.advertised_data.synced_ledger_infos = vec![highest_ledger_info.clone()];

    // Drive progress to start the state value stream
    drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap();
}

#[tokio::test]
#[should_panic(
    expected = "The snapshot sync for the target was marked as complete but the highest synced version is genesis!"
)]
async fn test_snapshot_sync_invalid_state() {
    // Create test data
    let synced_version = GENESIS_TRANSACTION_VERSION; // Genesis is the highest synced
    let highest_version = 1000000;
    let highest_ledger_info = create_random_epoch_ending_ledger_info(highest_version, 1);

    // Create a driver configuration with a genesis waypoint and state syncing
    let mut driver_configuration = create_full_node_driver_configuration();
    driver_configuration.config.bootstrapping_mode = BootstrappingMode::DownloadLatestStates;

    // Create the mock streaming client
    let mock_streaming_client = create_mock_streaming_client();

    // Create the mock metadata storage
    let mut metadata_storage = MockMetadataStorage::new();
    let highest_ledger_info_clone = highest_ledger_info.clone();
    metadata_storage
        .expect_previous_snapshot_sync_target()
        .return_once(move || Ok(Some(highest_ledger_info_clone)));
    metadata_storage
        .expect_is_snapshot_sync_complete()
        .returning(|_| Ok(true));

    // Create the bootstrapper
    let mut bootstrapper = create_bootstrapper_with_storage(
        driver_configuration,
        mock_streaming_client,
        metadata_storage,
        None,
        synced_version,
        true,
    );

    // Insert an epoch ending ledger info into the verified states of the bootstrapper
    manipulate_verified_epoch_states(&mut bootstrapper, true, true, Some(highest_version));

    // Create a global data summary
    let mut global_data_summary = create_global_summary(1);
    global_data_summary.advertised_data.synced_ledger_infos = vec![highest_ledger_info.clone()];

    // Drive progress and verify that the bootstrapper panics (due to invalid state)
    drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_snapshot_sync_lag() {
    // Create test data
    let num_versions_behind = 1000;
    let highest_version = 1000000;
    let synced_version = highest_version - num_versions_behind;
    let highest_ledger_info = create_random_epoch_ending_ledger_info(highest_version, 1);

    // Create a driver configuration with a genesis waypoint and state syncing
    let mut driver_configuration = create_full_node_driver_configuration();
    driver_configuration.config.bootstrapping_mode = BootstrappingMode::DownloadLatestStates;
    driver_configuration
        .config
        .num_versions_to_skip_snapshot_sync = num_versions_behind + 1;

    // Create the mock streaming client
    let mock_streaming_client = create_mock_streaming_client();

    // Create the mock metadata storage
    let mut metadata_storage = MockMetadataStorage::new();
    metadata_storage
        .expect_previous_snapshot_sync_target()
        .returning(|| Ok(None));

    // Create the bootstrapper
    let mut bootstrapper = create_bootstrapper_with_storage(
        driver_configuration,
        mock_streaming_client,
        metadata_storage,
        None,
        synced_version,
        true,
    );

    // Insert an epoch ending ledger info into the verified states of the bootstrapper
    manipulate_verified_epoch_states(&mut bootstrapper, true, true, Some(highest_version));

    // Create a global data summary
    let mut global_data_summary = create_global_summary(1);
    global_data_summary.advertised_data.synced_ledger_infos = vec![highest_ledger_info.clone()];

    // Drive progress to mark bootstrapping complete (we're within the snapshot sync lag)
    drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap();

    // Verify the bootstrapper has completed
    assert!(bootstrapper.is_bootstrapped());
}

#[tokio::test]
#[should_panic(
    expected = "You are currently 10000 versions behind the latest snapshot version (1000000)"
)]
async fn test_snapshot_sync_lag_panic() {
    // Create test data
    let num_versions_behind = 10000;
    let highest_version = 1000000;
    let synced_version = highest_version - num_versions_behind;
    let highest_ledger_info = create_random_epoch_ending_ledger_info(highest_version, 1);

    // Create a driver configuration with a genesis waypoint and state syncing
    let mut driver_configuration = create_full_node_driver_configuration();
    driver_configuration.config.bootstrapping_mode = BootstrappingMode::DownloadLatestStates;
    driver_configuration
        .config
        .num_versions_to_skip_snapshot_sync = num_versions_behind;

    // Create the mock streaming client
    let mock_streaming_client = create_mock_streaming_client();

    // Create the mock metadata storage
    let mut metadata_storage = MockMetadataStorage::new();
    metadata_storage
        .expect_previous_snapshot_sync_target()
        .returning(|| Ok(None));

    // Create the bootstrapper
    let mut bootstrapper = create_bootstrapper_with_storage(
        driver_configuration,
        mock_streaming_client,
        metadata_storage,
        None,
        synced_version,
        true,
    );

    // Insert an epoch ending ledger info into the verified states of the bootstrapper
    manipulate_verified_epoch_states(&mut bootstrapper, true, true, Some(highest_version));

    // Create a global data summary
    let mut global_data_summary = create_global_summary(1);
    global_data_summary.advertised_data.synced_ledger_infos = vec![highest_ledger_info.clone()];

    // Drive progress to panic the node (we're too many versions behind)
    drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_waypoint_mismatch() {
    // Create a waypoint
    let waypoint_version = 1;
    let waypoint_epoch = 1;
    let waypoint = create_random_epoch_ending_ledger_info(waypoint_version, waypoint_epoch);

    // Create a driver configuration with the specified waypoint
    let mut driver_configuration = create_full_node_driver_configuration();
    driver_configuration.waypoint = Waypoint::new_any(waypoint.ledger_info());

    // Create the mock streaming client
    let mut mock_streaming_client = create_mock_streaming_client();
    let (mut notification_sender, data_stream_listener) = create_data_stream_listener();
    let data_stream_id = data_stream_listener.data_stream_id;
    mock_streaming_client
        .expect_get_all_epoch_ending_ledger_infos()
        .with(eq(1))
        .return_once(move |_| Ok(data_stream_listener));
    let notification_id = 100;
    mock_streaming_client
        .expect_terminate_stream_with_feedback()
        .with(
            eq(data_stream_id),
            eq(Some(NotificationAndFeedback::new(
                notification_id,
                NotificationFeedback::PayloadProofFailed,
            ))),
        )
        .return_const(Ok(()));

    // Create the bootstrapper
    let (mut bootstrapper, _) =
        create_bootstrapper(driver_configuration, mock_streaming_client, None, true);

    // Create a global data summary up to the waypoint
    let mut global_data_summary = create_global_summary(waypoint_epoch);
    global_data_summary.advertised_data.synced_ledger_infos = vec![waypoint.clone()];

    // Drive progress to initialize the epoch ending data stream
    drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap();

    // Send an invalid epoch ending payload along the stream (invalid waypoint hash)
    let invalid_ledger_info = vec![create_random_epoch_ending_ledger_info(
        waypoint_version,
        waypoint_epoch,
    )];
    let data_notification = DataNotification::new(
        notification_id,
        DataPayload::EpochEndingLedgerInfos(invalid_ledger_info),
    );
    notification_sender.send(data_notification).await.unwrap();

    // Drive progress again and ensure we get a verification error
    let error = drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap_err();
    assert_matches!(error, Error::VerificationError(_));
}

#[tokio::test]
async fn test_waypoint_must_be_verified() {
    // Create a driver configuration with a genesis waypoint and a stream timeout of 1 second
    let mut driver_configuration = create_full_node_driver_configuration();
    driver_configuration.config.max_stream_wait_time_ms = 1000;

    // Create the mock streaming client
    let mut mock_streaming_client = create_mock_streaming_client();
    let (_notification_sender, data_stream_listener) = create_data_stream_listener();
    mock_streaming_client
        .expect_get_all_epoch_ending_ledger_infos()
        .with(eq(1))
        .return_once(move |_| Ok(data_stream_listener));

    // Create the bootstrapper
    let (mut bootstrapper, _) =
        create_bootstrapper(driver_configuration, mock_streaming_client, None, true);

    // Set fetched ledger infos to true but the waypoint is still not verified
    manipulate_verified_epoch_states(&mut bootstrapper, true, false, None);

    // Create a global data summary where epoch 0 and 1 have ended
    let global_data_summary = create_global_summary(1);

    // Drive progress to initialize the epoch ending data stream
    drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap();

    // Drive progress again and verify we get a timeout error as we're still waiting
    // for epoch ending ledger infos to verify the waypoint.
    let error = drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap_err();
    assert_matches!(error, Error::DataStreamNotificationTimeout(_));
}

#[tokio::test]
async fn test_waypoint_satisfiable() {
    // Create a driver configuration with a non-genesis waypoint
    let mut driver_configuration = create_full_node_driver_configuration();
    let waypoint = create_random_epoch_ending_ledger_info(10, 1);
    driver_configuration.waypoint = Waypoint::new_any(waypoint.ledger_info());

    // Create the mock streaming client
    let mock_streaming_client = create_mock_streaming_client();

    // Create the bootstrapper
    let (mut bootstrapper, _) =
        create_bootstrapper(driver_configuration, mock_streaming_client, None, true);

    // Create an empty global data summary
    let mut global_data_summary = GlobalDataSummary::empty();

    // Drive progress and verify that no advertised data is found
    let error = drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap_err();
    assert_matches!(error, Error::UnsatisfiableWaypoint(_));

    // Update the global data summary with advertised data lower than the waypoint
    global_data_summary.advertised_data.synced_ledger_infos =
        vec![create_random_epoch_ending_ledger_info(9, 5)];

    // Verify the waypoint is not satisfiable
    let error = drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap_err();
    assert_matches!(error, Error::UnsatisfiableWaypoint(_));
}

/// Creates a bootstrapper for testing
fn create_bootstrapper(
    driver_configuration: DriverConfiguration,
    mock_streaming_client: MockStreamingClient,
    time_service: Option<TimeService>,
    expect_reset_executor: bool,
) -> (
    Bootstrapper<MockMetadataStorage, MockStorageSynchronizer, MockStreamingClient>,
    OutputFallbackHandler,
) {
    // Initialize the logger for tests
    aptos_logger::Logger::init_for_testing();

    // Create the mock storage synchronizer
    let mock_storage_synchronizer = create_ready_storage_synchronizer(expect_reset_executor);

    // Create the mock metadata storage
    let mut metadata_storage = MockMetadataStorage::new();
    metadata_storage
        .expect_previous_snapshot_sync_target()
        .returning(|| Ok(None));

    // Create the mock db reader with only genesis loaded
    let mut mock_database_reader = create_mock_db_reader();
    mock_database_reader
        .expect_get_latest_epoch_state()
        .returning(|| Ok(create_empty_epoch_state()));
    mock_database_reader
        .expect_get_latest_ledger_info()
        .returning(|| Ok(create_epoch_ending_ledger_info()));
    mock_database_reader
        .expect_get_synced_version()
        .returning(|| Ok(Some(0)));
    mock_database_reader
        .expect_get_pre_committed_version()
        .returning(|| Ok(Some(0)));

    // Create the output fallback handler
    let time_service = time_service.unwrap_or_else(TimeService::mock);
    let output_fallback_handler =
        OutputFallbackHandler::new(driver_configuration.clone(), time_service);

    // Create the bootstrapper
    let bootstrapper = Bootstrapper::new(
        driver_configuration,
        metadata_storage,
        output_fallback_handler.clone(),
        mock_streaming_client,
        Arc::new(mock_database_reader),
        mock_storage_synchronizer,
    );

    (bootstrapper, output_fallback_handler)
}

/// Creates a bootstrapper for testing with a mock metadata storage
fn create_bootstrapper_with_storage(
    driver_configuration: DriverConfiguration,
    mock_streaming_client: MockStreamingClient,
    mock_metadata_storage: MockMetadataStorage,
    latest_synced_epoch: Option<u64>,
    latest_synced_version: Version,
    expect_reset_executor: bool,
) -> Bootstrapper<MockMetadataStorage, MockStorageSynchronizer, MockStreamingClient> {
    // Initialize the logger for tests
    aptos_logger::Logger::init_for_testing();

    // Create the mock storage synchronizer
    let mock_storage_synchronizer = create_ready_storage_synchronizer(expect_reset_executor);

    // Determine the epoch state and ledger info
    let (epoch_state, epoch_ending_ledger_info) = match latest_synced_epoch {
        Some(latest_synced_epoch) => (
            create_empty_epoch_state(),
            create_epoch_ending_ledger_info_for_epoch(latest_synced_epoch, latest_synced_version),
        ),
        None => (
            create_empty_epoch_state(),
            create_epoch_ending_ledger_info(),
        ),
    };

    // Create the mock db reader and set the expectations
    let mut mock_database_reader = create_mock_db_reader();
    mock_database_reader
        .expect_get_latest_epoch_state()
        .returning(move || Ok(epoch_state.clone()));
    mock_database_reader
        .expect_get_latest_ledger_info()
        .returning(move || Ok(epoch_ending_ledger_info.clone()));
    mock_database_reader
        .expect_get_synced_version()
        .returning(move || Ok(Some(latest_synced_version)));
    mock_database_reader
        .expect_get_pre_committed_version()
        .returning(move || Ok(Some(latest_synced_version)));

    // Create the output fallback handler
    let output_fallback_handler =
        OutputFallbackHandler::new(driver_configuration.clone(), TimeService::mock());

    Bootstrapper::new(
        driver_configuration,
        mock_metadata_storage,
        output_fallback_handler,
        mock_streaming_client,
        Arc::new(mock_database_reader),
        mock_storage_synchronizer,
    )
}

/// Drives progress for the given bootstrapper. If `until_bootstrapped`
/// is true this method will continue to drive the bootstrapper until
/// bootstrapping is complete.
async fn drive_progress(
    bootstrapper: &mut Bootstrapper<
        MockMetadataStorage,
        MockStorageSynchronizer,
        MockStreamingClient,
    >,
    global_data_summary: &GlobalDataSummary,
    until_bootstrapped: bool,
) -> Result<(), Error> {
    loop {
        // Attempt to drive progress
        bootstrapper.drive_progress(global_data_summary).await?;

        // Return early if we should only drive progress once or if we've already bootstrapped
        if !until_bootstrapped || bootstrapper.is_bootstrapped() {
            return Ok(());
        }
    }
}

/// Manipulates the internal state of the verified epoch states used by
/// the given bootstrapper and inserts a verified epoch ending ledger
/// info at the specified `highest_version_to_insert` (if provided).
fn manipulate_verified_epoch_states(
    bootstrapper: &mut Bootstrapper<
        MockMetadataStorage,
        MockStorageSynchronizer,
        MockStreamingClient,
    >,
    fetched_epochs: bool,
    verified_waypoint: bool,
    highest_version_to_insert: Option<Version>,
) {
    let verified_epoch_states = bootstrapper.get_verified_epoch_states();
    if fetched_epochs {
        verified_epoch_states.set_fetched_epoch_ending_ledger_infos();
    }
    if verified_waypoint {
        verified_epoch_states.set_verified_waypoint(0);
    }
    if let Some(highest_version) = highest_version_to_insert {
        let epoch_ending_ledger_info = create_random_epoch_ending_ledger_info(highest_version, 0);
        let waypoint_ledger_info = create_random_epoch_ending_ledger_info(0, 1);
        verified_epoch_states
            .update_verified_epoch_states(
                &epoch_ending_ledger_info,
                &Waypoint::new_any(waypoint_ledger_info.ledger_info()),
            )
            .unwrap();
    }
}

/// Verifies that the receiver gets a successful notification
fn verify_bootstrap_notification(notification_receiver: oneshot::Receiver<Result<(), Error>>) {
    assert_ok!(notification_receiver.now_or_never().unwrap().unwrap());
}

/// Handles the given storage synchronizer error for the bootstrapper
async fn handle_storage_synchronizer_error(
    bootstrapper: &mut Bootstrapper<
        MockMetadataStorage,
        MockStorageSynchronizer,
        MockStreamingClient,
    >,
    notification_id: NotificationId,
    notification_feedback: NotificationFeedback,
) {
    bootstrapper
        .handle_storage_synchronizer_error(NotificationAndFeedback::new(
            notification_id,
            notification_feedback,
        ))
        .await
        .unwrap();
}

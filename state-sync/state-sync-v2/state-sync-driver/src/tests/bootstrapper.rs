// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    bootstrapper::Bootstrapper,
    driver::DriverConfiguration,
    error::Error,
    tests::{
        mocks::{
            create_mock_db_reader, create_mock_streaming_client, create_ready_storage_synchronizer,
            MockStorageSynchronizer, MockStreamingClient,
        },
        utils::{
            create_data_stream_listener, create_full_node_driver_configuration,
            create_global_summary, create_output_list_with_proof,
            create_random_epoch_ending_ledger_info, create_startup_info, create_transaction_info,
            create_transaction_list_with_proof,
        },
    },
};
use aptos_config::config::BootstrappingMode;
use aptos_data_client::GlobalDataSummary;
use aptos_types::{
    transaction::{TransactionOutputListWithProof, Version},
    waypoint::Waypoint,
};
use claim::{assert_matches, assert_none, assert_ok};
use data_streaming_service::{
    data_notification::{DataNotification, DataPayload},
    streaming_client::NotificationFeedback,
};
use futures::{channel::oneshot, FutureExt};
use mockall::{predicate::eq, Sequence};
use std::sync::Arc;

#[tokio::test]
async fn test_bootstrap_genesis_waypoint() {
    // Create a driver configuration with a genesis waypoint
    let driver_configuration = create_full_node_driver_configuration();

    // Create the mock streaming client
    let mock_streaming_client = create_mock_streaming_client();

    // Create the bootstrapper and verify it's not yet bootstrapped
    let mut bootstrapper = create_bootstrapper(driver_configuration, mock_streaming_client);
    assert!(!bootstrapper.is_bootstrapped());

    // Subscribe to a bootstrapped notification
    let (bootstrap_notification_sender, bootstrap_notification_receiver) = oneshot::channel();
    bootstrapper
        .subscribe_to_bootstrap_notifications(bootstrap_notification_sender)
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
async fn test_bootstrap_immediate_notification() {
    // Create a driver configuration with a genesis waypoint
    let driver_configuration = create_full_node_driver_configuration();

    // Create the mock streaming client
    let mock_streaming_client = create_mock_streaming_client();

    // Create the bootstrapper
    let mut bootstrapper = create_bootstrapper(driver_configuration, mock_streaming_client);

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
    let mut bootstrapper = create_bootstrapper(driver_configuration, mock_streaming_client);

    // Create a global data summary where epoch 0 and 1 have ended
    let global_data_summary = create_global_summary(1);

    // Subscribe to a bootstrapped notification
    let (bootstrap_notification_sender, bootstrap_notification_receiver) = oneshot::channel();
    bootstrapper
        .subscribe_to_bootstrap_notifications(bootstrap_notification_sender)
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

    // Create the mock streaming client
    let mut mock_streaming_client = create_mock_streaming_client();
    let mut expectation_sequence = Sequence::new();
    let (_notification_sender_1, data_stream_listener_1) = create_data_stream_listener();
    let (_notification_sender_2, data_stream_listener_2) = create_data_stream_listener();
    for data_stream_listener in [data_stream_listener_1, data_stream_listener_2] {
        mock_streaming_client
            .expect_get_all_epoch_ending_ledger_infos()
            .times(1)
            .with(eq(1))
            .return_once(move |_| Ok(data_stream_listener))
            .in_sequence(&mut expectation_sequence);
    }

    // Create the bootstrapper
    let mut bootstrapper = create_bootstrapper(driver_configuration, mock_streaming_client);

    // Create a global data summary where epoch 0 and 1 have ended
    let global_data_summary = create_global_summary(1);

    // Drive progress to initialize the epoch ending data stream
    drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap();

    // Drive progress twice and verify we get non-critical timeouts
    for _ in 0..2 {
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
async fn test_data_stream_accounts() {
    // Create test data
    let notification_id = 50043;
    let highest_version = 10000;
    let highest_ledger_info = create_random_epoch_ending_ledger_info(highest_version, 1);

    // Create a driver configuration with a genesis waypoint and account state syncing
    let mut driver_configuration = create_full_node_driver_configuration();
    driver_configuration.config.bootstrapping_mode = BootstrappingMode::DownloadLatestAccountStates;

    // Create the mock streaming client
    let mut mock_streaming_client = create_mock_streaming_client();
    let mut expectation_sequence = Sequence::new();
    let (notification_sender_1, data_stream_listener_1) = create_data_stream_listener();
    let (_notification_sender_2, data_stream_listener_2) = create_data_stream_listener();
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
            eq(notification_id),
            eq(NotificationFeedback::InvalidPayloadData),
        )
        .return_const(Ok(()));

    // Create the bootstrapper
    let mut bootstrapper = create_bootstrapper(driver_configuration, mock_streaming_client);

    // Insert an epoch ending ledger info into the verified states of the bootstrapper
    manipulate_verified_epoch_states(&mut bootstrapper, true, true, Some(highest_version));

    // Create a global data summary
    let mut global_data_summary = create_global_summary(1);
    global_data_summary.advertised_data.synced_ledger_infos = vec![highest_ledger_info.clone()];

    // Drive progress to initialize the account states stream
    drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap();

    // Send an invalid output along the stream
    let data_notification = DataNotification {
        notification_id,
        data_payload: DataPayload::TransactionOutputsWithProof(create_output_list_with_proof()),
    };
    notification_sender_1.push((), data_notification).unwrap();

    // Drive progress again and ensure we get a verification error
    let error = drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap_err();
    assert_matches!(error, Error::VerificationError(_));

    // Drive progress to initialize the account states stream
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
    let (notification_sender_1, data_stream_listener_1) = create_data_stream_listener();
    let (_notification_sender_2, data_stream_listener_2) = create_data_stream_listener();
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
            eq(notification_id),
            eq(NotificationFeedback::InvalidPayloadData),
        )
        .return_const(Ok(()));

    // Create the bootstrapper
    let mut bootstrapper = create_bootstrapper(driver_configuration, mock_streaming_client);

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
    let data_notification = DataNotification {
        notification_id,
        data_payload: DataPayload::TransactionsWithProof(create_transaction_list_with_proof()),
    };
    notification_sender_1.push((), data_notification).unwrap();

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
    let (notification_sender_1, data_stream_listener_1) = create_data_stream_listener();
    let (_notification_sender_2, data_stream_listener_2) = create_data_stream_listener();
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
            eq(notification_id),
            eq(NotificationFeedback::EmptyPayloadData),
        )
        .return_const(Ok(()));

    // Create the bootstrapper
    let mut bootstrapper = create_bootstrapper(driver_configuration, mock_streaming_client);

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
    let data_notification = DataNotification {
        notification_id,
        data_payload: DataPayload::TransactionOutputsWithProof(
            TransactionOutputListWithProof::new_empty(),
        ),
    };
    notification_sender_1.push((), data_notification).unwrap();

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
async fn test_fetch_epoch_ending_ledger_infos() {
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
    let mut bootstrapper = create_bootstrapper(driver_configuration, mock_streaming_client);

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
    let (notification_sender, data_stream_listener) = create_data_stream_listener();
    mock_streaming_client
        .expect_get_all_epoch_ending_ledger_infos()
        .with(eq(1))
        .return_once(move |_| Ok(data_stream_listener));
    let notification_id = 100;
    mock_streaming_client
        .expect_terminate_stream_with_feedback()
        .with(
            eq(notification_id),
            eq(NotificationFeedback::PayloadProofFailed),
        )
        .return_const(Ok(()));

    // Create the bootstrapper
    let mut bootstrapper = create_bootstrapper(driver_configuration, mock_streaming_client);

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
    let data_notification = DataNotification {
        notification_id,
        data_payload: DataPayload::EpochEndingLedgerInfos(invalid_ledger_info),
    };
    notification_sender.push((), data_notification).unwrap();

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
    let mut bootstrapper = create_bootstrapper(driver_configuration, mock_streaming_client);

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
    let mut bootstrapper = create_bootstrapper(driver_configuration, mock_streaming_client);

    // Create an empty global data summary
    let mut global_data_summary = GlobalDataSummary::empty();

    // Drive progress and verify that no advertised data is found
    let error = drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap_err();
    assert_matches!(error, Error::AdvertisedDataError(_));

    // Update the global data summary with advertised data lower than the waypoint
    global_data_summary.advertised_data.synced_ledger_infos =
        vec![create_random_epoch_ending_ledger_info(9, 5)];

    // Verify the waypoint is not satisfiable
    let error = drive_progress(&mut bootstrapper, &global_data_summary, false)
        .await
        .unwrap_err();
    assert_matches!(error, Error::AdvertisedDataError(_));
}

/// Creates a bootstrapper for testing
fn create_bootstrapper(
    driver_configuration: DriverConfiguration,
    mock_streaming_client: MockStreamingClient,
) -> Bootstrapper<MockStorageSynchronizer, MockStreamingClient> {
    // Initialize the logger for tests
    aptos_logger::Logger::init_for_testing();

    // Create the mock storage synchronizer
    let mock_storage_synchronizer = create_ready_storage_synchronizer();

    // Create the mock db reader with only genesis loaded
    let mut mock_database_reader = create_mock_db_reader();
    mock_database_reader
        .expect_get_startup_info()
        .returning(|| Ok(Some(create_startup_info())));
    mock_database_reader
        .expect_get_latest_transaction_info_option()
        .returning(|| Ok(Some((0, create_transaction_info()))));

    Bootstrapper::new(
        driver_configuration,
        mock_streaming_client,
        Arc::new(mock_database_reader),
        mock_storage_synchronizer,
    )
}

/// Drives progress for the given bootstrapper. If `until_bootstrapped`
/// is true this method will continue to drive the bootstrapper until
/// bootstrapping is complete.
async fn drive_progress(
    bootstrapper: &mut Bootstrapper<MockStorageSynchronizer, MockStreamingClient>,
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
    bootstrapper: &mut Bootstrapper<MockStorageSynchronizer, MockStreamingClient>,
    fetched_epochs: bool,
    verified_waypoint: bool,
    highest_version_to_insert: Option<Version>,
) {
    let verified_epoch_states = bootstrapper.get_verified_epoch_states();
    if fetched_epochs {
        verified_epoch_states.set_fetched_epoch_ending_ledger_infos();
    }
    if verified_waypoint {
        verified_epoch_states.set_verified_waypoint();
    }
    if let Some(highest_version) = highest_version_to_insert {
        let epoch_ending_ledger_info = create_random_epoch_ending_ledger_info(highest_version, 0);
        let waypoint_ledger_info = create_random_epoch_ending_ledger_info(0, 1);
        verified_epoch_states
            .verify_epoch_ending_ledger_info(
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

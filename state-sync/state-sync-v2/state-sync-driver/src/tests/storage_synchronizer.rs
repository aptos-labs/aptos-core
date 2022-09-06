// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::Error,
    metadata_storage::PersistentMetadataStorage,
    notification_handlers::{
        CommitNotification, CommitNotificationListener, CommittedTransactions,
        ErrorNotificationListener, MempoolNotificationHandler,
    },
    storage_synchronizer::{StorageSynchronizer, StorageSynchronizerInterface},
    tests::{
        mocks::{
            create_mock_db_writer, create_mock_executor, create_mock_reader_writer,
            create_mock_receiver, MockChunkExecutor,
        },
        utils::{
            create_epoch_ending_ledger_info, create_event, create_output_list_with_proof,
            create_state_value_chunk_with_proof, create_transaction,
            create_transaction_list_with_proof, verify_mempool_and_event_notification,
        },
    },
};
use anyhow::format_err;
use aptos_config::config::StateSyncDriverConfig;
use aptos_infallible::{Mutex, RwLock};
use aptos_types::{
    ledger_info::LedgerInfoWithSignatures,
    on_chain_config::ON_CHAIN_CONFIG_REGISTRY,
    transaction::{TransactionOutputListWithProof, Version},
};
use claims::assert_matches;
use data_streaming_service::data_notification::NotificationId;
use event_notifications::EventSubscriptionService;
use executor_types::ChunkCommitNotification;
use futures::StreamExt;
use mempool_notifications::MempoolNotificationListener;
use mockall::predicate::always;
use std::{sync::Arc, time::Duration};
use storage_interface::DbReaderWriter;
use tokio::task::JoinHandle;

#[tokio::test(flavor = "multi_thread")]
async fn test_apply_transaction_outputs() {
    // Create test data
    let transaction_to_commit = create_transaction();
    let event_to_commit = create_event(None);

    // Setup the mock executor
    let mut chunk_executor = create_mock_executor();
    chunk_executor
        .expect_apply_chunk()
        .with(always(), always(), always())
        .returning(|_, _, _| Ok(()));
    let expected_commit_return = Ok(ChunkCommitNotification {
        committed_events: vec![event_to_commit.clone()],
        committed_transactions: vec![transaction_to_commit.clone()],
        reconfiguration_occurred: false,
    });
    chunk_executor
        .expect_commit_chunk()
        .return_once(move || expected_commit_return);

    // Create the storage synchronizer
    let (_, _, event_subscription_service, mut mempool_listener, mut storage_synchronizer, _, _) =
        create_storage_synchronizer(chunk_executor, create_mock_reader_writer(None, None));

    // Subscribe to the expected event
    let mut event_listener = event_subscription_service
        .lock()
        .subscribe_to_events(vec![*event_to_commit.key()])
        .unwrap();

    // Attempt to apply a chunk of outputs
    storage_synchronizer
        .apply_transaction_outputs(
            0,
            create_output_list_with_proof(),
            create_epoch_ending_ledger_info(),
            None,
        )
        .await
        .unwrap();

    // Verify we get a mempool and event notification. Also verify that there's no pending data.
    verify_mempool_and_event_notification(
        Some(&mut event_listener),
        &mut mempool_listener,
        vec![transaction_to_commit],
        vec![event_to_commit],
    )
    .await;
    verify_no_pending_data(&storage_synchronizer);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_apply_transaction_outputs_error() {
    // Setup the mock executor
    let mut chunk_executor = create_mock_executor();
    chunk_executor
        .expect_apply_chunk()
        .with(always(), always(), always())
        .returning(|_, _, _| Err(format_err!("Failed to apply chunk!")));

    // Create the storage synchronizer
    let (_, mut error_listener, _, _, mut storage_synchronizer, _, _) =
        create_storage_synchronizer(chunk_executor, create_mock_reader_writer(None, None));

    // Attempt to apply a chunk of outputs
    let notification_id = 100;
    storage_synchronizer
        .apply_transaction_outputs(
            notification_id,
            create_output_list_with_proof(),
            create_epoch_ending_ledger_info(),
            None,
        )
        .await
        .unwrap();

    // Verify we get an error notification and that there's no pending data
    verify_error_notification(&mut error_listener, notification_id).await;
    verify_no_pending_data(&storage_synchronizer);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_commit_chunk_error() {
    // Setup the mock executor
    let mut chunk_executor = create_mock_executor();
    chunk_executor
        .expect_execute_chunk()
        .with(always(), always(), always())
        .returning(|_, _, _| Ok(()));
    chunk_executor
        .expect_commit_chunk()
        .return_once(|| Err(format_err!("Failed to commit chunk!")));

    // Create the storage synchronizer
    let (_, mut error_listener, _, _, mut storage_synchronizer, _, _) =
        create_storage_synchronizer(chunk_executor, create_mock_reader_writer(None, None));

    // Attempt to execute a chunk of transactions
    let notification_id = 100;
    storage_synchronizer
        .execute_transactions(
            notification_id,
            create_transaction_list_with_proof(),
            create_epoch_ending_ledger_info(),
            None,
        )
        .await
        .unwrap();

    // Verify we get an error notification and that there's no pending data
    verify_error_notification(&mut error_listener, notification_id).await;
    verify_no_pending_data(&storage_synchronizer);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_execute_transactions() {
    // Create test data
    let transaction_to_commit = create_transaction();
    let event_to_commit = create_event(None);

    // Setup the mock executor
    let mut chunk_executor = create_mock_executor();
    chunk_executor
        .expect_execute_chunk()
        .with(always(), always(), always())
        .returning(|_, _, _| Ok(()));
    let expected_commit_return = Ok(ChunkCommitNotification {
        committed_events: vec![event_to_commit.clone()],
        committed_transactions: vec![transaction_to_commit.clone()],
        reconfiguration_occurred: false,
    });
    chunk_executor
        .expect_commit_chunk()
        .return_once(move || expected_commit_return);

    // Create the storage synchronizer
    let (_, _, event_subscription_service, mut mempool_listener, mut storage_synchronizer, _, _) =
        create_storage_synchronizer(chunk_executor, create_mock_reader_writer(None, None));

    // Subscribe to the expected event
    let mut event_listener = event_subscription_service
        .lock()
        .subscribe_to_events(vec![*event_to_commit.key()])
        .unwrap();

    // Attempt to execute a chunk of transactions
    storage_synchronizer
        .execute_transactions(
            0,
            create_transaction_list_with_proof(),
            create_epoch_ending_ledger_info(),
            None,
        )
        .await
        .unwrap();

    // Verify we get a mempool and event notification. Also verify that there's no pending data.
    verify_mempool_and_event_notification(
        Some(&mut event_listener),
        &mut mempool_listener,
        vec![transaction_to_commit],
        vec![event_to_commit],
    )
    .await;
    verify_no_pending_data(&storage_synchronizer);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_execute_transactions_error() {
    // Setup the mock executor
    let mut chunk_executor = create_mock_executor();
    chunk_executor
        .expect_execute_chunk()
        .with(always(), always(), always())
        .returning(|_, _, _| Err(format_err!("Failed to execute chunk!")));

    // Create the storage synchronizer
    let (_, mut error_listener, _, _, mut storage_synchronizer, _, _) =
        create_storage_synchronizer(chunk_executor, create_mock_reader_writer(None, None));

    // Attempt to execute a chunk of transactions
    let notification_id = 100;
    storage_synchronizer
        .execute_transactions(
            notification_id,
            create_transaction_list_with_proof(),
            create_epoch_ending_ledger_info(),
            None,
        )
        .await
        .unwrap();

    // Verify we get an error notification and that there's no pending data
    verify_error_notification(&mut error_listener, notification_id).await;
    verify_no_pending_data(&storage_synchronizer);
}

#[tokio::test(flavor = "multi_thread")]
#[should_panic]
async fn test_initialize_state_synchronizer_missing_info() {
    // Create test data that is missing transaction infos
    let mut output_list_with_proof = create_output_list_with_proof();
    output_list_with_proof.proof.transaction_infos = vec![]; // This is invalid!

    // Create the storage synchronizer
    let (_, _, _, _, mut storage_synchronizer, _, _) = create_storage_synchronizer(
        create_mock_executor(),
        create_mock_reader_writer(None, None),
    );

    // Initialize the state synchronizer
    let state_synchronizer_handle = storage_synchronizer
        .initialize_state_synchronizer(
            vec![create_epoch_ending_ledger_info()],
            create_epoch_ending_ledger_info(),
            output_list_with_proof,
        )
        .unwrap();

    // The handler should panic as it was given invalid data
    state_synchronizer_handle.await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
#[should_panic]
async fn test_initialize_state_synchronizer_receiver_error() {
    // Setup the mock db writer. The db writer should always fail.
    let mut db_writer = create_mock_db_writer();
    db_writer
        .expect_get_state_snapshot_receiver()
        .returning(|_, _| Err(format_err!("Failed to get snapshot receiver!")));

    // Create the storage synchronizer
    let (_, _, _, _, mut storage_synchronizer, _, _) = create_storage_synchronizer(
        create_mock_executor(),
        create_mock_reader_writer(None, Some(db_writer)),
    );

    // Initialize the state synchronizer
    let state_synchronizer_handle = storage_synchronizer
        .initialize_state_synchronizer(
            vec![create_epoch_ending_ledger_info()],
            create_epoch_ending_ledger_info(),
            create_output_list_with_proof(),
        )
        .unwrap();

    // The handler should panic as storage failed to return a snapshot receiver
    state_synchronizer_handle.await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn test_save_states_completion() {
    // Create test data
    let target_ledger_info = create_epoch_ending_ledger_info();
    let epoch_change_proofs = [
        create_epoch_ending_ledger_info(),
        create_epoch_ending_ledger_info(),
        target_ledger_info.clone(),
    ];
    let output_list_with_proof = create_output_list_with_proof();

    // Setup the mock snapshot receiver
    let mut snapshot_receiver = create_mock_receiver();
    snapshot_receiver
        .expect_add_chunk()
        .with(always(), always())
        .returning(|_, _| Ok(()));
    snapshot_receiver.expect_finish_box().returning(|| Ok(()));

    // Setup the mock executor
    let mut chunk_executor = create_mock_executor();
    chunk_executor.expect_reset().returning(|| Ok(()));

    // Setup the mock db writer
    let mut db_writer = create_mock_db_writer();
    db_writer
        .expect_get_state_snapshot_receiver()
        .with(always(), always())
        .return_once(move |_, _| Ok(Box::new(snapshot_receiver)));
    let target_ledger_info_clone = target_ledger_info.clone();
    let output_list_with_proof_clone = output_list_with_proof.clone();
    let epoch_change_proofs_clone = epoch_change_proofs.clone();
    db_writer
        .expect_finalize_state_snapshot()
        .withf(
            move |version: &Version,
                  output_with_proof: &TransactionOutputListWithProof,
                  ledger_infos: &[LedgerInfoWithSignatures]| {
                version == &target_ledger_info_clone.ledger_info().version()
                    && output_with_proof == &output_list_with_proof_clone
                    && ledger_infos == epoch_change_proofs_clone
            },
        )
        .returning(|_, _, _| Ok(()));

    // Create the storage synchronizer
    let (mut commit_listener, _, _, _, mut storage_synchronizer, _, _) =
        create_storage_synchronizer(
            chunk_executor,
            create_mock_reader_writer(None, Some(db_writer)),
        );

    // Subscribe to the expected event
    let expected_event = output_list_with_proof.transactions_and_outputs[0]
        .1
        .events()[0]
        .clone();

    // Initialize the state synchronizer
    let state_synchronizer_handle = storage_synchronizer
        .initialize_state_synchronizer(
            epoch_change_proofs.to_vec(),
            target_ledger_info,
            output_list_with_proof.clone(),
        )
        .unwrap();

    // Save multiple state chunks (including the last chunk)
    storage_synchronizer
        .save_state_values(0, create_state_value_chunk_with_proof(false))
        .unwrap();
    storage_synchronizer
        .save_state_values(1, create_state_value_chunk_with_proof(true))
        .unwrap();

    // Verify we get a commit notification
    let expected_transaction = output_list_with_proof.transactions_and_outputs[0].0.clone();
    let expected_committed_transactions = CommittedTransactions {
        events: vec![expected_event.clone()],
        transactions: vec![expected_transaction.clone()],
    };

    verify_snapshot_commit_notification(
        &mut commit_listener,
        expected_committed_transactions.clone(),
    )
    .await;

    // The handler should return as we've finished writing all states
    state_synchronizer_handle.await.unwrap();
    verify_no_pending_data(&storage_synchronizer);
}

#[tokio::test(flavor = "multi_thread")]
#[should_panic]
async fn test_save_states_dropped_error_listener() {
    // Setup the mock snapshot receiver
    let mut snapshot_receiver = create_mock_receiver();
    snapshot_receiver
        .expect_add_chunk()
        .with(always(), always())
        .returning(|_, _| Ok(()));

    // Setup the mock db writer
    let mut db_writer = create_mock_db_writer();
    db_writer
        .expect_get_state_snapshot_receiver()
        .with(always(), always())
        .return_once(move |_, _| Ok(Box::new(snapshot_receiver)));

    // Create the storage synchronizer (drop all listeners)
    let (_, _, _, _, mut storage_synchronizer, _, _) = create_storage_synchronizer(
        create_mock_executor(),
        create_mock_reader_writer(None, Some(db_writer)),
    );

    // Initialize the state synchronizer
    let state_synchronizer_handle = storage_synchronizer
        .initialize_state_synchronizer(
            vec![create_epoch_ending_ledger_info()],
            create_epoch_ending_ledger_info(),
            create_output_list_with_proof(),
        )
        .unwrap();

    // Save the last state chunk
    let notification_id = 0;
    storage_synchronizer
        .save_state_values(notification_id, create_state_value_chunk_with_proof(true))
        .unwrap();

    // The handler should panic as the commit listener was dropped
    state_synchronizer_handle.await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn test_save_states_invalid_chunk() {
    // Setup the mock snapshot receiver to always return errors
    let mut snapshot_receiver = create_mock_receiver();
    snapshot_receiver
        .expect_add_chunk()
        .with(always(), always())
        .returning(|_, _| Err(format_err!("Invalid chunk!")));

    // Setup the mock db writer
    let mut db_writer = create_mock_db_writer();
    db_writer
        .expect_get_state_snapshot_receiver()
        .with(always(), always())
        .return_once(move |_, _| Ok(Box::new(snapshot_receiver)));

    // Create the storage synchronizer
    let (_, mut error_listener, _, _, mut storage_synchronizer, _, _) = create_storage_synchronizer(
        create_mock_executor(),
        create_mock_reader_writer(None, Some(db_writer)),
    );

    // Initialize the state synchronizer
    let _ = storage_synchronizer
        .initialize_state_synchronizer(
            vec![create_epoch_ending_ledger_info()],
            create_epoch_ending_ledger_info(),
            create_output_list_with_proof(),
        )
        .unwrap();

    // Save a state chunk and verify we get an error notification
    let notification_id = 0;
    storage_synchronizer
        .save_state_values(notification_id, create_state_value_chunk_with_proof(false))
        .unwrap();
    verify_error_notification(&mut error_listener, notification_id).await;
}

#[test]
#[should_panic]
fn test_save_states_without_initialize() {
    // Create the storage synchronizer
    let (_, _, _, _, mut storage_synchronizer, _, _) = create_storage_synchronizer(
        create_mock_executor(),
        create_mock_reader_writer(None, None),
    );

    // Attempting to save the states should panic as the state
    // synchronizer was not initialized!
    let _ = storage_synchronizer.save_state_values(0, create_state_value_chunk_with_proof(false));
}

/// Creates a storage synchronizer for testing
fn create_storage_synchronizer(
    mock_chunk_executor: MockChunkExecutor,
    mock_reader_writer: DbReaderWriter,
) -> (
    CommitNotificationListener,
    ErrorNotificationListener,
    Arc<Mutex<EventSubscriptionService>>,
    MempoolNotificationListener,
    StorageSynchronizer<MockChunkExecutor, PersistentMetadataStorage>,
    JoinHandle<()>,
    JoinHandle<()>,
) {
    aptos_logger::Logger::init_for_testing();

    // Create the notification channels
    let (commit_notification_sender, commit_notification_listener) =
        CommitNotificationListener::new();
    let (error_notification_sender, error_notification_listener) = ErrorNotificationListener::new();

    // Create the event subscription service
    let event_subscription_service = Arc::new(Mutex::new(EventSubscriptionService::new(
        ON_CHAIN_CONFIG_REGISTRY,
        Arc::new(RwLock::new(mock_reader_writer.clone())),
    )));

    // Create the mempool notification handler
    let (mempool_notification_sender, mempool_notification_listener) =
        mempool_notifications::new_mempool_notifier_listener_pair();
    let mempool_notification_handler = MempoolNotificationHandler::new(mempool_notification_sender);

    // Create the metadata storage
    let db_path = aptos_temppath::TempPath::new();
    let metadata_storage = PersistentMetadataStorage::new(db_path.path());

    // Create the storage synchronizer
    let (storage_synchronizer, executor_handle, committer_handle) = StorageSynchronizer::new(
        StateSyncDriverConfig::default(),
        Arc::new(mock_chunk_executor),
        commit_notification_sender,
        error_notification_sender,
        event_subscription_service.clone(),
        mempool_notification_handler,
        metadata_storage,
        mock_reader_writer,
        None,
    );

    (
        commit_notification_listener,
        error_notification_listener,
        event_subscription_service,
        mempool_notification_listener,
        storage_synchronizer,
        executor_handle,
        committer_handle,
    )
}

/// Verifies that the expected snapshot commit notification is received by the listener
async fn verify_snapshot_commit_notification(
    commit_listener: &mut CommitNotificationListener,
    expected_committed_transactions: CommittedTransactions,
) {
    let CommitNotification::CommittedStateSnapshot(committed_snapshot) =
        commit_listener.select_next_some().await;
    assert_eq!(
        committed_snapshot.committed_transaction,
        expected_committed_transactions
    );
}

/// Verifies that the expected error notification is received by the listener
async fn verify_error_notification(
    error_listener: &mut ErrorNotificationListener,
    expected_notification_id: NotificationId,
) {
    let error_notification = error_listener.select_next_some().await;
    assert_eq!(error_notification.notification_id, expected_notification_id);
    assert_matches!(error_notification.error, Error::UnexpectedError(_));
}

/// Verifies that no pending data remains in the storage synchronizer.
/// Note: due to asynchronous execution, we might need to wait some
/// time for the pipelines to drain.
fn verify_no_pending_data(
    storage_synchronizer: &StorageSynchronizer<MockChunkExecutor, PersistentMetadataStorage>,
) {
    let max_drain_time_secs = 10;
    for _ in 0..max_drain_time_secs {
        if !storage_synchronizer.pending_storage_data() {
            return;
        }
        std::thread::sleep(Duration::from_secs(1));
    }
    panic!("Timed-out waiting for the storage synchronizer to drain!");
}

// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    driver_factory::DriverFactory,
    metadata_storage::PersistentMetadataStorage,
    notification_handlers::CommitNotification,
    tests::utils::{
        create_event, create_ledger_info_at_version, create_reconfig_event, create_transaction,
        verify_commit_notification,
    },
};
use velor_config::config::{NodeConfig, RoleType, StateSyncDriverConfig};
use velor_consensus_notifications::{ConsensusNotificationSender, ConsensusNotifier};
use velor_data_client::client::VelorDataClient;
use velor_data_streaming_service::streaming_client::new_streaming_service_client_listener_pair;
use velor_db::VelorDB;
use velor_event_notifications::{
    DbBackedOnChainConfig, EventNotificationListener, EventSubscriptionService,
    ReconfigNotificationListener,
};
use velor_executor::chunk_executor::ChunkExecutor;
use velor_executor_test_helpers::bootstrap_genesis;
use velor_infallible::RwLock;
use velor_mempool_notifications::MempoolNotificationListener;
use velor_network::application::{interface::NetworkClient, storage::PeersAndMetadata};
use velor_storage_interface::DbReaderWriter;
use velor_storage_service_client::StorageServiceClient;
use velor_storage_service_notifications::StorageServiceNotificationListener;
use velor_time_service::TimeService;
use velor_types::{
    event::EventKey,
    transaction::{Transaction, WriteSetPayload},
    waypoint::Waypoint,
};
use velor_vm::velor_vm::VelorVMBlockExecutor;
use claims::{assert_err, assert_none};
use futures::{channel::mpsc::UnboundedSender, FutureExt, SinkExt, StreamExt};
use ntest::timeout;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::time::sleep;

#[tokio::test(flavor = "multi_thread")]
#[timeout(120_000)]
async fn test_auto_bootstrapping() {
    // Create a driver for a validator with a waypoint at version 0
    let (validator_driver, _, consensus_notifier, _, _, _, _, time_service) =
        create_validator_driver(None).await;

    // Verify auto-bootstrapping hasn't happened yet
    let result = consensus_notifier
        .sync_to_target(create_ledger_info_at_version(0))
        .await;
    assert_err!(result);

    // Wait for validator auto bootstrapping
    wait_for_auto_bootstrapping(validator_driver, time_service).await;
}

#[tokio::test]
#[timeout(120_000)]
async fn test_consensus_commit_notification() {
    // Create a driver for a full node
    let (_full_node_driver, _, consensus_notifier, _, _, _, _, _) =
        create_full_node_driver(None).await;

    // Verify that full nodes can't process commit notifications
    let result = consensus_notifier
        .notify_new_commit(vec![create_transaction()], vec![])
        .await;
    assert_err!(result);

    // Create a driver for a validator with a waypoint at version 0
    let (_validator_driver, _, consensus_notifier, _, _, _, _, _) =
        create_validator_driver(None).await;

    // Send a new commit notification and verify the node isn't bootstrapped
    let result = consensus_notifier
        .notify_new_commit(vec![create_transaction()], vec![])
        .await;
    assert_err!(result);
}

#[tokio::test(flavor = "multi_thread")]
#[timeout(120_000)]
async fn test_snapshot_commit_notifications() {
    // Create a driver for a validator with a waypoint at version 0
    let subscription_event_key = EventKey::random();
    let (
        validator_driver,
        mut commit_notification_sender,
        _,
        mut mempool_listener,
        _,
        mut event_listener,
        mut storage_service_listener,
        time_service,
    ) = create_validator_driver(Some(vec![subscription_event_key])).await;

    // Wait for validator auto bootstrapping
    wait_for_auto_bootstrapping(validator_driver, time_service).await;

    // Create commit data for testing
    let transactions = vec![create_transaction(), create_transaction()];
    let events = vec![
        create_event(Some(subscription_event_key)),
        create_event(Some(subscription_event_key)),
    ];

    // Send a new commit notification to the driver (from the snapshot receiver)
    let commit_notification = CommitNotification::new_committed_state_snapshot(
        events.clone(),
        transactions.clone(),
        10_000,
        0,
    );
    commit_notification_sender
        .send(commit_notification)
        .await
        .unwrap();

    // Verify that all components are notified
    verify_commit_notification(
        Some(&mut event_listener),
        &mut mempool_listener,
        &mut storage_service_listener,
        transactions,
        events,
        0,
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
#[timeout(120_000)]
async fn test_consensus_commit_notifications() {
    // Create a driver for a validator with a waypoint at version 0
    let subscription_event_key = EventKey::random();
    let (
        validator_driver,
        _,
        consensus_notifier,
        mut mempool_listener,
        _,
        mut event_listener,
        mut storage_service_listener,
        time_service,
    ) = create_validator_driver(Some(vec![subscription_event_key])).await;

    // Wait for validator auto bootstrapping
    wait_for_auto_bootstrapping(validator_driver, time_service).await;

    // Create commit data for testing
    let transactions = vec![create_transaction(), create_transaction()];
    let events = vec![
        create_event(Some(subscription_event_key)),
        create_event(Some(subscription_event_key)),
    ];

    // Send a new consensus commit notification to the driver
    let committed_transactions = transactions.clone();
    let committed_events = events.clone();
    let join_handle = tokio::spawn(async move {
        consensus_notifier
            .notify_new_commit(committed_transactions, committed_events)
            .await
            .unwrap();
    });

    // Verify that all components are notified
    verify_commit_notification(
        Some(&mut event_listener),
        &mut mempool_listener,
        &mut storage_service_listener,
        transactions,
        events,
        0,
    )
    .await;

    // Ensure the consensus notification is acknowledged
    join_handle.await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
#[timeout(120_000)]
async fn test_reconfiguration_notifications() {
    // Create a driver for a validator with a waypoint at version 0
    let (
        validator_driver,
        _,
        consensus_notifier,
        mut mempool_listener,
        mut reconfig_listener,
        _,
        mut storage_service_listener,
        time_service,
    ) = create_validator_driver(None).await;

    // Wait for validator auto bootstrapping
    wait_for_auto_bootstrapping(validator_driver, time_service).await;
    let reconfig_event = create_reconfig_event();

    // Test different events
    for event in [
        create_event(Some(EventKey::random())),
        reconfig_event.clone(),
        create_event(Some(EventKey::random())),
        reconfig_event.clone(),
    ] {
        // Create commit data for testing
        let transactions = vec![create_transaction(), create_transaction()];
        let events = vec![event.clone()];

        // Send a new consensus commit notification to the driver
        let committed_transactions = transactions.clone();
        let committed_events = events.clone();
        let consensus_notifier = consensus_notifier.clone();
        let join_handle = tokio::spawn(async move {
            consensus_notifier
                .notify_new_commit(committed_transactions, committed_events)
                .await
                .unwrap();
        });

        // Verify that mempool and the storage service are notified
        verify_commit_notification(
            None,
            &mut mempool_listener,
            &mut storage_service_listener,
            transactions,
            events,
            0,
        )
        .await;

        // Verify the reconfiguration listener is notified if a reconfiguration occurred
        if event == reconfig_event {
            let reconfig_notification = reconfig_listener.select_next_some().await;
            assert_eq!(reconfig_notification.version, 0);
        } else {
            assert_none!(reconfig_listener.select_next_some().now_or_never());
        }

        // Ensure the consensus notification is acknowledged
        join_handle.await.unwrap();
    }
}

#[tokio::test]
#[timeout(120_000)]
async fn test_consensus_sync_request() {
    // Create a driver for a full node
    let (_full_node_driver, _, consensus_notifier, _, _, _, _, _) =
        create_full_node_driver(None).await;

    // Verify that full nodes can't process sync requests
    let result = consensus_notifier
        .sync_to_target(create_ledger_info_at_version(0))
        .await;
    assert_err!(result);

    // Create a driver for a validator with a waypoint at version 0
    let (_validator_driver, _, consensus_notifier, _, _, _, _, _) =
        create_validator_driver(None).await;

    // Send a new sync request and verify the node isn't bootstrapped
    let result = consensus_notifier
        .sync_to_target(create_ledger_info_at_version(0))
        .await;
    assert_err!(result);
}

/// Creates a state sync driver for a validator node
async fn create_validator_driver(
    event_key_subscriptions: Option<Vec<EventKey>>,
) -> (
    DriverFactory,
    UnboundedSender<CommitNotification>,
    ConsensusNotifier,
    MempoolNotificationListener,
    ReconfigNotificationListener<DbBackedOnChainConfig>,
    EventNotificationListener,
    StorageServiceNotificationListener,
    TimeService,
) {
    let mut node_config = NodeConfig::default();
    node_config.base.role = RoleType::Validator;
    node_config
        .state_sync
        .state_sync_driver
        .enable_auto_bootstrapping = true;

    create_driver_for_tests(node_config, Waypoint::default(), event_key_subscriptions).await
}

/// Creates a state sync driver for a full node
async fn create_full_node_driver(
    event_key_subscriptions: Option<Vec<EventKey>>,
) -> (
    DriverFactory,
    UnboundedSender<CommitNotification>,
    ConsensusNotifier,
    MempoolNotificationListener,
    ReconfigNotificationListener<DbBackedOnChainConfig>,
    EventNotificationListener,
    StorageServiceNotificationListener,
    TimeService,
) {
    let mut node_config = NodeConfig::default();
    node_config.base.role = RoleType::FullNode;

    create_driver_for_tests(node_config, Waypoint::default(), event_key_subscriptions).await
}

/// Creates a state sync driver using the given node config and waypoint
async fn create_driver_for_tests(
    node_config: NodeConfig,
    waypoint: Waypoint,
    event_key_subscriptions: Option<Vec<EventKey>>,
) -> (
    DriverFactory,
    UnboundedSender<CommitNotification>,
    ConsensusNotifier,
    MempoolNotificationListener,
    ReconfigNotificationListener<DbBackedOnChainConfig>,
    EventNotificationListener,
    StorageServiceNotificationListener,
    TimeService,
) {
    // Initialize the logger for tests
    velor_logger::Logger::init_for_testing();

    // Create test velor database
    let db_path = velor_temppath::TempPath::new();
    db_path.create_as_dir().unwrap();
    let (_, db_rw) = DbReaderWriter::wrap(VelorDB::new_for_test(db_path.path()));

    // Bootstrap the genesis transaction
    let (genesis, _) = velor_vm_genesis::test_genesis_change_set_and_validators(Some(1));
    let genesis_txn = Transaction::GenesisTransaction(WriteSetPayload::Direct(genesis));
    bootstrap_genesis::<VelorVMBlockExecutor>(&db_rw, &genesis_txn).unwrap();

    // Create the event subscription service and subscribe to events and reconfigurations
    let mut event_subscription_service =
        EventSubscriptionService::new(Arc::new(RwLock::new(db_rw.clone())));
    let mut reconfiguration_subscriber = event_subscription_service
        .subscribe_to_reconfigurations()
        .unwrap();
    let event_key_subscriptions =
        event_key_subscriptions.unwrap_or_else(|| vec![EventKey::random()]);
    let event_subscriber = event_subscription_service
        .subscribe_to_events(event_key_subscriptions, vec![])
        .unwrap();

    // Create consensus and mempool notifiers and listeners
    let (consensus_notifier, consensus_listener) =
        velor_consensus_notifications::new_consensus_notifier_listener_pair(5000);
    let (mempool_notifier, mempool_listener) =
        velor_mempool_notifications::new_mempool_notifier_listener_pair(100);

    // Create the storage service notifier and listener
    let (storage_service_notifier, storage_service_listener) =
        velor_storage_service_notifications::new_storage_service_notifier_listener_pair();

    // Create the chunk executor
    let chunk_executor = Arc::new(ChunkExecutor::<VelorVMBlockExecutor>::new(db_rw.clone()));

    // Create a streaming service client
    let (streaming_service_client, _) = new_streaming_service_client_listener_pair();

    // Create a test velor data client
    let time_service = TimeService::mock();
    let network_client = StorageServiceClient::new(NetworkClient::new(
        vec![],
        vec![],
        HashMap::new(),
        PeersAndMetadata::new(&[]),
    ));
    let (velor_data_client, _) = VelorDataClient::new(
        node_config.state_sync.velor_data_client,
        node_config.base.clone(),
        time_service.clone(),
        db_rw.reader.clone(),
        network_client,
        None,
    );

    // Create the metadata storage
    let metadata_storage = PersistentMetadataStorage::new(db_path.path());

    // Create and spawn the driver
    let (driver_factory, commit_notification_sender) =
        DriverFactory::create_and_spawn_driver_internal(
            false,
            &node_config,
            waypoint,
            db_rw,
            chunk_executor,
            mempool_notifier,
            storage_service_notifier,
            metadata_storage,
            consensus_listener,
            event_subscription_service,
            velor_data_client,
            streaming_service_client,
            time_service.clone(),
        );

    // The driver will notify reconfiguration subscribers of the initial configs.
    // Verify we've received this notification.
    reconfiguration_subscriber.select_next_some().await;

    (
        driver_factory,
        commit_notification_sender,
        consensus_notifier,
        mempool_listener,
        reconfiguration_subscriber,
        event_subscriber,
        storage_service_listener,
        time_service,
    )
}

/// Waits for node auto bootstrapping by the driver
async fn wait_for_auto_bootstrapping(validator_driver: DriverFactory, time_service: TimeService) {
    // Create the driver client and a join handle that waits on auto bootstrapping
    let driver_client = validator_driver.create_driver_client();
    let auto_bootstrapping_handle = tokio::spawn(async move {
        driver_client.notify_once_bootstrapped().await.unwrap();
    });

    // Spawn a task that continuously elapses time
    tokio::spawn(async move {
        time_service
            .into_mock()
            .advance_async(Duration::from_secs(
                StateSyncDriverConfig::default().max_connection_deadline_secs,
            ))
            .await;
        sleep(Duration::from_secs(1)).await;
    });

    // Wait until the validator is auto-bootstrapped
    auto_bootstrapping_handle.await.unwrap();
}

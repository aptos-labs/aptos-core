// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    driver_factory::DriverFactory,
    metadata_storage::PersistentMetadataStorage,
    tests::utils::{
        create_event, create_ledger_info_at_version, create_transaction,
        verify_mempool_and_event_notification,
    },
};
use aptos_config::config::{NodeConfig, RoleType};
use aptos_data_client::aptosnet::AptosNetDataClient;
use aptos_infallible::RwLock;
use aptos_time_service::TimeService;
use aptos_types::{
    event::EventKey,
    on_chain_config::{new_epoch_event_key, ON_CHAIN_CONFIG_REGISTRY},
    transaction::{Transaction, WriteSetPayload},
    waypoint::Waypoint,
};
use aptos_vm::AptosVM;
use aptosdb::AptosDB;
use claims::{assert_err, assert_none};
use consensus_notifications::{ConsensusNotificationSender, ConsensusNotifier};
use data_streaming_service::streaming_client::new_streaming_service_client_listener_pair;
use event_notifications::{
    EventNotificationListener, EventSubscriptionService, ReconfigNotificationListener,
};
use executor::chunk_executor::ChunkExecutor;
use executor_test_helpers::bootstrap_genesis;
use futures::{FutureExt, StreamExt};
use mempool_notifications::MempoolNotificationListener;
use network::application::{interface::MultiNetworkSender, storage::PeerMetadataStorage};
use std::{collections::HashMap, sync::Arc};
use storage_interface::DbReaderWriter;
use storage_service_client::StorageServiceClient;

// TODO(joshlind): extend these tests to cover more functionality!

#[tokio::test(flavor = "multi_thread")]
async fn test_auto_bootstrapping() {
    // Create a driver for a validator with a waypoint at version 0
    let (validator_driver, _, _, _, _) = create_validator_driver(None).await;

    // Wait until the validator is bootstrapped (auto-bootstrapping should occur)
    let driver_client = validator_driver.create_driver_client();
    driver_client.notify_once_bootstrapped().await.unwrap();
}

#[tokio::test]
async fn test_consensus_commit_notification() {
    // Create a driver for a full node
    let (_full_node_driver, consensus_notifier, _, _, _) = create_full_node_driver(None).await;

    // Verify that full nodes can't process commit notifications
    let result = consensus_notifier
        .notify_new_commit(vec![create_transaction()], vec![])
        .await;
    assert_err!(result);

    // Create a driver for a validator with a waypoint at version 0
    let (_validator_driver, consensus_notifier, _, _, _) = create_validator_driver(None).await;

    // Send a new commit notification and verify the node isn't bootstrapped
    let result = consensus_notifier
        .notify_new_commit(vec![create_transaction()], vec![])
        .await;
    assert_err!(result);
}

#[tokio::test]
async fn test_mempool_commit_notifications() {
    // Create a driver for a validator with a waypoint at version 0
    let subscription_event_key = EventKey::random();
    let (validator_driver, consensus_notifier, mut mempool_listener, _, mut event_listener) =
        create_validator_driver(Some(vec![subscription_event_key])).await;

    // Wait until the validator is bootstrapped
    let driver_client = validator_driver.create_driver_client();
    driver_client.notify_once_bootstrapped().await.unwrap();

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

    // Verify mempool is notified and that the event listener is notified
    verify_mempool_and_event_notification(
        Some(&mut event_listener),
        &mut mempool_listener,
        transactions,
        events,
    )
    .await;

    // Ensure the consensus notification is acknowledged
    join_handle.await.unwrap();
}

#[tokio::test]
async fn test_reconfiguration_notifications() {
    // Create a driver for a validator with a waypoint at version 0
    let (validator_driver, consensus_notifier, mut mempool_listener, mut reconfig_listener, _) =
        create_validator_driver(None).await;

    // Wait until the validator is bootstrapped
    let driver_client = validator_driver.create_driver_client();
    driver_client.notify_once_bootstrapped().await.unwrap();

    // Test different events
    let reconfiguration_event = new_epoch_event_key();
    for event_key in [
        EventKey::random(),
        reconfiguration_event,
        EventKey::random(),
        reconfiguration_event,
    ] {
        // Create commit data for testing
        let transactions = vec![create_transaction(), create_transaction()];
        let events = vec![create_event(Some(event_key))];

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

        // Verify mempool is notified
        verify_mempool_and_event_notification(None, &mut mempool_listener, transactions, events)
            .await;

        // Verify the reconfiguration listener is notified if a reconfiguration occurred
        if event_key == reconfiguration_event {
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
async fn test_consensus_sync_request() {
    // Create a driver for a full node
    let (_full_node_driver, consensus_notifier, _, _, _) = create_full_node_driver(None).await;

    // Verify that full nodes can't process sync requests
    let result = consensus_notifier
        .sync_to_target(create_ledger_info_at_version(0))
        .await;
    assert_err!(result);

    // Create a driver for a validator with a waypoint at version 0
    let (_validator_driver, consensus_notifier, _, _, _) = create_validator_driver(None).await;

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
    ConsensusNotifier,
    MempoolNotificationListener,
    ReconfigNotificationListener,
    EventNotificationListener,
) {
    let mut node_config = NodeConfig::default();
    node_config.base.role = RoleType::Validator;

    create_driver_for_tests(node_config, Waypoint::default(), event_key_subscriptions).await
}

/// Creates a state sync driver for a full node
async fn create_full_node_driver(
    event_key_subscriptions: Option<Vec<EventKey>>,
) -> (
    DriverFactory,
    ConsensusNotifier,
    MempoolNotificationListener,
    ReconfigNotificationListener,
    EventNotificationListener,
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
    ConsensusNotifier,
    MempoolNotificationListener,
    ReconfigNotificationListener,
    EventNotificationListener,
) {
    // Create test aptos database
    let db_path = aptos_temppath::TempPath::new();
    db_path.create_as_dir().unwrap();
    let (_, db_rw) = DbReaderWriter::wrap(AptosDB::new_for_test(db_path.path()));

    // Bootstrap the genesis transaction
    let (genesis, _) = vm_genesis::test_genesis_change_set_and_validators(Some(1));
    let genesis_txn = Transaction::GenesisTransaction(WriteSetPayload::Direct(genesis));
    bootstrap_genesis::<AptosVM>(&db_rw, &genesis_txn).unwrap();

    // Create the event subscription service and subscribe to events and reconfigurations
    let mut event_subscription_service = EventSubscriptionService::new(
        ON_CHAIN_CONFIG_REGISTRY,
        Arc::new(RwLock::new(db_rw.clone())),
    );
    let mut reconfiguration_subscriber = event_subscription_service
        .subscribe_to_reconfigurations()
        .unwrap();
    let event_key_subscriptions =
        event_key_subscriptions.unwrap_or_else(|| vec![EventKey::random()]);
    let event_subscriber = event_subscription_service
        .subscribe_to_events(event_key_subscriptions)
        .unwrap();

    // Create consensus and mempool notifiers and listeners
    let (consensus_notifier, consensus_listener) =
        consensus_notifications::new_consensus_notifier_listener_pair(5000);
    let (mempool_notifier, mempool_listener) =
        mempool_notifications::new_mempool_notifier_listener_pair();

    // Create the chunk executor
    let chunk_executor = Arc::new(ChunkExecutor::<AptosVM>::new(db_rw.clone()));

    // Create a streaming service client
    let (streaming_service_client, _) = new_streaming_service_client_listener_pair();

    // Create a test aptos data client
    let network_client = StorageServiceClient::new(
        MultiNetworkSender::new(HashMap::new()),
        PeerMetadataStorage::new(&[]),
    );
    let (aptos_data_client, _) = AptosNetDataClient::new(
        node_config.state_sync.aptos_data_client,
        node_config.base.clone(),
        node_config.state_sync.storage_service,
        TimeService::mock(),
        network_client,
        None,
    );

    // Create the metadata storage
    let metadata_storage = PersistentMetadataStorage::new(db_path.path());

    // Create and spawn the driver
    let driver_factory = DriverFactory::create_and_spawn_driver(
        false,
        &node_config,
        waypoint,
        db_rw,
        chunk_executor,
        mempool_notifier,
        metadata_storage,
        consensus_listener,
        event_subscription_service,
        aptos_data_client,
        streaming_service_client,
    );

    // The driver will notify reconfiguration subscribers of the initial configs.
    // Verify we've received this notification.
    reconfiguration_subscriber.select_next_some().await;

    (
        driver_factory,
        consensus_notifier,
        mempool_listener,
        reconfiguration_subscriber,
        event_subscriber,
    )
}

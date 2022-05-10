// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    driver_factory::DriverFactory,
    tests::utils::{create_ledger_info_at_version, create_transaction},
};
use aptos_config::config::{NodeConfig, RoleType};
use aptos_data_client::aptosnet::AptosNetDataClient;
use aptos_infallible::RwLock;
use aptos_time_service::TimeService;
use aptos_types::{
    move_resource::MoveStorage,
    on_chain_config::ON_CHAIN_CONFIG_REGISTRY,
    transaction::{Transaction, WriteSetPayload},
    waypoint::Waypoint,
};
use aptos_vm::AptosVM;
use aptosdb::AptosDB;
use claim::assert_err;
use consensus_notifications::{ConsensusNotificationSender, ConsensusNotifier};
use data_streaming_service::streaming_client::new_streaming_service_client_listener_pair;
use event_notifications::{
    EventNotificationSender, EventSubscriptionService, ReconfigNotificationListener,
};
use executor::chunk_executor::ChunkExecutor;
use executor_test_helpers::bootstrap_genesis;
use mempool_notifications::MempoolNotificationListener;
use network::application::{interface::MultiNetworkSender, storage::PeerMetadataStorage};
use std::{collections::HashMap, sync::Arc};
use storage_interface::{DbReader, DbReaderWriter};
use storage_service_client::StorageServiceClient;

// TODO(joshlind): extend these tests to cover more functionality!

#[tokio::test]
async fn test_consensus_commit_notification() {
    // Create a driver for a full node
    let (_full_node_driver, consensus_notifier, _, _) = create_full_node_driver();

    // Verify that full nodes can't process commit notifications
    let result = consensus_notifier
        .notify_new_commit(vec![create_transaction()], vec![])
        .await;
    assert_err!(result);

    // Create a driver for a validator with a waypoint at version 0
    let (_validator_driver, consensus_notifier, _, _) = create_validator_driver();

    // Send a new commit notification and verify the node isn't bootstrapped
    let result = consensus_notifier
        .notify_new_commit(vec![create_transaction()], vec![])
        .await;
    assert_err!(result);
}

#[tokio::test]
async fn test_consensus_sync_request() {
    // Create a driver for a full node
    let (_full_node_driver, consensus_notifier, _, _) = create_full_node_driver();

    // Verify that full nodes can't process sync requests
    let result = consensus_notifier
        .sync_to_target(create_ledger_info_at_version(0))
        .await;
    assert_err!(result);

    // Create a driver for a validator with a waypoint at version 0
    let (_validator_driver, consensus_notifier, _, _) = create_validator_driver();

    // Send a new sync request and verify the node isn't bootstrapped
    let result = consensus_notifier
        .sync_to_target(create_ledger_info_at_version(0))
        .await;
    assert_err!(result);
}

/// Creates a state sync driver for a validator node
pub fn create_validator_driver() -> (
    DriverFactory,
    ConsensusNotifier,
    MempoolNotificationListener,
    ReconfigNotificationListener,
) {
    let mut node_config = NodeConfig::default();
    node_config.base.role = RoleType::Validator;

    create_driver_for_tests(node_config, Waypoint::default())
}

/// Creates a state sync driver for a full node
pub fn create_full_node_driver() -> (
    DriverFactory,
    ConsensusNotifier,
    MempoolNotificationListener,
    ReconfigNotificationListener,
) {
    let mut node_config = NodeConfig::default();
    node_config.base.role = RoleType::FullNode;

    create_driver_for_tests(node_config, Waypoint::default())
}

/// Creates a state sync driver using the given node config and waypoint
fn create_driver_for_tests(
    node_config: NodeConfig,
    waypoint: Waypoint,
) -> (
    DriverFactory,
    ConsensusNotifier,
    MempoolNotificationListener,
    ReconfigNotificationListener,
) {
    // Create test aptos database
    let db_path = aptos_temppath::TempPath::new();
    db_path.create_as_dir().unwrap();
    let (db, db_rw) = DbReaderWriter::wrap(AptosDB::new_for_test(db_path.path()));

    // Bootstrap the genesis transaction
    let (genesis, _) = vm_genesis::test_genesis_change_set_and_validators(Some(1));
    let genesis_txn = Transaction::GenesisTransaction(WriteSetPayload::Direct(genesis));
    bootstrap_genesis::<AptosVM>(&db_rw, &genesis_txn).unwrap();

    // Create the event subscription service and notify initial configs
    let storage: Arc<dyn DbReader> = db;
    let synced_version = (&*storage).fetch_synced_version().unwrap();
    let mut event_subscription_service = EventSubscriptionService::new(
        ON_CHAIN_CONFIG_REGISTRY,
        Arc::new(RwLock::new(db_rw.clone())),
    );
    let reconfiguration_subscriber = event_subscription_service
        .subscribe_to_reconfigurations()
        .unwrap();
    event_subscription_service
        .notify_initial_configs(synced_version)
        .unwrap();

    // Create consensus and mempool notifiers and listeners
    let (consensus_notifier, consensus_listener) =
        consensus_notifications::new_consensus_notifier_listener_pair(1000);
    let (mempool_notifier, mempool_listener) =
        mempool_notifications::new_mempool_notifier_listener_pair();

    // Create the chunk executor
    let chunk_executor = Arc::new(ChunkExecutor::<AptosVM>::new(db_rw.clone()).unwrap());

    // Create a streaming service client
    let (streaming_service_client, _) = new_streaming_service_client_listener_pair();

    // Create a test aptos data client
    let network_client = StorageServiceClient::new(
        MultiNetworkSender::new(HashMap::new()),
        PeerMetadataStorage::new(&[]),
    );
    let (aptos_data_client, _) = AptosNetDataClient::new(
        node_config.state_sync.aptos_data_client,
        node_config.state_sync.storage_service,
        TimeService::mock(),
        network_client,
        None,
    );

    // Create and spawn the driver
    let driver_factory = DriverFactory::create_and_spawn_driver(
        false,
        &node_config,
        waypoint,
        db_rw,
        chunk_executor,
        mempool_notifier,
        consensus_listener,
        event_subscription_service,
        aptos_data_client,
        streaming_service_client,
    );

    (
        driver_factory,
        consensus_notifier,
        mempool_listener,
        reconfiguration_subscriber,
    )
}

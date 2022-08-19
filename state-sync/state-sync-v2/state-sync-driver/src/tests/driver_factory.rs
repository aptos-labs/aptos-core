// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{driver_factory::DriverFactory, metadata_storage::PersistentMetadataStorage};
use aptos_config::{
    config::{
        RocksdbConfigs, DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD, NO_OP_STORAGE_PRUNER_CONFIG,
        TARGET_SNAPSHOT_SIZE,
    },
    utils::get_genesis_txn,
};
use aptos_data_client::aptosnet::AptosNetDataClient;
use aptos_genesis::test_utils::test_config;
use aptos_infallible::RwLock;
use aptos_temppath::TempPath;
use aptos_time_service::TimeService;
use aptos_types::on_chain_config::ON_CHAIN_CONFIG_REGISTRY;
use aptos_vm::AptosVM;
use aptosdb::AptosDB;
use consensus_notifications::new_consensus_notifier_listener_pair;
use data_streaming_service::streaming_client::new_streaming_service_client_listener_pair;
use event_notifications::EventSubscriptionService;
use executor::chunk_executor::ChunkExecutor;
use executor_test_helpers::bootstrap_genesis;
use futures::{FutureExt, StreamExt};
use mempool_notifications::new_mempool_notifier_listener_pair;
use network::application::{interface::MultiNetworkSender, storage::PeerMetadataStorage};
use std::{collections::HashMap, sync::Arc};
use storage_interface::DbReaderWriter;
use storage_service_client::StorageServiceClient;

#[test]
fn test_new_initialized_configs() {
    // Create a test database
    let tmp_dir = TempPath::new();
    let db = AptosDB::open(
        &tmp_dir,
        false,
        NO_OP_STORAGE_PRUNER_CONFIG,
        RocksdbConfigs::default(),
        false,
        TARGET_SNAPSHOT_SIZE,
        DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
    )
    .unwrap();
    let (_, db_rw) = DbReaderWriter::wrap(db);

    // Bootstrap the database
    let (node_config, _) = test_config();
    bootstrap_genesis::<AptosVM>(&db_rw, get_genesis_txn(&node_config).unwrap()).unwrap();

    // Create mempool and consensus notifiers
    let (mempool_notifier, _) = new_mempool_notifier_listener_pair();
    let (_, consensus_listener) = new_consensus_notifier_listener_pair(0);

    // Create the event subscription service and a reconfig subscriber
    let mut event_subscription_service = EventSubscriptionService::new(
        ON_CHAIN_CONFIG_REGISTRY,
        Arc::new(RwLock::new(db_rw.clone())),
    );
    let mut reconfiguration_subscriber = event_subscription_service
        .subscribe_to_reconfigurations()
        .unwrap();

    // Create a test streaming service client
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

    // Create the state sync driver factory
    let chunk_executor = Arc::new(ChunkExecutor::<AptosVM>::new(db_rw.clone()));
    let metadata_storage = PersistentMetadataStorage::new(tmp_dir.path());
    let _ = DriverFactory::create_and_spawn_driver(
        true,
        &node_config,
        node_config.base.waypoint.waypoint(),
        db_rw,
        chunk_executor,
        mempool_notifier,
        metadata_storage,
        consensus_listener,
        event_subscription_service,
        aptos_data_client,
        streaming_service_client,
    );

    // Verify the initial configs were notified
    assert!(reconfiguration_subscriber
        .select_next_some()
        .now_or_never()
        .is_some());
}

// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::network::ApplicationNetworkInterfaces;
use aptos_config::config::{NodeConfig, StateSyncConfig};
use aptos_consensus_notifications::ConsensusNotifier;
use aptos_data_client::{client::AptosDataClient, poller};
use aptos_data_streaming_service::{
    streaming_client::{new_streaming_service_client_listener_pair, StreamingServiceClient},
    streaming_service::DataStreamingService,
};
use aptos_event_notifications::{
    DbBackedOnChainConfig, EventNotificationListener, EventSubscriptionService,
    ReconfigNotificationListener,
};
use aptos_executor::chunk_executor::ChunkExecutor;
use aptos_infallible::RwLock;
use aptos_mempool_notifications::MempoolNotificationListener;
use aptos_network::application::{
    interface::{NetworkClient, NetworkClientInterface, NetworkServiceEvents},
    storage::PeersAndMetadata,
};
use aptos_state_sync_driver::{
    driver_factory::{DriverFactory, StateSyncRuntime},
    metadata_storage::PersistentMetadataStorage,
};
use aptos_storage_interface::{DbReader, DbReaderWriter};
use aptos_storage_service_client::StorageServiceClient;
use aptos_storage_service_notifications::StorageServiceNotificationListener;
use aptos_storage_service_server::{
    network::StorageServiceNetworkEvents, storage::StorageReader, StorageServiceServer,
};
use aptos_storage_service_types::StorageServiceMessage;
use aptos_time_service::TimeService;
use aptos_types::waypoint::Waypoint;
use aptos_vm::aptos_vm::AptosVMBlockExecutor;
use std::sync::Arc;
use tokio::runtime::Handle;

/// Creates the event subscription service and two reconfiguration
/// notification listeners (for mempool and consensus, respectively).
pub fn create_event_subscription_service(
    node_config: &NodeConfig,
    db_rw: &DbReaderWriter,
) -> (
    EventSubscriptionService,
    ReconfigNotificationListener<DbBackedOnChainConfig>,
    Option<ReconfigNotificationListener<DbBackedOnChainConfig>>,
    Option<ReconfigNotificationListener<DbBackedOnChainConfig>>,
    Option<(
        ReconfigNotificationListener<DbBackedOnChainConfig>,
        EventNotificationListener,
    )>, // (reconfig_events, dkg_start_events) for DKG
    Option<(
        ReconfigNotificationListener<DbBackedOnChainConfig>,
        EventNotificationListener,
    )>, // (reconfig_events, jwk_updated_events) for JWK consensus
) {
    // Create the event subscription service
    let mut event_subscription_service =
        EventSubscriptionService::new(Arc::new(RwLock::new(db_rw.clone())));

    // Create a reconfiguration subscription for mempool
    let mempool_reconfig_subscription = event_subscription_service
        .subscribe_to_reconfigurations()
        .expect("Mempool must subscribe to reconfigurations");

    // Create a reconfiguration subscription for consensus observer (if enabled)
    let consensus_observer_reconfig_subscription =
        if node_config.consensus_observer.observer_enabled {
            Some(
                event_subscription_service
                    .subscribe_to_reconfigurations()
                    .expect("Consensus observer must subscribe to reconfigurations"),
            )
        } else {
            None
        };

    // Create a reconfiguration subscription for consensus
    let consensus_reconfig_subscription = if node_config.base.role.is_validator() {
        Some(
            event_subscription_service
                .subscribe_to_reconfigurations()
                .expect("Consensus must subscribe to reconfigurations"),
        )
    } else {
        None
    };

    // Create reconfiguration subscriptions for DKG
    let dkg_subscriptions = if node_config.base.role.is_validator() {
        let reconfig_events = event_subscription_service
            .subscribe_to_reconfigurations()
            .expect("DKG must subscribe to reconfigurations");
        let dkg_start_events = event_subscription_service
            .subscribe_to_events(vec![], vec![
                "0x1::dkg::DKGStartEvent".to_string(),
                "0x1::chunky_dkg::ChunkyDKGStartEvent".to_string(),
            ])
            .expect("Consensus must subscribe to DKG events");
        Some((reconfig_events, dkg_start_events))
    } else {
        None
    };

    // Create reconfiguration subscriptions for JWK consensus
    let jwk_consensus_subscriptions = if node_config.base.role.is_validator() {
        let reconfig_events = event_subscription_service
            .subscribe_to_reconfigurations()
            .expect("JWK consensus must subscribe to reconfigurations");
        let jwk_updated_events = event_subscription_service
            .subscribe_to_events(vec![], vec!["0x1::jwks::ObservedJWKsUpdated".to_string()])
            .expect("JWK consensus must subscribe to DKG events");
        Some((reconfig_events, jwk_updated_events))
    } else {
        None
    };

    (
        event_subscription_service,
        mempool_reconfig_subscription,
        consensus_observer_reconfig_subscription,
        consensus_reconfig_subscription,
        dkg_subscriptions,
        jwk_consensus_subscriptions,
    )
}

/// Sets up all state sync runtimes and return the notification endpoints
pub fn start_state_sync_and_get_notification_handles(
    node_config: &NodeConfig,
    storage_network_interfaces: ApplicationNetworkInterfaces<StorageServiceMessage>,
    waypoint: Waypoint,
    event_subscription_service: EventSubscriptionService,
    db_rw: DbReaderWriter,
) -> anyhow::Result<(
    AptosDataClient,
    StateSyncRuntime,
    MempoolNotificationListener,
    ConsensusNotifier,
)> {
    // Create a single shared runtime for all state sync subsystems
    let state_sync_config = node_config.state_sync;
    let runtime = aptos_runtimes::spawn_named_runtime(
        "state-sync".into(),
        state_sync_config.num_runtime_threads,
    );
    let handle = runtime.handle().clone();

    // Get the network client and events
    let network_client = storage_network_interfaces.network_client;
    let network_service_events = storage_network_interfaces.network_service_events;

    // Start the data client
    let peers_and_metadata = network_client.get_peers_and_metadata();
    let aptos_data_client =
        setup_aptos_data_client(node_config, network_client, db_rw.reader.clone(), &handle)?;

    // Start the data streaming service
    let streaming_service_client =
        setup_data_streaming_service(state_sync_config, aptos_data_client.clone(), &handle)?;

    // Create the chunk executor and persistent storage
    let chunk_executor = Arc::new(ChunkExecutor::<AptosVMBlockExecutor>::new(db_rw.clone()));
    let metadata_storage = PersistentMetadataStorage::new(&node_config.storage.dir());

    // Create notification senders and listeners for mempool, consensus and the storage service
    let (mempool_notifier, mempool_listener) =
        aptos_mempool_notifications::new_mempool_notifier_listener_pair(
            state_sync_config
                .state_sync_driver
                .max_pending_mempool_notifications,
        );
    let (consensus_notifier, consensus_listener) =
        aptos_consensus_notifications::new_consensus_notifier_listener_pair(
            state_sync_config
                .state_sync_driver
                .commit_notification_timeout_ms,
        );
    let (storage_service_notifier, storage_service_listener) =
        aptos_storage_service_notifications::new_storage_service_notifier_listener_pair();

    // Start the state sync storage service
    setup_state_sync_storage_service(
        state_sync_config,
        peers_and_metadata,
        network_service_events,
        &db_rw,
        storage_service_listener,
        &handle,
    )?;

    // Create the state sync driver factory
    let state_sync = DriverFactory::create_and_spawn_driver(
        Some(handle),
        node_config,
        waypoint,
        db_rw,
        chunk_executor,
        mempool_notifier,
        storage_service_notifier,
        metadata_storage,
        consensus_listener,
        event_subscription_service,
        aptos_data_client.clone(),
        streaming_service_client,
        TimeService::real(),
    );

    // Create a new state sync runtime handle
    let state_sync_runtime = StateSyncRuntime::new(runtime, state_sync);

    Ok((
        aptos_data_client,
        state_sync_runtime,
        mempool_listener,
        consensus_notifier,
    ))
}

/// Sets up the data streaming service on the given runtime handle
fn setup_data_streaming_service(
    state_sync_config: StateSyncConfig,
    aptos_data_client: AptosDataClient,
    handle: &Handle,
) -> anyhow::Result<StreamingServiceClient> {
    // Create the data streaming service
    let (streaming_service_client, streaming_service_listener) =
        new_streaming_service_client_listener_pair();
    let data_streaming_service = DataStreamingService::new(
        state_sync_config.aptos_data_client,
        state_sync_config.data_streaming_service,
        aptos_data_client,
        streaming_service_listener,
        TimeService::real(),
    );

    // Spawn the data streaming service on the shared runtime
    handle.spawn(data_streaming_service.start_service());

    Ok(streaming_service_client)
}

/// Sets up the aptos data client on the given runtime handle
fn setup_aptos_data_client(
    node_config: &NodeConfig,
    network_client: NetworkClient<StorageServiceMessage>,
    storage: Arc<dyn DbReader>,
    handle: &Handle,
) -> anyhow::Result<AptosDataClient> {
    // Create the storage service client
    let storage_service_client = StorageServiceClient::new(network_client);

    // Create the data client and spawn the data poller
    let (aptos_data_client, data_summary_poller) = AptosDataClient::new(
        node_config.state_sync.aptos_data_client,
        node_config.base.clone(),
        TimeService::real(),
        storage,
        storage_service_client,
        Some(handle.clone()),
    );
    handle.spawn(poller::start_poller(data_summary_poller));

    Ok(aptos_data_client)
}

/// Sets up the state sync storage service on the given runtime handle
fn setup_state_sync_storage_service(
    config: StateSyncConfig,
    peers_and_metadata: Arc<PeersAndMetadata>,
    network_service_events: NetworkServiceEvents<StorageServiceMessage>,
    db_rw: &DbReaderWriter,
    storage_service_listener: StorageServiceNotificationListener,
    handle: &Handle,
) -> anyhow::Result<()> {
    // Spawn the state sync storage service servers on the shared runtime
    let storage_reader = StorageReader::new(
        config.storage_service,
        Arc::clone(&db_rw.reader),
        TimeService::real(),
    );
    let service = StorageServiceServer::new(
        config,
        handle.clone(),
        storage_reader,
        TimeService::real(),
        peers_and_metadata,
        StorageServiceNetworkEvents::new(network_service_events),
        storage_service_listener,
    );
    handle.spawn(service.start());

    Ok(())
}

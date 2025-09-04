// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    driver::{DriverConfiguration, StateSyncDriver},
    driver_client::{ClientNotificationListener, DriverClient, DriverNotification},
    metadata_storage::MetadataStorageInterface,
    notification_handlers::{
        CommitNotification, CommitNotificationListener, ConsensusNotificationHandler,
        ErrorNotificationListener, MempoolNotificationHandler, StorageServiceNotificationHandler,
    },
    storage_synchronizer::StorageSynchronizer,
};
use velor_config::config::NodeConfig;
use velor_consensus_notifications::ConsensusNotificationListener;
use velor_data_client::client::VelorDataClient;
use velor_data_streaming_service::streaming_client::StreamingServiceClient;
use velor_event_notifications::{EventNotificationSender, EventSubscriptionService};
use velor_executor_types::ChunkExecutorTrait;
use velor_infallible::Mutex;
use velor_mempool_notifications::MempoolNotificationSender;
use velor_storage_interface::DbReaderWriter;
use velor_storage_service_notifications::StorageServiceNotificationSender;
use velor_time_service::TimeService;
use velor_types::waypoint::Waypoint;
use futures::{
    channel::{mpsc, mpsc::UnboundedSender},
    executor::block_on,
};
use std::sync::Arc;
use tokio::runtime::Runtime;

/// Creates a new state sync driver and client
pub struct DriverFactory {
    client_notification_sender: mpsc::UnboundedSender<DriverNotification>,
    _driver_runtime: Option<Runtime>,
}

impl DriverFactory {
    /// Creates and spawns a new state sync driver and returns the factory
    pub fn create_and_spawn_driver<
        ChunkExecutor: ChunkExecutorTrait + 'static,
        MempoolNotifier: MempoolNotificationSender + 'static,
        MetadataStorage: MetadataStorageInterface + Clone + Send + Sync + 'static,
        StorageServiceNotifier: StorageServiceNotificationSender + 'static,
    >(
        create_runtime: bool,
        node_config: &NodeConfig,
        waypoint: Waypoint,
        storage: DbReaderWriter,
        chunk_executor: Arc<ChunkExecutor>,
        mempool_notification_sender: MempoolNotifier,
        storage_service_notification_sender: StorageServiceNotifier,
        metadata_storage: MetadataStorage,
        consensus_listener: ConsensusNotificationListener,
        event_subscription_service: EventSubscriptionService,
        velor_data_client: VelorDataClient,
        streaming_service_client: StreamingServiceClient,
        time_service: TimeService,
    ) -> Self {
        let (driver_factory, _) = Self::create_and_spawn_driver_internal(
            create_runtime,
            node_config,
            waypoint,
            storage,
            chunk_executor,
            mempool_notification_sender,
            storage_service_notification_sender,
            metadata_storage,
            consensus_listener,
            event_subscription_service,
            velor_data_client,
            streaming_service_client,
            time_service,
        );
        driver_factory
    }

    /// A simple utility function that creates a new state sync driver
    /// and returns both the factory as well as the commit notification
    /// sender for the driver. This is useful for testing.
    pub(crate) fn create_and_spawn_driver_internal<
        ChunkExecutor: ChunkExecutorTrait + 'static,
        MempoolNotifier: MempoolNotificationSender + 'static,
        MetadataStorage: MetadataStorageInterface + Clone + Send + Sync + 'static,
        StorageServiceNotifier: StorageServiceNotificationSender + 'static,
    >(
        create_runtime: bool,
        node_config: &NodeConfig,
        waypoint: Waypoint,
        storage: DbReaderWriter,
        chunk_executor: Arc<ChunkExecutor>,
        mempool_notification_sender: MempoolNotifier,
        storage_service_notification_sender: StorageServiceNotifier,
        metadata_storage: MetadataStorage,
        consensus_listener: ConsensusNotificationListener,
        mut event_subscription_service: EventSubscriptionService,
        velor_data_client: VelorDataClient,
        streaming_service_client: StreamingServiceClient,
        time_service: TimeService,
    ) -> (Self, UnboundedSender<CommitNotification>) {
        // Notify subscribers of the initial on-chain config values
        match storage.reader.get_latest_state_checkpoint_version() {
            Ok(Some(synced_version)) => {
                if let Err(error) =
                    event_subscription_service.notify_initial_configs(synced_version)
                {
                    panic!(
                        "Failed to notify subscribers of initial on-chain configs: {:?}",
                        error
                    )
                }
            },
            Ok(None) => {
                panic!("Latest state checkpoint version not found.")
            },
            Err(error) => panic!("Failed to fetch the initial synced version: {:?}", error),
        }

        // Create the notification handlers
        let (client_notification_sender, client_notification_receiver) = mpsc::unbounded();
        let client_notification_listener =
            ClientNotificationListener::new(client_notification_receiver);
        let (commit_notification_sender, commit_notification_listener) =
            CommitNotificationListener::new();
        let consensus_notification_handler =
            ConsensusNotificationHandler::new(consensus_listener, time_service.clone());
        let (error_notification_sender, error_notification_listener) =
            ErrorNotificationListener::new();
        let mempool_notification_handler =
            MempoolNotificationHandler::new(mempool_notification_sender);
        let storage_service_notification_handler =
            StorageServiceNotificationHandler::new(storage_service_notification_sender);

        // Create a new runtime (if required)
        let driver_runtime = if create_runtime {
            let runtime = velor_runtimes::spawn_named_runtime("sync-driver".into(), None);
            Some(runtime)
        } else {
            None
        };

        // Create the storage synchronizer
        let event_subscription_service = Arc::new(Mutex::new(event_subscription_service));
        let (storage_synchronizer, _) = StorageSynchronizer::new(
            node_config.state_sync.state_sync_driver,
            chunk_executor,
            commit_notification_sender.clone(),
            error_notification_sender,
            event_subscription_service.clone(),
            mempool_notification_handler.clone(),
            storage_service_notification_handler.clone(),
            metadata_storage.clone(),
            storage.clone(),
            driver_runtime.as_ref(),
        );

        // Create the driver configuration
        let driver_configuration = DriverConfiguration::new(
            node_config.state_sync.state_sync_driver,
            node_config.consensus_observer,
            node_config.base.role,
            waypoint,
        );

        // Create the state sync driver
        let state_sync_driver = StateSyncDriver::new(
            client_notification_listener,
            commit_notification_listener,
            consensus_notification_handler,
            driver_configuration,
            error_notification_listener,
            event_subscription_service,
            mempool_notification_handler,
            metadata_storage,
            storage_service_notification_handler,
            storage_synchronizer,
            velor_data_client,
            streaming_service_client,
            storage.reader,
            time_service,
        );

        // Spawn the driver
        if let Some(driver_runtime) = &driver_runtime {
            driver_runtime.spawn(state_sync_driver.start_driver());
        } else {
            tokio::spawn(state_sync_driver.start_driver());
        }

        // Create the driver factory
        let driver_factory = Self {
            client_notification_sender,
            _driver_runtime: driver_runtime,
        };

        (driver_factory, commit_notification_sender)
    }

    /// Returns a new client that can be used to communicate with the driver
    pub fn create_driver_client(&self) -> DriverClient {
        DriverClient::new(self.client_notification_sender.clone())
    }
}

/// A struct for holding the various runtimes required by state sync v2.
/// Note: it's useful to maintain separate runtimes because the logger
/// can prepend all logs with the runtime thread name.
pub struct StateSyncRuntimes {
    _velor_data_client: Runtime,
    state_sync: DriverFactory,
    _storage_service: Runtime,
    _streaming_service: Runtime,
}

impl StateSyncRuntimes {
    pub fn new(
        velor_data_client: Runtime,
        state_sync: DriverFactory,
        storage_service: Runtime,
        streaming_service: Runtime,
    ) -> Self {
        Self {
            _velor_data_client: velor_data_client,
            state_sync,
            _storage_service: storage_service,
            _streaming_service: streaming_service,
        }
    }

    pub fn block_until_initialized(&self) {
        let state_sync_client = self.state_sync.create_driver_client();
        block_on(state_sync_client.notify_once_bootstrapped())
            .expect("State sync v2 initialization failure");
    }
}

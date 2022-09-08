// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    driver::{DriverConfiguration, StateSyncDriver},
    driver_client::{ClientNotificationListener, DriverClient, DriverNotification},
    metadata_storage::MetadataStorageInterface,
    notification_handlers::{
        CommitNotificationListener, ConsensusNotificationHandler, ErrorNotificationListener,
        MempoolNotificationHandler,
    },
    storage_synchronizer::StorageSynchronizer,
};
use aptos_config::config::NodeConfig;
use aptos_data_client::aptosnet::AptosNetDataClient;
use aptos_infallible::Mutex;
use aptos_types::{move_resource::MoveStorage, waypoint::Waypoint};
use consensus_notifications::ConsensusNotificationListener;
use data_streaming_service::streaming_client::StreamingServiceClient;
use event_notifications::{EventNotificationSender, EventSubscriptionService};
use executor_types::ChunkExecutorTrait;
use futures::{channel::mpsc, executor::block_on};
use mempool_notifications::MempoolNotificationSender;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use storage_interface::DbReaderWriter;
use tokio::runtime::{Builder, Runtime};

/// Creates a new state sync driver and client
pub struct DriverFactory {
    client_notification_sender: mpsc::UnboundedSender<DriverNotification>,
    _driver_runtime: Option<Runtime>,
}

impl DriverFactory {
    /// Creates and spawns a new state sync driver
    pub fn create_and_spawn_driver<
        ChunkExecutor: ChunkExecutorTrait + 'static,
        MempoolNotifier: MempoolNotificationSender + 'static,
        MetadataStorage: MetadataStorageInterface + Clone + Send + Sync + 'static,
    >(
        create_runtime: bool,
        node_config: &NodeConfig,
        waypoint: Waypoint,
        storage: DbReaderWriter,
        chunk_executor: Arc<ChunkExecutor>,
        mempool_notification_sender: MempoolNotifier,
        metadata_storage: MetadataStorage,
        consensus_listener: ConsensusNotificationListener,
        mut event_subscription_service: EventSubscriptionService,
        aptos_data_client: AptosNetDataClient,
        streaming_service_client: StreamingServiceClient,
    ) -> Self {
        // Notify subscribers of the initial on-chain config values
        match (&*storage.reader).fetch_latest_state_checkpoint_version() {
            Ok(synced_version) => {
                if let Err(error) =
                    event_subscription_service.notify_initial_configs(synced_version)
                {
                    panic!(
                        "Failed to notify subscribers of initial on-chain configs: {:?}",
                        error
                    )
                }
            }
            Err(error) => panic!("Failed to fetch the initial synced version: {:?}", error),
        }

        // Create the notification handlers
        let (client_notification_sender, client_notification_receiver) = mpsc::unbounded();
        let client_notification_listener =
            ClientNotificationListener::new(client_notification_receiver);
        let (commit_notification_sender, commit_notification_listener) =
            CommitNotificationListener::new();
        let consensus_notification_handler = ConsensusNotificationHandler::new(consensus_listener);
        let (error_notification_sender, error_notification_listener) =
            ErrorNotificationListener::new();
        let mempool_notification_handler =
            MempoolNotificationHandler::new(mempool_notification_sender);

        // Create a new runtime (if required)
        let driver_runtime = if create_runtime {
            Some(
                Builder::new_multi_thread()
                    .thread_name_fn(|| {
                        static ATOMIC_ID: AtomicUsize = AtomicUsize::new(0);
                        let id = ATOMIC_ID.fetch_add(1, Ordering::SeqCst);
                        format!("sync-driver-{}", id)
                    })
                    .disable_lifo_slot()
                    .enable_all()
                    .build()
                    .expect("Failed to create state sync v2 driver runtime!"),
            )
        } else {
            None
        };

        // Create the storage synchronizer
        let event_subscription_service = Arc::new(Mutex::new(event_subscription_service));
        let (storage_synchronizer, _, _) = StorageSynchronizer::new(
            node_config.state_sync.state_sync_driver,
            chunk_executor,
            commit_notification_sender,
            error_notification_sender,
            event_subscription_service.clone(),
            mempool_notification_handler.clone(),
            metadata_storage.clone(),
            storage.clone(),
            driver_runtime.as_ref(),
        );

        // Create the driver configuration
        let driver_configuration = DriverConfiguration::new(
            node_config.state_sync.state_sync_driver,
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
            storage_synchronizer,
            aptos_data_client,
            streaming_service_client,
            storage.reader,
        );

        // Spawn the driver
        if let Some(driver_runtime) = &driver_runtime {
            driver_runtime.spawn(state_sync_driver.start_driver());
        } else {
            tokio::spawn(state_sync_driver.start_driver());
        }

        Self {
            client_notification_sender,
            _driver_runtime: driver_runtime,
        }
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
    _aptos_data_client: Runtime,
    state_sync: DriverFactory,
    _storage_service: Runtime,
    _streaming_service: Runtime,
}

impl StateSyncRuntimes {
    pub fn new(
        aptos_data_client: Runtime,
        state_sync: DriverFactory,
        storage_service: Runtime,
        streaming_service: Runtime,
    ) -> Self {
        Self {
            _aptos_data_client: aptos_data_client,
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

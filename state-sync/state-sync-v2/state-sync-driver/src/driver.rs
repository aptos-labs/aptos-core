// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    bootstrapper::Bootstrapper,
    continuous_syncer::ContinuousSyncer,
    driver_client::{ClientNotificationListener, DriverNotification},
    error::Error,
    notification_handlers::{ConsensusNotificationHandler, MempoolNotificationHandler},
    storage_synchronizer::StorageSynchronizerInterface,
    utils,
};
use consensus_notifications::{
    ConsensusCommitNotification, ConsensusNotification, ConsensusSyncNotification,
};
use data_streaming_service::streaming_client::StreamingServiceClient;
use diem_config::config::{RoleType, StateSyncDriverConfig};
use diem_data_client::DiemDataClient;
use diem_infallible::Mutex;
use diem_logger::*;
use diem_types::waypoint::Waypoint;
use event_notifications::EventSubscriptionService;
use futures::StreamExt;
use mempool_notifications::MempoolNotificationSender;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tokio_stream::wrappers::IntervalStream;

// TODO(joshlind): use structured logging!

/// The configuration of the state sync driver
#[derive(Clone)]
pub struct DriverConfiguration {
    // The config file of the driver
    pub config: StateSyncDriverConfig,

    // The role of the node
    pub role: RoleType,

    // The trusted waypoint for the node
    pub waypoint: Waypoint,
}

impl DriverConfiguration {
    pub fn new(config: StateSyncDriverConfig, role: RoleType, waypoint: Waypoint) -> Self {
        Self {
            config,
            role,
            waypoint,
        }
    }
}

/// The state sync driver that drives synchronization progress
pub struct StateSyncDriver<DataClient, MempoolNotifier, StorageSyncer> {
    // The component that manages the initial bootstrapping of the node
    bootstrapper: Bootstrapper<MempoolNotifier, StorageSyncer>,

    // The listener for client notifications
    client_notification_listener: ClientNotificationListener,

    // The handler for notifications from consensus
    consensus_notification_handler: ConsensusNotificationHandler,

    // The component that manages the continuous syncing of the node
    continuous_syncer: ContinuousSyncer<MempoolNotifier, StorageSyncer>,

    // The client for checking the global data summary of our peers
    diem_data_client: DataClient,

    // The configuration for the driver
    driver_configuration: DriverConfiguration,

    // The event subscription service to notify listeners of on-chain events
    event_subscription_service: Arc<Mutex<EventSubscriptionService>>,

    // The handler for notifications to mempool
    mempool_notification_handler: MempoolNotificationHandler<MempoolNotifier>,

    // The storage synchronizer used to update local storage
    storage_synchronizer: Arc<Mutex<StorageSyncer>>,
}

impl<
        DataClient: DiemDataClient + Send + Clone + 'static,
        MempoolNotifier: MempoolNotificationSender,
        StorageSyncer: StorageSynchronizerInterface,
    > StateSyncDriver<DataClient, MempoolNotifier, StorageSyncer>
{
    pub fn new(
        client_notification_listener: ClientNotificationListener,
        consensus_notification_handler: ConsensusNotificationHandler,
        driver_configuration: DriverConfiguration,
        event_subscription_service: EventSubscriptionService,
        mempool_notification_handler: MempoolNotificationHandler<MempoolNotifier>,
        storage_synchronizer: StorageSyncer,
        diem_data_client: DataClient,
        streaming_service_client: StreamingServiceClient,
    ) -> Self {
        let event_subscription_service = Arc::new(Mutex::new(event_subscription_service));
        let storage_synchronizer = Arc::new(Mutex::new(storage_synchronizer));
        let bootstrapper = Bootstrapper::new(
            driver_configuration.clone(),
            event_subscription_service.clone(),
            mempool_notification_handler.clone(),
            streaming_service_client.clone(),
            storage_synchronizer.clone(),
        );
        let continuous_syncer = ContinuousSyncer::new(
            driver_configuration.clone(),
            event_subscription_service.clone(),
            mempool_notification_handler.clone(),
            streaming_service_client,
            storage_synchronizer.clone(),
        );

        Self {
            bootstrapper,
            client_notification_listener,
            continuous_syncer,
            consensus_notification_handler,
            diem_data_client,
            driver_configuration,
            event_subscription_service,
            mempool_notification_handler,
            storage_synchronizer,
        }
    }

    /// Starts the state sync driver
    pub async fn start_driver(mut self) {
        let mut progress_check_interval = IntervalStream::new(interval(Duration::from_millis(
            self.driver_configuration.config.progress_check_interval_ms,
        )))
        .fuse();

        loop {
            ::futures::select! {
                notification = self.consensus_notification_handler.select_next_some() => {
                    self.handle_consensus_notification(notification).await;
                }
                notification = self.client_notification_listener.select_next_some() => {
                    self.handle_client_notification(notification);
                },
                _ = progress_check_interval.select_next_some() => {
                    self.drive_progress().await;
                }
            }
        }
    }

    /// Handles a notification sent by consensus
    async fn handle_consensus_notification(&mut self, notification: ConsensusNotification) {
        // Verify the notification: full nodes shouldn't receive notifications
        // and consensus should only send notifications after bootstrapping!
        let result = if self.driver_configuration.role == RoleType::FullNode {
            Err(Error::FullNodeConsensusNotification(format!(
                "Received consensus notification: {:?}",
                notification
            )))
        } else if !self.bootstrapper.is_bootstrapped() {
            Err(Error::BootstrapNotComplete(format!(
                "Received consensus notification: {:?}",
                notification
            )))
        } else {
            Ok(())
        };

        // Respond to consensus with any verification errors and then return
        if let Err(error) = result {
            match notification {
                ConsensusNotification::NotifyCommit(commit_notification) => {
                    let _ = self
                        .consensus_notification_handler
                        .respond_to_commit_notification(commit_notification, Err(error.clone()))
                        .await;
                }
                ConsensusNotification::SyncToTarget(sync_notification) => {
                    let _ = self
                        .consensus_notification_handler
                        .respond_to_sync_notification(sync_notification, Err(error.clone()))
                        .await;
                }
            }
            error!(
                "Error encountered when handling the consensus notification: {:?}",
                error
            );
            return;
        }

        // Handle the notification
        let result = match notification {
            ConsensusNotification::NotifyCommit(commit_notification) => {
                self.handle_consensus_commit_notification(commit_notification)
                    .await
            }
            ConsensusNotification::SyncToTarget(sync_notification) => {
                self.handle_consensus_sync_notification(sync_notification)
                    .await
            }
        };

        // Log any errors from notification handling
        if let Err(error) = result {
            error!(
                "Error encountered when handling the consensus notification: {:?}",
                error
            );
        }
    }

    /// Handles a commit notification sent by consensus
    async fn handle_consensus_commit_notification(
        &mut self,
        commit_notification: ConsensusCommitNotification,
    ) -> Result<(), Error> {
        debug!("Received a consensus commit notification!");

        // TODO(joshlind): can we get consensus to forward the events?
        // Notify mempool and the event subscription service
        let committed_transactions = commit_notification.transactions.clone();
        let reconfiguration_events = commit_notification.reconfiguration_events.clone();
        let latest_storage_summary = self.storage_synchronizer.lock().get_storage_summary()?;
        utils::notify_committed_events_and_transactions(
            &latest_storage_summary,
            self.mempool_notification_handler.clone(),
            committed_transactions,
            self.event_subscription_service.clone(),
            reconfiguration_events,
        )
        .await
        .map_err(|error| Error::EventNotificationError(format!("{:?}", error)))?;

        // Respond to consensus successfully
        self.consensus_notification_handler
            .respond_to_commit_notification(commit_notification, Ok(()))
            .await
    }

    /// Handles a consensus notification to sync to a specified target
    async fn handle_consensus_sync_notification(
        &mut self,
        sync_notification: ConsensusSyncNotification,
    ) -> Result<(), Error> {
        let latest_storage_summary = self.storage_synchronizer.lock().get_storage_summary()?;
        debug!(
            "Received a consensus sync notification! Target version: {:?}. Latest storage summary: {:?}",
            sync_notification.target, &latest_storage_summary,
        );
        // Initialize a new sync request
        let latest_storage_summary = self.storage_synchronizer.lock().get_storage_summary()?;
        self.consensus_notification_handler
            .initialize_sync_request(sync_notification, latest_storage_summary)
            .await
    }

    /// Handles a client notification sent by the driver client
    fn handle_client_notification(&mut self, notification: DriverNotification) {
        debug!("Received a notify bootstrap notification from the client!");

        // TODO(joshlind): refactor this if the client only supports one notification type!
        // Extract the bootstrap notifier channel
        let DriverNotification::NotifyOnceBootstrapped(notifier_channel) = notification;

        // Subscribe the bootstrap notifier channel
        self.bootstrapper
            .subscribe_to_bootstrap_notifications(notifier_channel);
    }

    /// Checks if the node has successfully reached the sync target
    async fn check_sync_request_progress(&mut self) -> Result<(), Error> {
        if !self.consensus_notification_handler.active_sync_request() {
            return Ok(());
        }

        let latest_storage_summary = self.storage_synchronizer.lock().get_storage_summary()?;
        self.consensus_notification_handler
            .check_sync_request_progress(&latest_storage_summary)
            .await
    }

    /// Returns true iff consensus is currently executing
    fn check_if_consensus_executing(&self) -> bool {
        self.driver_configuration.role == RoleType::Validator
            && self.bootstrapper.is_bootstrapped()
            && !self.consensus_notification_handler.active_sync_request()
    }

    /// Checks that state sync is making progress
    async fn drive_progress(&mut self) {
        trace!("Checking progress of the state sync driver!");

        // If consensus is executing, there's nothing to do.
        if self.check_if_consensus_executing() {
            trace!("Consensus is executing. There's nothing to do.");
            return;
        }

        // Fetch the global data summary and verify we have active peers
        let global_data_summary = self.diem_data_client.get_global_data_summary();
        if global_data_summary.is_empty() {
            trace!("The global data summary is empty! It's likely that we have no active peers.");
            return;
        }

        // Check the progress of any sync requests
        if let Err(error) = self.check_sync_request_progress().await {
            error!(
                "Error found when checking the sync request progress: {:?}",
                error
            );
        }

        // If consensus is executing, there's nothing to do
        if self.check_if_consensus_executing() {
            trace!("Consensus is executing. There's nothing to do.");
            return;
        }

        // Drive progress depending on if we're bootstrapping or continuously syncing
        if self.bootstrapper.is_bootstrapped() {
            let consensus_sync_request = self
                .consensus_notification_handler
                .get_consensus_sync_request();
            if let Err(error) = self
                .continuous_syncer
                .drive_progress(consensus_sync_request)
                .await
            {
                error!(
                    "Error found when driving progress of the continuous syncer: {:?}",
                    error
                );
            }
        } else if let Err(error) = self.bootstrapper.drive_progress(&global_data_summary).await {
            error!(
                "Error found when checking the bootstrapper progress: {:?}",
                error
            );
        };
    }
}

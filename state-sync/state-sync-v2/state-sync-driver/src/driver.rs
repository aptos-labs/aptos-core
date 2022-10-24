// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    bootstrapper::Bootstrapper,
    continuous_syncer::ContinuousSyncer,
    driver_client::{ClientNotificationListener, DriverNotification},
    error::Error,
    logging::{LogEntry, LogSchema},
    metadata_storage::MetadataStorageInterface,
    metrics,
    metrics::ExecutingComponent,
    notification_handlers::{
        CommitNotification, CommitNotificationListener, CommittedTransactions,
        ConsensusNotificationHandler, ErrorNotification, ErrorNotificationListener,
        MempoolNotificationHandler,
    },
    storage_synchronizer::StorageSynchronizerInterface,
    utils,
    utils::PENDING_DATA_LOG_FREQ_SECS,
};
use aptos_config::config::{RoleType, StateSyncDriverConfig};
use aptos_data_client::AptosDataClient;
use aptos_infallible::Mutex;
use aptos_logger::prelude::*;
use aptos_types::waypoint::Waypoint;
use consensus_notifications::{
    ConsensusCommitNotification, ConsensusNotification, ConsensusSyncNotification,
};
use data_streaming_service::streaming_client::{
    DataStreamingClient, NotificationAndFeedback, NotificationFeedback,
};
use event_notifications::EventSubscriptionService;
use futures::StreamExt;
use mempool_notifications::MempoolNotificationSender;
use std::{sync::Arc, time::SystemTime};
use storage_interface::DbReader;
use tokio::task::yield_now;
use tokio::time::{interval, Duration};
use tokio_stream::wrappers::IntervalStream;

// Useful constants for the driver
const DRIVER_ERROR_LOG_FREQ_SECS: u64 = 3;

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
pub struct StateSyncDriver<
    DataClient,
    MempoolNotifier,
    MetadataStorage,
    StorageSyncer,
    StreamingClient,
> {
    // The component that manages the initial bootstrapping of the node
    bootstrapper: Bootstrapper<MetadataStorage, StorageSyncer, StreamingClient>,

    // The listener for client notifications
    client_notification_listener: ClientNotificationListener,

    // The listener for commit notifications
    commit_notification_listener: CommitNotificationListener,

    // The handler for notifications from consensus
    consensus_notification_handler: ConsensusNotificationHandler,

    // The component that manages the continuous syncing of the node
    continuous_syncer: ContinuousSyncer<StorageSyncer, StreamingClient>,

    // The client for checking the global data summary of our peers
    aptos_data_client: DataClient,

    // The configuration for the driver
    driver_configuration: DriverConfiguration,

    // The listener for errors from the storage synchronizer
    error_notification_listener: ErrorNotificationListener,

    // The event subscription service to notify listeners of on-chain events
    event_subscription_service: Arc<Mutex<EventSubscriptionService>>,

    // The handler for notifications to mempool
    mempool_notification_handler: MempoolNotificationHandler<MempoolNotifier>,

    // The timestamp at which the driver started executing
    start_time: Option<SystemTime>,

    // The interface to read from storage
    storage: Arc<dyn DbReader>,

    // The storage synchronizer used to update local storage
    storage_synchronizer: StorageSyncer,
}

impl<
        DataClient: AptosDataClient + Send + Clone + 'static,
        MempoolNotifier: MempoolNotificationSender,
        MetadataStorage: MetadataStorageInterface + Clone,
        StorageSyncer: StorageSynchronizerInterface + Clone,
        StreamingClient: DataStreamingClient + Clone,
    >
    StateSyncDriver<DataClient, MempoolNotifier, MetadataStorage, StorageSyncer, StreamingClient>
{
    pub fn new(
        client_notification_listener: ClientNotificationListener,
        commit_notification_listener: CommitNotificationListener,
        consensus_notification_handler: ConsensusNotificationHandler,
        driver_configuration: DriverConfiguration,
        error_notification_listener: ErrorNotificationListener,
        event_subscription_service: Arc<Mutex<EventSubscriptionService>>,
        mempool_notification_handler: MempoolNotificationHandler<MempoolNotifier>,
        metadata_storage: MetadataStorage,
        storage_synchronizer: StorageSyncer,
        aptos_data_client: DataClient,
        streaming_client: StreamingClient,
        storage: Arc<dyn DbReader>,
    ) -> Self {
        let bootstrapper = Bootstrapper::new(
            driver_configuration.clone(),
            metadata_storage,
            streaming_client.clone(),
            storage.clone(),
            storage_synchronizer.clone(),
        );
        let continuous_syncer = ContinuousSyncer::new(
            driver_configuration.clone(),
            streaming_client,
            storage.clone(),
            storage_synchronizer.clone(),
        );

        Self {
            bootstrapper,
            client_notification_listener,
            commit_notification_listener,
            consensus_notification_handler,
            continuous_syncer,
            aptos_data_client,
            driver_configuration,
            error_notification_listener,
            event_subscription_service,
            mempool_notification_handler,
            start_time: None,
            storage,
            storage_synchronizer,
        }
    }

    /// Starts the state sync driver
    pub async fn start_driver(mut self) {
        let mut progress_check_interval = IntervalStream::new(interval(Duration::from_millis(
            self.driver_configuration.config.progress_check_interval_ms,
        )))
        .fuse();

        // Start the driver
        info!(LogSchema::new(LogEntry::Driver).message("Started the state sync v2 driver!"));
        self.start_time = Some(SystemTime::now());
        loop {
            ::futures::select! {
                notification = self.client_notification_listener.select_next_some() => {
                    self.handle_client_notification(notification).await;
                },
                notification = self.commit_notification_listener.select_next_some() => {
                    self.handle_commit_notification(notification).await;
                }
                notification = self.consensus_notification_handler.select_next_some() => {
                    self.handle_consensus_notification(notification).await;
                }
                notification = self.error_notification_listener.select_next_some() => {
                    self.handle_error_notification(notification).await;
                }
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
            error!(LogSchema::new(LogEntry::ConsensusNotification)
                .error(&error)
                .message("Error encountered when handling the consensus notification!"));
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
            error!(LogSchema::new(LogEntry::ConsensusNotification)
                .error(&error)
                .message("Error encountered when handling the consensus notification!"));
        }
    }

    /// Handles a commit notification sent by consensus
    async fn handle_consensus_commit_notification(
        &mut self,
        consensus_commit_notification: ConsensusCommitNotification,
    ) -> Result<(), Error> {
        info!(
            LogSchema::new(LogEntry::ConsensusNotification).message(&format!(
                "Received a consensus commit notification! Total transactions: {:?}, events: {:?}",
                consensus_commit_notification.transactions.len(),
                consensus_commit_notification.reconfiguration_events.len()
            ))
        );
        self.update_consensus_commit_metrics(&consensus_commit_notification);

        // TODO(joshlind): can we get consensus to forward the events?

        // Handle the commit notification
        let committed_transactions = CommittedTransactions {
            events: consensus_commit_notification.reconfiguration_events.clone(),
            transactions: consensus_commit_notification.transactions.clone(),
        };
        utils::handle_committed_transactions(
            committed_transactions,
            self.storage.clone(),
            self.mempool_notification_handler.clone(),
            self.event_subscription_service.clone(),
        )
        .await;

        // Respond to consensus successfully
        self.consensus_notification_handler
            .respond_to_commit_notification(consensus_commit_notification, Ok(()))
            .await?;

        // Check the progress of any sync requests. We need this here because
        // consensus might issue a sync request and then commit (asynchronously).
        self.check_sync_request_progress().await
    }

    /// Updates the storage synchronizer metrics based on the consensus
    /// commit notification.
    fn update_consensus_commit_metrics(
        &self,
        consensus_commit_notification: &ConsensusCommitNotification,
    ) {
        // Update the driver metrics
        metrics::increment_counter(
            &metrics::DRIVER_COUNTERS,
            metrics::DRIVER_CONSENSUS_COMMIT_NOTIFICATION,
        );

        // Update the synced versions
        let operations = [
            metrics::StorageSynchronizerOperations::ExecutedTransactions,
            metrics::StorageSynchronizerOperations::Synced,
        ];
        for operation in operations {
            metrics::increment_gauge(
                &metrics::STORAGE_SYNCHRONIZER_OPERATIONS,
                operation.get_label(),
                consensus_commit_notification.transactions.len() as u64,
            );
        }

        // Update the synced epoch
        if !consensus_commit_notification
            .reconfiguration_events
            .is_empty()
        {
            utils::update_new_epoch_metrics();
        }
    }

    /// Handles a consensus notification to sync to a specified target
    async fn handle_consensus_sync_notification(
        &mut self,
        sync_notification: ConsensusSyncNotification,
    ) -> Result<(), Error> {
        let latest_synced_version = utils::fetch_latest_synced_version(self.storage.clone())?;
        info!(
            LogSchema::new(LogEntry::ConsensusNotification).message(&format!(
            "Received a consensus sync notification! Target version: {:?}. Latest synced version: {:?}",
            sync_notification.target, latest_synced_version,
            ))
        );
        metrics::increment_counter(
            &metrics::DRIVER_COUNTERS,
            metrics::DRIVER_CONSENSUS_SYNC_NOTIFICATION,
        );

        // Initialize a new sync request
        let latest_synced_ledger_info =
            utils::fetch_latest_synced_ledger_info(self.storage.clone())?;
        self.consensus_notification_handler
            .initialize_sync_request(sync_notification, latest_synced_ledger_info)
            .await
    }

    /// Handles a client notification sent by the driver client
    async fn handle_client_notification(&mut self, notification: DriverNotification) {
        debug!(LogSchema::new(LogEntry::ClientNotification)
            .message("Received a notify bootstrap notification from the client!"));
        metrics::increment_counter(
            &metrics::DRIVER_COUNTERS,
            metrics::DRIVER_CLIENT_NOTIFICATION,
        );

        // TODO(joshlind): refactor this if the client only supports one notification type!
        // Extract the bootstrap notifier channel
        let DriverNotification::NotifyOnceBootstrapped(notifier_channel) = notification;

        // Subscribe the bootstrap notifier channel
        if let Err(error) = self
            .bootstrapper
            .subscribe_to_bootstrap_notifications(notifier_channel)
            .await
        {
            error!(LogSchema::new(LogEntry::ClientNotification)
                .error(&error)
                .message("Failed to subscribe to bootstrap notifications!"));
        }
    }

    /// Handles a commit notification sent by the storage synchronizer for a
    /// new state snapshot.
    async fn handle_commit_notification(&mut self, commit_notification: CommitNotification) {
        let CommitNotification::CommittedStateSnapshot(committed_snapshot) = commit_notification;
        info!(
            LogSchema::new(LogEntry::SynchronizerNotification).message(&format!(
                "Received a state snapshot commit notification from the storage synchronizer. \
                        Snapshot version: {:?}. Last committed index: {:?}.",
                committed_snapshot.version, committed_snapshot.last_committed_state_index,
            ))
        );

        // Handle the committed transactions and events
        utils::handle_committed_transactions(
            committed_snapshot.committed_transaction,
            self.storage.clone(),
            self.mempool_notification_handler.clone(),
            self.event_subscription_service.clone(),
        )
        .await;
    }

    /// Handles an error notification sent by the storage synchronizer
    async fn handle_error_notification(&mut self, error_notification: ErrorNotification) {
        error!(LogSchema::new(LogEntry::SynchronizerNotification)
            .error_notification(error_notification.clone())
            .message("Received an error notification from the storage synchronizer!"));

        // Terminate the currently active streams
        let notification_id = error_notification.notification_id;
        let notification_feedback = NotificationFeedback::InvalidPayloadData;
        if self.bootstrapper.is_bootstrapped() {
            if let Err(error) = self
                .continuous_syncer
                .reset_active_stream(Some(NotificationAndFeedback::new(
                    notification_id,
                    notification_feedback,
                )))
                .await
            {
                panic!(
                    "Failed to terminate the active stream for the continuous syncer! Error: {:?}",
                    error
                );
            }
        } else if let Err(error) = self
            .bootstrapper
            .reset_active_stream(Some(NotificationAndFeedback::new(
                notification_id,
                notification_feedback,
            )))
            .await
        {
            panic!(
                "Failed to terminate the active stream for the bootstrapper! Error: {:?}",
                error
            );
        };
    }

    /// Checks if the node has successfully reached the sync target
    async fn check_sync_request_progress(&mut self) -> Result<(), Error> {
        if !self.active_sync_request() {
            return Ok(()); // There's no pending sync request
        }

        // There's a sync request. Fetch it and check if we're still behind the target.
        let sync_request = self.consensus_notification_handler.get_sync_request();
        let sync_target_version = sync_request
            .lock()
            .as_ref()
            .expect("We've already verified there is an active sync request!")
            .get_sync_target_version();
        let latest_synced_ledger_info =
            utils::fetch_latest_synced_ledger_info(self.storage.clone())?;
        if latest_synced_ledger_info.ledger_info().version() < sync_target_version {
            return Ok(());
        }

        // Wait for the storage synchronizer to drain (if it hasn't already).
        // This prevents notifying consensus prematurely.
        while self.storage_synchronizer.pending_storage_data() {
            sample!(
                SampleRate::Duration(Duration::from_secs(PENDING_DATA_LOG_FREQ_SECS)),
                info!("Waiting for the storage synchronizer to handle pending data!")
            );

            // Yield to avoid starving the storage synchronizer threads.
            yield_now().await;
        }

        // Refresh the latest synced ledger info and handle the sync request
        let latest_synced_ledger_info =
            utils::fetch_latest_synced_ledger_info(self.storage.clone())?;
        self.consensus_notification_handler
            .check_sync_request_progress(latest_synced_ledger_info)
            .await?;

        // If the sync request was successfully handled, reset the continuous syncer
        // so that in the event another sync request occurs, we have fresh state.
        if !self.active_sync_request() {
            self.continuous_syncer.reset_active_stream(None).await?;
            self.storage_synchronizer.finish_chunk_executor(); // Consensus is now in control
        }
        Ok(())
    }

    /// Returns true iff there's an active sync request from consensus
    fn active_sync_request(&self) -> bool {
        self.consensus_notification_handler.active_sync_request()
    }

    /// Returns true iff this node is a validator
    fn is_validator(&self) -> bool {
        self.driver_configuration.role == RoleType::Validator
    }

    /// Returns true iff consensus is currently executing
    fn check_if_consensus_executing(&self) -> bool {
        self.is_validator() && self.bootstrapper.is_bootstrapped() && !self.active_sync_request()
    }

    /// Checks if the connection deadline has passed. If so, validators with
    /// genesis waypoints will be automatically marked as bootstrapped. This
    /// helps in the case of single node deployments, where there are no peers
    /// and state sync is trivial.
    async fn check_auto_bootstrapping(&mut self) {
        if !self.bootstrapper.is_bootstrapped()
            && self.is_validator()
            && self.driver_configuration.waypoint.version() == 0
        {
            if let Some(start_time) = self.start_time {
                if let Some(connection_deadline) = start_time.checked_add(Duration::from_secs(
                    self.driver_configuration
                        .config
                        .max_connection_deadline_secs,
                )) {
                    if SystemTime::now()
                        .duration_since(connection_deadline)
                        .is_ok()
                    {
                        info!(LogSchema::new(LogEntry::AutoBootstrapping).message(
                            "Passed the connection deadline! Auto-bootstrapping the validator!"
                        ));
                        if let Err(error) = self.bootstrapper.bootstrapping_complete().await {
                            error!(LogSchema::new(LogEntry::AutoBootstrapping)
                                .error(&error)
                                .message("Failed to mark bootstrapping as complete!"));
                        }
                    }
                } else {
                    error!(LogSchema::new(LogEntry::AutoBootstrapping)
                        .message("The connection deadline overflowed! Unable to auto-bootstrap!"));
                }
            }
        }
    }

    /// Checks that state sync is making progress
    async fn drive_progress(&mut self) {
        // Fetch the global data summary and verify we have active peers
        let global_data_summary = self.aptos_data_client.get_global_data_summary();
        if global_data_summary.is_empty() {
            trace!(LogSchema::new(LogEntry::Driver).message(
                "The global data summary is empty! It's likely that we have no active peers."
            ));
            return self.check_auto_bootstrapping().await;
        }

        // Check the progress of any sync requests
        if let Err(error) = self.check_sync_request_progress().await {
            error!(LogSchema::new(LogEntry::Driver)
                .error(&error)
                .message("Error found when checking the sync request progress!"));
        }

        // If consensus is executing, there's nothing to do
        if self.check_if_consensus_executing() {
            trace!(LogSchema::new(LogEntry::Driver)
                .message("Consensus is executing. There's nothing to do."));
            metrics::increment_counter(
                &metrics::EXECUTING_COMPONENT,
                ExecutingComponent::Consensus.get_label(),
            );
            return;
        }

        // Drive progress depending on if we're bootstrapping or continuously syncing
        if self.bootstrapper.is_bootstrapped() {
            // Fetch any consensus sync requests
            let consensus_sync_request = self.consensus_notification_handler.get_sync_request();

            // Attempt to continuously sync
            metrics::increment_counter(
                &metrics::EXECUTING_COMPONENT,
                ExecutingComponent::ContinuousSyncer.get_label(),
            );
            if let Err(error) = self
                .continuous_syncer
                .drive_progress(consensus_sync_request)
                .await
            {
                sample!(
                    SampleRate::Duration(Duration::from_secs(DRIVER_ERROR_LOG_FREQ_SECS)),
                    error!(LogSchema::new(LogEntry::Driver)
                        .error(&error)
                        .message("Error found when driving progress of the continuous syncer!"));
                );
                metrics::increment_counter(&metrics::CONTINUOUS_SYNCER_ERRORS, error.get_label());
            }
        } else {
            metrics::increment_counter(
                &metrics::EXECUTING_COMPONENT,
                ExecutingComponent::Bootstrapper.get_label(),
            );
            if let Err(error) = self.bootstrapper.drive_progress(&global_data_summary).await {
                sample!(
                        SampleRate::Duration(Duration::from_secs(DRIVER_ERROR_LOG_FREQ_SECS)),
                        error!(LogSchema::new(LogEntry::Driver)
                            .error(&error)
                            .message("Error found when checking the bootstrapper progress!"));
                );
                metrics::increment_counter(&metrics::BOOTSTRAPPER_ERRORS, error.get_label());
            }
        };
    }
}

// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
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
        MempoolNotificationHandler, StorageServiceNotificationHandler,
    },
    storage_synchronizer::StorageSynchronizerInterface,
    utils,
    utils::{OutputFallbackHandler, PENDING_DATA_LOG_FREQ_SECS},
};
use aptos_config::config::{ConsensusObserverConfig, RoleType, StateSyncDriverConfig};
use aptos_consensus_notifications::{
    ConsensusCommitNotification, ConsensusNotification, ConsensusSyncDurationNotification,
    ConsensusSyncTargetNotification,
};
use aptos_data_client::interface::AptosDataClientInterface;
use aptos_data_streaming_service::streaming_client::{
    DataStreamingClient, NotificationAndFeedback, NotificationFeedback,
};
use aptos_event_notifications::EventSubscriptionService;
use aptos_infallible::Mutex;
use aptos_logger::prelude::*;
use aptos_mempool_notifications::MempoolNotificationSender;
use aptos_storage_interface::DbReader;
use aptos_storage_service_notifications::StorageServiceNotificationSender;
use aptos_time_service::{TimeService, TimeServiceTrait};
use aptos_types::{contract_event::ContractEvent, waypoint::Waypoint};
use futures::StreamExt;
use std::{sync::Arc, time::Instant};
use tokio::{
    task::yield_now,
    time::{interval, Duration},
};
use tokio_stream::wrappers::IntervalStream;

// Useful constants for the driver
const DRIVER_INFO_LOG_FREQ_SECS: u64 = 2;
const DRIVER_ERROR_LOG_FREQ_SECS: u64 = 3;

/// The configuration of the state sync driver
#[derive(Clone)]
pub struct DriverConfiguration {
    // The config file of the driver
    pub config: StateSyncDriverConfig,

    // The config for consensus observer
    pub consensus_observer_config: ConsensusObserverConfig,

    // The role of the node
    pub role: RoleType,

    // The trusted waypoint for the node
    pub waypoint: Waypoint,
}

impl DriverConfiguration {
    pub fn new(
        config: StateSyncDriverConfig,
        consensus_observer_config: ConsensusObserverConfig,
        role: RoleType,
        waypoint: Waypoint,
    ) -> Self {
        Self {
            config,
            consensus_observer_config,
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
    StorageServiceNotifier,
    StorageSyncer,
    StreamingClient,
> {
    // The component that manages the initial bootstrapping of the node
    bootstrapper: Bootstrapper<MetadataStorage, StorageSyncer, StreamingClient>,

    // The listener for client notifications
    client_notification_listener: ClientNotificationListener,

    // The listener for commit notifications
    commit_notification_listener: CommitNotificationListener,

    // The handler for notifications from consensus or consensus observer
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
    start_time: Option<Instant>,

    // The interface to read from storage
    storage: Arc<dyn DbReader>,

    // The handler for notifications to the storage service
    storage_service_notification_handler: StorageServiceNotificationHandler<StorageServiceNotifier>,

    // The storage synchronizer used to update local storage
    storage_synchronizer: StorageSyncer,

    // The time service
    time_service: TimeService,
}

impl<
        DataClient: AptosDataClientInterface + Send + Clone + 'static,
        MempoolNotifier: MempoolNotificationSender,
        MetadataStorage: MetadataStorageInterface + Clone,
        StorageServiceNotifier: StorageServiceNotificationSender,
        StorageSyncer: StorageSynchronizerInterface + Clone,
        StreamingClient: DataStreamingClient + Clone,
    >
    StateSyncDriver<
        DataClient,
        MempoolNotifier,
        MetadataStorage,
        StorageServiceNotifier,
        StorageSyncer,
        StreamingClient,
    >
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        client_notification_listener: ClientNotificationListener,
        commit_notification_listener: CommitNotificationListener,
        consensus_notification_handler: ConsensusNotificationHandler,
        driver_configuration: DriverConfiguration,
        error_notification_listener: ErrorNotificationListener,
        event_subscription_service: Arc<Mutex<EventSubscriptionService>>,
        mempool_notification_handler: MempoolNotificationHandler<MempoolNotifier>,
        metadata_storage: MetadataStorage,
        storage_service_notification_handler: StorageServiceNotificationHandler<
            StorageServiceNotifier,
        >,
        storage_synchronizer: StorageSyncer,
        aptos_data_client: DataClient,
        streaming_client: StreamingClient,
        storage: Arc<dyn DbReader>,
        time_service: TimeService,
    ) -> Self {
        let output_fallback_handler =
            OutputFallbackHandler::new(driver_configuration.clone(), time_service.clone());
        let bootstrapper = Bootstrapper::new(
            driver_configuration.clone(),
            metadata_storage,
            output_fallback_handler.clone(),
            streaming_client.clone(),
            storage.clone(),
            storage_synchronizer.clone(),
        );
        let continuous_syncer = ContinuousSyncer::new(
            driver_configuration.clone(),
            streaming_client,
            output_fallback_handler,
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
            storage_service_notification_handler,
            storage_synchronizer,
            time_service,
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
        self.start_time = Some(self.time_service.now());
        loop {
            ::futures::select! {
                notification = self.client_notification_listener.select_next_some() => {
                    self.handle_client_notification(notification).await;
                },
                notification = self.commit_notification_listener.select_next_some() => {
                    self.handle_snapshot_commit_notification(notification).await;
                }
                notification = self.consensus_notification_handler.select_next_some() => {
                    self.handle_consensus_or_observer_notification(notification).await;
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

    /// Handles a notification sent by consensus or consensus observer
    async fn handle_consensus_or_observer_notification(
        &mut self,
        notification: ConsensusNotification,
    ) {
        // Verify the notification before processing it
        let result = if !self.is_consensus_or_observer_enabled() {
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

        // Handle any verification errors
        if let Err(error) = result {
            match notification {
                ConsensusNotification::NotifyCommit(commit_notification) => {
                    let _ = self
                        .consensus_notification_handler
                        .respond_to_commit_notification(commit_notification, Err(error.clone()));
                },
                ConsensusNotification::SyncToTarget(sync_notification) => {
                    let _ = self
                        .consensus_notification_handler
                        .respond_to_sync_target_notification(sync_notification, Err(error.clone()));
                },
                ConsensusNotification::SyncForDuration(sync_notification) => {
                    let _ = self
                        .consensus_notification_handler
                        .respond_to_sync_duration_notification(
                            sync_notification,
                            Err(error.clone()),
                            None,
                        );
                },
            }
            warn!(LogSchema::new(LogEntry::ConsensusNotification)
                .error(&error)
                .message("Error encountered when handling the consensus notification!"));
            return;
        }

        // Handle the notification
        let result = match notification {
            ConsensusNotification::NotifyCommit(commit_notification) => {
                self.handle_consensus_commit_notification(commit_notification)
                    .await
            },
            ConsensusNotification::SyncToTarget(sync_notification) => {
                self.handle_consensus_sync_target_notification(sync_notification)
                    .await
            },
            ConsensusNotification::SyncForDuration(sync_notification) => {
                self.handle_consensus_sync_duration_notification(sync_notification)
                    .await
            },
        };

        // Log any errors from notification handling
        if let Err(error) = result {
            warn!(LogSchema::new(LogEntry::ConsensusNotification)
                .error(&error)
                .message("Error encountered when handling the consensus notification!"));
        }
    }

    /// Handles a commit notification sent by consensus or consensus observer
    async fn handle_consensus_commit_notification(
        &mut self,
        commit_notification: ConsensusCommitNotification,
    ) -> Result<(), Error> {
        info!(
            LogSchema::new(LogEntry::ConsensusNotification).message(&format!(
                "Received a consensus commit notification! Total transactions: {:?}, events: {:?}",
                commit_notification.get_transactions().len(),
                commit_notification.get_subscribable_events().len()
            ))
        );
        self.update_consensus_commit_metrics(&commit_notification);

        // Handle the commit notification
        let committed_transactions = CommittedTransactions {
            events: commit_notification.get_subscribable_events().clone(),
            transactions: commit_notification.get_transactions().clone(),
        };
        utils::handle_committed_transactions(
            committed_transactions,
            self.storage.clone(),
            self.mempool_notification_handler.clone(),
            self.event_subscription_service.clone(),
            self.storage_service_notification_handler.clone(),
        )
        .await;

        // Respond successfully
        self.consensus_notification_handler
            .respond_to_commit_notification(commit_notification, Ok(()))?;

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

        // Update the number of executed transactions
        let num_synced_transactions = consensus_commit_notification.get_transactions().len();
        metrics::increment_gauge(
            &metrics::STORAGE_SYNCHRONIZER_OPERATIONS,
            metrics::StorageSynchronizerOperations::ExecutedTransactions.get_label(),
            num_synced_transactions as u64,
        );

        // Update the synced version metrics
        utils::update_new_synced_metrics(self.storage.clone(), num_synced_transactions);

        // Update the synced epoch metrics
        let reconfiguration_occurred = consensus_commit_notification
            .get_subscribable_events()
            .iter()
            .any(ContractEvent::is_new_epoch_event);
        utils::update_new_epoch_metrics(self.storage.clone(), reconfiguration_occurred);
    }

    /// Handles a consensus or consensus observer request to sync for a specified duration
    async fn handle_consensus_sync_duration_notification(
        &mut self,
        sync_duration_notification: ConsensusSyncDurationNotification,
    ) -> Result<(), Error> {
        // Update the sync duration notification metrics
        let latest_synced_version = utils::fetch_pre_committed_version(self.storage.clone())?;
        info!(
            LogSchema::new(LogEntry::ConsensusNotification).message(&format!(
                "Received a consensus sync duration notification! Duration: {:?}. Latest synced version: {:?}",
                sync_duration_notification.get_duration(), latest_synced_version,
            ))
        );
        metrics::increment_counter(
            &metrics::DRIVER_COUNTERS,
            metrics::DRIVER_CONSENSUS_SYNC_DURATION_NOTIFICATION,
        );

        // Initialize a new sync request
        self.consensus_notification_handler
            .initialize_sync_duration_request(sync_duration_notification)
            .await
    }

    /// Handles a consensus or consensus observer request to sync to a specified target
    async fn handle_consensus_sync_target_notification(
        &mut self,
        sync_target_notification: ConsensusSyncTargetNotification,
    ) -> Result<(), Error> {
        // Fetch the pre-committed and committed versions
        let latest_pre_committed_version =
            utils::fetch_pre_committed_version(self.storage.clone())?;
        let latest_synced_ledger_info =
            utils::fetch_latest_synced_ledger_info(self.storage.clone())?;
        let latest_committed_version = latest_synced_ledger_info.ledger_info().version();

        // Update the sync target notification logs and metrics
        info!(
            LogSchema::new(LogEntry::ConsensusNotification).message(&format!(
                "Received a consensus sync target notification! Target: {:?}. \
                Latest pre-committed version: {}. Latest committed version: {}.",
                sync_target_notification.get_target(),
                latest_pre_committed_version,
                latest_committed_version,
            ))
        );
        metrics::increment_counter(
            &metrics::DRIVER_COUNTERS,
            metrics::DRIVER_CONSENSUS_SYNC_TARGET_NOTIFICATION,
        );

        // Initialize a new sync request
        self.consensus_notification_handler
            .initialize_sync_target_request(
                sync_target_notification,
                latest_pre_committed_version,
                latest_synced_ledger_info,
            )
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
            warn!(LogSchema::new(LogEntry::ClientNotification)
                .error(&error)
                .message("Failed to subscribe to bootstrap notifications!"));
        }
    }

    /// Handles a notification from the storage synchronizer for a new state snapshot
    async fn handle_snapshot_commit_notification(
        &mut self,
        commit_notification: CommitNotification,
    ) {
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
            self.storage_service_notification_handler.clone(),
        )
        .await;
    }

    /// Handles an error notification sent by the storage synchronizer
    async fn handle_error_notification(&mut self, error_notification: ErrorNotification) {
        warn!(LogSchema::new(LogEntry::SynchronizerNotification)
            .error_notification(error_notification.clone())
            .message("Received an error notification from the storage synchronizer!"));

        // Terminate the currently active streams
        let notification_id = error_notification.notification_id;
        let notification_feedback = NotificationFeedback::InvalidPayloadData;
        if self.bootstrapper.is_bootstrapped() {
            if let Err(error) = self
                .continuous_syncer
                .handle_storage_synchronizer_error(NotificationAndFeedback::new(
                    notification_id,
                    notification_feedback,
                ))
                .await
            {
                error!(LogSchema::new(LogEntry::SynchronizerNotification)
                    .message(&format!(
                        "Failed to terminate the active stream for the continuous syncer! Error: {:?}",
                        error
                    )));
            }
        } else if let Err(error) = self
            .bootstrapper
            .handle_storage_synchronizer_error(NotificationAndFeedback::new(
                notification_id,
                notification_feedback,
            ))
            .await
        {
            error!(
                LogSchema::new(LogEntry::SynchronizerNotification).message(&format!(
                    "Failed to terminate the active stream for the bootstrapper! Error: {:?}",
                    error
                ))
            );
        };
    }

    /// Checks if the node has successfully reached the sync target or duration
    async fn check_sync_request_progress(&mut self) -> Result<(), Error> {
        // Check if the sync request has been satisfied
        let consensus_sync_request = self.consensus_notification_handler.get_sync_request();
        match consensus_sync_request.lock().as_ref() {
            Some(consensus_sync_request) => {
                let latest_synced_ledger_info =
                    utils::fetch_latest_synced_ledger_info(self.storage.clone())?;
                if !consensus_sync_request
                    .sync_request_satisfied(&latest_synced_ledger_info, self.time_service.clone())
                {
                    return Ok(()); // The sync request hasn't been satisfied yet
                }
            },
            None => {
                return Ok(()); // There's no active sync request
            },
        }

        // The sync request has been satisfied. Wait for the storage synchronizer
        // to drain. This prevents notifying consensus prematurely.
        while self.storage_synchronizer.pending_storage_data() {
            sample!(
                SampleRate::Duration(Duration::from_secs(PENDING_DATA_LOG_FREQ_SECS)),
                info!("Waiting for the storage synchronizer to handle pending data!")
            );

            // Yield to avoid starving the storage synchronizer threads.
            yield_now().await;
        }

        // If the request was to sync for a specified duration, we should only
        // stop syncing when the synced version and synced ledger info version match.
        // Otherwise, the DB will be left in an inconsistent state on handover.
        if let Some(sync_request) = consensus_sync_request.lock().as_ref() {
            if sync_request.is_sync_duration_request() {
                // Get the latest synced version and ledger info version
                let latest_synced_version =
                    utils::fetch_pre_committed_version(self.storage.clone())?;
                let latest_synced_ledger_info =
                    utils::fetch_latest_synced_ledger_info(self.storage.clone())?;
                let latest_ledger_info_version = latest_synced_ledger_info.ledger_info().version();

                // Check if the latest synced version matches the latest ledger info version
                if latest_synced_version != latest_ledger_info_version {
                    sample!(
                        SampleRate::Duration(Duration::from_secs(DRIVER_INFO_LOG_FREQ_SECS)),
                        info!(
                            "Waiting for state sync to sync to a ledger info! \
                            Latest synced version: {:?}, latest ledger info version: {:?}",
                            latest_synced_version, latest_ledger_info_version
                        )
                    );

                    return Ok(()); // State sync should continue to run
                }
            }
        }

        // Handle the satisfied sync request
        let latest_synced_ledger_info =
            utils::fetch_latest_synced_ledger_info(self.storage.clone())?;
        self.consensus_notification_handler
            .handle_satisfied_sync_request(latest_synced_ledger_info)
            .await?;

        // If the sync request was successfully handled, reset the continuous syncer
        // so that in the event another sync request occurs, we have fresh state.
        if !self.active_sync_request() {
            self.continuous_syncer.reset_active_stream(None).await?;
            self.storage_synchronizer.finish_chunk_executor(); // Consensus or consensus observer is now in control
        }

        Ok(())
    }

    /// Returns true iff there's an active sync request from consensus or consensus observer
    fn active_sync_request(&self) -> bool {
        self.consensus_notification_handler.active_sync_request()
    }

    /// Returns true iff this node enables consensus or consensus observer
    fn is_consensus_or_observer_enabled(&self) -> bool {
        self.driver_configuration.role == RoleType::Validator
            || self
                .driver_configuration
                .consensus_observer_config
                .observer_enabled
    }

    /// Returns true iff consensus or consensus observer is currently executing
    fn check_if_consensus_or_observer_executing(&self) -> bool {
        self.is_consensus_or_observer_enabled()
            && self.bootstrapper.is_bootstrapped()
            && !self.active_sync_request()
    }

    /// Checks if the connection deadline has passed. If so, validators with
    /// genesis waypoints will be automatically marked as bootstrapped. This
    /// helps in the case of single node deployments, where there are no peers
    /// and state sync is trivial.
    async fn check_auto_bootstrapping(&mut self) {
        if !self.bootstrapper.is_bootstrapped()
            && self.is_consensus_or_observer_enabled()
            && self.driver_configuration.config.enable_auto_bootstrapping
            && self.driver_configuration.waypoint.version() == 0
        {
            if let Some(start_time) = self.start_time {
                if let Some(connection_deadline) = start_time.checked_add(Duration::from_secs(
                    self.driver_configuration
                        .config
                        .max_connection_deadline_secs,
                )) {
                    if self.time_service.now() >= connection_deadline {
                        info!(LogSchema::new(LogEntry::AutoBootstrapping).message(
                            "Passed the connection deadline! Auto-bootstrapping the validator!"
                        ));
                        if let Err(error) = self.bootstrapper.bootstrapping_complete().await {
                            warn!(LogSchema::new(LogEntry::AutoBootstrapping)
                                .error(&error)
                                .message("Failed to mark bootstrapping as complete!"));
                        }
                    }
                } else {
                    warn!(LogSchema::new(LogEntry::AutoBootstrapping)
                        .message("The connection deadline overflowed! Unable to auto-bootstrap!"));
                }
            }
        }
    }

    /// Checks that state sync is making progress
    async fn drive_progress(&mut self) {
        // Update the executing component metrics
        self.update_executing_component_metrics();

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
            warn!(LogSchema::new(LogEntry::Driver)
                .error(&error)
                .message("Error found when checking the sync request progress!"));
        }

        // If consensus or consensus observer is executing, there's nothing to do
        if self.check_if_consensus_or_observer_executing() {
            return;
        }

        // Drive progress depending on if we're bootstrapping or continuously syncing
        if self.bootstrapper.is_bootstrapped() {
            // Fetch any consensus sync requests
            let consensus_sync_request = self.consensus_notification_handler.get_sync_request();

            // Attempt to continuously sync
            if let Err(error) = self
                .continuous_syncer
                .drive_progress(consensus_sync_request)
                .await
            {
                sample!(
                    SampleRate::Duration(Duration::from_secs(DRIVER_ERROR_LOG_FREQ_SECS)),
                    warn!(LogSchema::new(LogEntry::Driver)
                        .error(&error)
                        .message("Error found when driving progress of the continuous syncer!"));
                );
                metrics::increment_counter(&metrics::CONTINUOUS_SYNCER_ERRORS, error.get_label());
            }
        } else if let Err(error) = self.bootstrapper.drive_progress(&global_data_summary).await {
            sample!(
                    SampleRate::Duration(Duration::from_secs(DRIVER_ERROR_LOG_FREQ_SECS)),
                    warn!(LogSchema::new(LogEntry::Driver)
                        .error(&error)
                        .message("Error found when checking the bootstrapper progress!"));
            );
            metrics::increment_counter(&metrics::BOOTSTRAPPER_ERRORS, error.get_label());
        };
    }

    /// Updates the executing component metrics for the driver
    fn update_executing_component_metrics(&self) {
        // Determine the executing component
        let executing_component = if self.check_if_consensus_or_observer_executing() {
            if self.driver_configuration.role.is_validator() {
                ExecutingComponent::Consensus
            } else {
                ExecutingComponent::ConsensusObserver
            }
        } else if self.bootstrapper.is_bootstrapped() {
            ExecutingComponent::ContinuousSyncer
        } else {
            ExecutingComponent::Bootstrapper
        };

        // Increment the executing component counter
        metrics::increment_counter(
            &metrics::EXECUTING_COMPONENT,
            executing_component.get_label(),
        );

        // Set the consensus executing gauge
        if executing_component == ExecutingComponent::Consensus {
            metrics::CONSENSUS_EXECUTING_GAUGE.set(1);
        } else {
            metrics::CONSENSUS_EXECUTING_GAUGE.set(0);
        }
    }
}

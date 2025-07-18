// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    driver::DriverConfiguration,
    error::Error,
    logging::{LogEntry, LogSchema},
    metadata_storage::MetadataStorageInterface,
    metrics,
    metrics::ExecutingComponent,
    storage_synchronizer::{NotificationMetadata, StorageSynchronizerInterface},
    utils,
    utils::{OutputFallbackHandler, SpeculativeStreamState, PENDING_DATA_LOG_FREQ_SECS},
};
use aptos_config::config::BootstrappingMode;
use aptos_data_client::global_summary::GlobalDataSummary;
use aptos_data_streaming_service::{
    data_notification::{DataNotification, DataPayload, NotificationId},
    data_stream::DataStreamListener,
    streaming_client::{DataStreamingClient, NotificationAndFeedback, NotificationFeedback},
};
use aptos_logger::{prelude::*, sample::SampleRate};
use aptos_storage_interface::DbReader;
use aptos_types::{
    epoch_change::Verifier,
    epoch_state::EpochState,
    ledger_info::LedgerInfoWithSignatures,
    state_store::state_value::StateValueChunkWithProof,
    transaction::{TransactionListWithProof, TransactionOutputListWithProof, Version},
    waypoint::Waypoint,
};
use futures::channel::oneshot;
use std::{collections::BTreeMap, sync::Arc, time::Duration};

// Useful bootstrapper constants
const BOOTSTRAPPER_LOG_INTERVAL_SECS: u64 = 3;
pub const GENESIS_TRANSACTION_VERSION: u64 = 0; // The expected version of the genesis transaction

/// A simple container for verified epoch states and epoch ending ledger infos
/// that have been fetched from the network.
#[derive(Clone)]
pub(crate) struct VerifiedEpochStates {
    // If new epoch ending ledger infos have been fetched from the network
    fetched_epoch_ending_ledger_infos: bool,

    // The highest epoch ending version fetched thus far
    highest_fetched_epoch_ending_version: Version,

    // The latest epoch state that has been verified by the node
    latest_epoch_state: EpochState,

    // A map from versions to epoch ending ledger infos fetched from the network
    new_epoch_ending_ledger_infos: BTreeMap<Version, LedgerInfoWithSignatures>,

    // If the node has successfully verified the waypoint
    verified_waypoint: bool,
}

impl VerifiedEpochStates {
    pub fn new(latest_epoch_state: EpochState) -> Self {
        Self {
            fetched_epoch_ending_ledger_infos: false,
            highest_fetched_epoch_ending_version: 0,
            latest_epoch_state,
            new_epoch_ending_ledger_infos: BTreeMap::new(),
            verified_waypoint: false,
        }
    }

    /// Returns true iff the node has already fetched any new epoch
    /// ending ledger infos from the network.
    pub fn fetched_epoch_ending_ledger_infos(&self) -> bool {
        self.fetched_epoch_ending_ledger_infos
    }

    /// Sets `fetched_epoch_ending_ledger_infos` to true
    pub fn set_fetched_epoch_ending_ledger_infos(&mut self) {
        self.fetched_epoch_ending_ledger_infos = true;
    }

    /// Returns true iff the node has verified the waypoint
    pub fn verified_waypoint(&self) -> bool {
        self.verified_waypoint
    }

    /// Sets `verified_waypoint` to true
    pub fn set_verified_waypoint(&mut self, waypoint_version: Version) {
        info!(LogSchema::new(LogEntry::Bootstrapper).message(&format!(
            "The waypoint has been verified! Waypoint version: {:?}.",
            waypoint_version
        )));

        self.verified_waypoint = true;
    }

    /// Verifies the given epoch ending ledger info, updates our latest
    /// trusted epoch state and attempts to verify any given waypoint.
    pub fn update_verified_epoch_states(
        &mut self,
        epoch_ending_ledger_info: &LedgerInfoWithSignatures,
        waypoint: &Waypoint,
    ) -> Result<(), Error> {
        // Verify the ledger info against the latest epoch state
        self.latest_epoch_state
            .verify(epoch_ending_ledger_info)
            .map_err(|error| {
                Error::VerificationError(format!("Ledger info failed verification: {:?}", error))
            })?;

        // Update the latest epoch state with the next epoch
        if let Some(next_epoch_state) = epoch_ending_ledger_info.ledger_info().next_epoch_state() {
            self.highest_fetched_epoch_ending_version =
                epoch_ending_ledger_info.ledger_info().version();
            self.latest_epoch_state = next_epoch_state.clone();
            self.insert_new_epoch_ending_ledger_info(epoch_ending_ledger_info.clone())?;

            trace!(LogSchema::new(LogEntry::Bootstrapper).message(&format!(
                "Updated the latest epoch state to epoch: {:?}",
                self.latest_epoch_state.epoch
            )));
        } else {
            return Err(Error::VerificationError(
                "The ledger info was not epoch ending!".into(),
            ));
        }

        // Check if the ledger info corresponds to the trusted waypoint
        self.verify_waypoint(epoch_ending_ledger_info, waypoint)
    }

    /// Attempts to verify the waypoint using the new epoch ending ledger info
    fn verify_waypoint(
        &mut self,
        epoch_ending_ledger_info: &LedgerInfoWithSignatures,
        waypoint: &Waypoint,
    ) -> Result<(), Error> {
        if !self.verified_waypoint {
            // Fetch the waypoint and ledger info versions
            let waypoint_version = waypoint.version();
            let ledger_info = epoch_ending_ledger_info.ledger_info();
            let ledger_info_version = ledger_info.version();

            // Verify we haven't missed the waypoint
            if ledger_info_version > waypoint_version {
                panic!(
                    "Failed to verify the waypoint: ledger info version is too high! Waypoint version: {:?}, ledger info version: {:?}",
                    waypoint_version, ledger_info_version
                );
            }

            // Check if we've found the ledger info corresponding to the waypoint version
            if ledger_info_version == waypoint_version {
                match waypoint.verify(ledger_info) {
                    Ok(()) => self.set_verified_waypoint(waypoint_version),
                    Err(error) => {
                        panic!(
                            "Failed to verify the waypoint: {:?}! Waypoint: {:?}, given ledger info: {:?}",
                            error, waypoint, ledger_info
                        );
                    },
                }
            }
        }

        Ok(())
    }

    /// Adds an epoch ending ledger info to the new epoch ending ledger infos map
    fn insert_new_epoch_ending_ledger_info(
        &mut self,
        epoch_ending_ledger_info: LedgerInfoWithSignatures,
    ) -> Result<(), Error> {
        let ledger_info = epoch_ending_ledger_info.ledger_info();
        info!(LogSchema::new(LogEntry::Bootstrapper).message(&format!(
            "Adding a new epoch to the epoch ending ledger infos. Epoch: {:?}, Version: {:?}, Ends epoch: {:?}, Waypoint: {:?}",
            ledger_info.epoch(), ledger_info.version(), ledger_info.ends_epoch(), Waypoint::new_epoch_boundary(ledger_info),
        )));

        // Insert the version to ledger info mapping
        let version = ledger_info.version();
        if let Some(epoch_ending_ledger_info) = self
            .new_epoch_ending_ledger_infos
            .insert(version, epoch_ending_ledger_info)
        {
            Err(Error::UnexpectedError(format!(
                "Duplicate epoch ending ledger info found!\
                 Version: {:?}, \
                 ledger info: {:?}",
                version, epoch_ending_ledger_info,
            )))
        } else {
            Ok(())
        }
    }

    /// Return all epoch ending ledger infos
    pub fn all_epoch_ending_ledger_infos(&self) -> Vec<LedgerInfoWithSignatures> {
        self.new_epoch_ending_ledger_infos
            .values()
            .cloned()
            .collect()
    }

    /// Returns any epoch ending ledger info associated with the given version
    pub fn get_epoch_ending_ledger_info(
        &self,
        version: Version,
    ) -> Option<LedgerInfoWithSignatures> {
        self.new_epoch_ending_ledger_infos.get(&version).cloned()
    }

    /// Returns the highest known ledger info we've fetched (if any)
    pub fn get_highest_known_ledger_info(&self) -> Result<Option<LedgerInfoWithSignatures>, Error> {
        let highest_known_ledger_info = if !self.new_epoch_ending_ledger_infos.is_empty() {
            let highest_fetched_ledger_info = self
                .get_epoch_ending_ledger_info(self.highest_fetched_epoch_ending_version)
                .ok_or_else(|| {
                    Error::UnexpectedError(format!(
                        "The highest known ledger info for version: {:?} was not found!",
                        self.highest_fetched_epoch_ending_version
                    ))
                })?;
            Some(highest_fetched_ledger_info)
        } else {
            None
        };
        Ok(highest_known_ledger_info)
    }

    /// Returns the next epoch ending version after the given version (if one
    /// exists).
    pub fn next_epoch_ending_version(&self, version: Version) -> Option<Version> {
        // BTreeMap keys are iterated through in increasing key orders (i.e., versions)
        for (epoch_ending_version, _) in self.new_epoch_ending_ledger_infos.iter() {
            if *epoch_ending_version > version {
                return Some(*epoch_ending_version);
            }
        }
        None
    }
}

/// A simple container to manage data related to state value snapshot syncing
pub(crate) struct StateValueSyncer {
    // Whether or not a state snapshot receiver has been initialized
    initialized_state_snapshot_receiver: bool,

    // The epoch ending ledger info for the version we're syncing
    ledger_info_to_sync: Option<LedgerInfoWithSignatures>,

    // The next state value index to process (all state values before this have been
    // processed -- i.e., sent to the storage synchronizer).
    next_state_index_to_process: u64,

    // The transaction output (inc. info and proof) for the version we're syncing
    transaction_output_to_sync: Option<TransactionOutputListWithProof>,
}

impl StateValueSyncer {
    pub fn new() -> Self {
        Self {
            initialized_state_snapshot_receiver: false,
            ledger_info_to_sync: None,
            next_state_index_to_process: 0,
            transaction_output_to_sync: None,
        }
    }

    /// Sets the ledger info to sync
    pub fn set_ledger_info_to_sync(&mut self, ledger_info_to_sync: LedgerInfoWithSignatures) {
        self.ledger_info_to_sync = Some(ledger_info_to_sync);
    }

    /// Sets the transaction output to sync
    pub fn set_transaction_output_to_sync(
        &mut self,
        transaction_output_to_sync: TransactionOutputListWithProof,
    ) {
        self.transaction_output_to_sync = Some(transaction_output_to_sync);
    }

    /// Updates the next state index to process
    pub fn update_next_state_index_to_process(&mut self, next_state_index_to_process: u64) {
        self.next_state_index_to_process = next_state_index_to_process;
    }
}

/// A simple component that manages the bootstrapping of the node
pub struct Bootstrapper<MetadataStorage, StorageSyncer, StreamingClient> {
    // The currently active data stream (provided by the data streaming service)
    active_data_stream: Option<DataStreamListener>,

    // The channel used to notify a listener of successful bootstrapping
    bootstrap_notifier_channel: Option<oneshot::Sender<Result<(), Error>>>,

    // If the node has completed bootstrapping
    bootstrapped: bool,

    // The config of the state sync driver
    driver_configuration: DriverConfiguration,

    // The storage to write metadata about the syncing progress
    metadata_storage: MetadataStorage,

    // The handler for output fallback behaviour
    output_fallback_handler: OutputFallbackHandler,

    // The speculative state tracking the active data stream
    speculative_stream_state: Option<SpeculativeStreamState>,

    // The component used to sync state values (if downloading states)
    state_value_syncer: StateValueSyncer,

    // The client through which to stream data from the Aptos network
    streaming_client: StreamingClient,

    // The interface to read from storage
    storage: Arc<dyn DbReader>,

    // The storage synchronizer used to update local storage
    storage_synchronizer: StorageSyncer,

    // The epoch states verified by this node (held in memory)
    verified_epoch_states: VerifiedEpochStates,
}

impl<
        MetadataStorage: MetadataStorageInterface + Clone,
        StorageSyncer: StorageSynchronizerInterface + Clone,
        StreamingClient: DataStreamingClient + Clone,
    > Bootstrapper<MetadataStorage, StorageSyncer, StreamingClient>
{
    pub fn new(
        driver_configuration: DriverConfiguration,
        metadata_storage: MetadataStorage,
        output_fallback_handler: OutputFallbackHandler,
        streaming_client: StreamingClient,
        storage: Arc<dyn DbReader>,
        storage_synchronizer: StorageSyncer,
    ) -> Self {
        // Load the latest epoch state from storage
        let latest_epoch_state = utils::fetch_latest_epoch_state(storage.clone())
            .expect("Unable to fetch latest epoch state!");
        let verified_epoch_states = VerifiedEpochStates::new(latest_epoch_state);

        Self {
            state_value_syncer: StateValueSyncer::new(),
            active_data_stream: None,
            bootstrap_notifier_channel: None,
            bootstrapped: false,
            driver_configuration,
            metadata_storage,
            output_fallback_handler,
            speculative_stream_state: None,
            streaming_client,
            storage,
            storage_synchronizer,
            verified_epoch_states,
        }
    }

    /// Returns the bootstrapping mode of the node
    fn get_bootstrapping_mode(&self) -> BootstrappingMode {
        self.driver_configuration.config.bootstrapping_mode
    }

    /// Returns true iff the node has already completed bootstrapping
    pub fn is_bootstrapped(&self) -> bool {
        self.bootstrapped
    }

    /// Marks bootstrapping as complete and notifies any listeners
    pub async fn bootstrapping_complete(&mut self) -> Result<(), Error> {
        info!(LogSchema::new(LogEntry::Bootstrapper)
            .message("The node has successfully bootstrapped!"));
        self.bootstrapped = true;
        self.notify_listeners_if_bootstrapped().await
    }

    /// Subscribes the specified channel to bootstrap completion notifications
    pub async fn subscribe_to_bootstrap_notifications(
        &mut self,
        bootstrap_notifier_channel: oneshot::Sender<Result<(), Error>>,
    ) -> Result<(), Error> {
        if self.bootstrap_notifier_channel.is_some() {
            return Err(Error::UnexpectedError(
                "Only one boostrap subscriber is supported at a time!".into(),
            ));
        }

        self.bootstrap_notifier_channel = Some(bootstrap_notifier_channel);
        self.notify_listeners_if_bootstrapped().await
    }

    /// Notifies any listeners if we've now bootstrapped
    async fn notify_listeners_if_bootstrapped(&mut self) -> Result<(), Error> {
        if self.is_bootstrapped() {
            if let Some(notifier_channel) = self.bootstrap_notifier_channel.take() {
                if let Err(error) = notifier_channel.send(Ok(())) {
                    return Err(Error::CallbackSendFailed(format!(
                        "Bootstrap notification error: {:?}",
                        error
                    )));
                }
            }
            self.reset_active_stream(None).await?;
            self.storage_synchronizer.finish_chunk_executor(); // The bootstrapper is now complete
        }

        Ok(())
    }

    /// Checks if the bootstrapper is able to make progress
    pub async fn drive_progress(
        &mut self,
        global_data_summary: &GlobalDataSummary,
    ) -> Result<(), Error> {
        if self.is_bootstrapped() {
            return Err(Error::AlreadyBootstrapped(
                "The bootstrapper should not attempt to make progress!".into(),
            ));
        }

        if self.active_data_stream.is_some() {
            // We have an active data stream. Process any notifications!
            self.process_active_stream_notifications().await?;
        } else if self.storage_synchronizer.pending_storage_data() {
            // Wait for any pending data to be processed
            sample!(
                SampleRate::Duration(Duration::from_secs(PENDING_DATA_LOG_FREQ_SECS)),
                info!("Waiting for the storage synchronizer to handle pending data!")
            );
        } else {
            // Fetch a new data stream to start streaming data
            self.initialize_active_data_stream(global_data_summary)
                .await?;
        }

        // Check if we've now bootstrapped
        self.notify_listeners_if_bootstrapped().await
    }

    /// Returns true iff the bootstrapper should continue to fetch epoch ending
    /// ledger infos (in order to make progress).
    fn should_fetch_epoch_ending_ledger_infos(&self) -> bool {
        !self
            .verified_epoch_states
            .fetched_epoch_ending_ledger_infos()
            || !self.verified_epoch_states.verified_waypoint()
    }

    /// Initializes an active data stream so that we can begin to process notifications
    async fn initialize_active_data_stream(
        &mut self,
        global_data_summary: &GlobalDataSummary,
    ) -> Result<(), Error> {
        // Reset the chunk executor to flush any invalid state currently held in-memory
        self.storage_synchronizer.reset_chunk_executor()?;

        // Always fetch the new epoch ending ledger infos first
        if self.should_fetch_epoch_ending_ledger_infos() {
            return self
                .fetch_epoch_ending_ledger_infos(global_data_summary)
                .await;
        }

        // Get the highest synced and known ledger info versions
        let highest_synced_version = utils::fetch_pre_committed_version(self.storage.clone())?;
        let highest_known_ledger_info = self.get_highest_known_ledger_info()?;
        let highest_known_ledger_version = highest_known_ledger_info.ledger_info().version();

        // Check if we need to sync more data
        if self.get_bootstrapping_mode().is_fast_sync()
            && highest_synced_version == GENESIS_TRANSACTION_VERSION
            && highest_known_ledger_version == GENESIS_TRANSACTION_VERSION
        {
            // The node is fast syncing and an epoch change isn't
            // advertised. We need to fast sync to genesis.
            info!(LogSchema::new(LogEntry::Bootstrapper).message(&format!(
                "Fast syncing to genesis! Highest synced and advertised version is {}.",
                highest_synced_version
            )));
        } else if highest_synced_version >= highest_known_ledger_version {
            // Otherwise, if we've already synced to the highest known version, there's nothing to do
            info!(LogSchema::new(LogEntry::Bootstrapper)
                .message(&format!("Highest synced version {} is >= highest known ledger version {}, nothing needs to be done.",
                    highest_synced_version, highest_known_ledger_version)));
            return self.bootstrapping_complete().await;
        }

        sample!(
            SampleRate::Duration(Duration::from_secs(BOOTSTRAPPER_LOG_INTERVAL_SECS)),
            info!(LogSchema::new(LogEntry::Bootstrapper).message(&format!(
                "Highest synced version is {}, highest_known_ledger_info is {:?}, bootstrapping_mode is {:?}.",
                highest_synced_version, highest_known_ledger_info, self.get_bootstrapping_mode()))
            );
        );

        // Bootstrap according to the mode
        if self.get_bootstrapping_mode().is_fast_sync() {
            // We're fast syncing
            self.fetch_missing_state_snapshot_data(
                highest_synced_version,
                highest_known_ledger_info,
            )
            .await
        } else {
            // We're transaction and/or output syncing
            self.fetch_missing_transaction_data(highest_synced_version, highest_known_ledger_info)
                .await
        }
    }

    /// Fetches all missing state snapshot data in order to bootstrap the node
    async fn fetch_missing_state_snapshot_data(
        &mut self,
        highest_synced_version: Version,
        highest_known_ledger_info: LedgerInfoWithSignatures,
    ) -> Result<(), Error> {
        if highest_synced_version == GENESIS_TRANSACTION_VERSION {
            // We're syncing a new node. Check the progress and fetch any missing data
            if let Some(target) = self.metadata_storage.previous_snapshot_sync_target()? {
                if self.metadata_storage.is_snapshot_sync_complete(&target)? {
                    // Fast syncing to the target is complete. Verify that the
                    // highest synced version matches the target.
                    if target.ledger_info().version() == GENESIS_TRANSACTION_VERSION {
                        info!(LogSchema::new(LogEntry::Bootstrapper).message(&format!(
                            "The fast sync to genesis is complete! Target: {:?}",
                            target
                        )));
                        self.bootstrapping_complete().await
                    } else {
                        Err(Error::UnexpectedError(format!(
                            "The snapshot sync for the target was marked as complete but \
                        the highest synced version is genesis! Something has gone wrong! \
                        Target snapshot sync: {:?}",
                            target
                        )))
                    }
                } else {
                    // Continue snapshot syncing to the target
                    self.fetch_missing_state_values(target, true).await
                }
            } else {
                // No snapshot sync has started. Start a new sync for the highest known ledger info.
                self.fetch_missing_state_values(highest_known_ledger_info, false)
                    .await
            }
        } else {
            // This node has already synced some state. Ensure the node is not too far behind.
            let highest_known_ledger_version = highest_known_ledger_info.ledger_info().version();
            let num_versions_behind = highest_known_ledger_version
                .checked_sub(highest_synced_version)
                .ok_or_else(|| {
                    Error::IntegerOverflow("The number of versions behind has overflown!".into())
                })?;
            let max_num_versions_behind = self
                .driver_configuration
                .config
                .num_versions_to_skip_snapshot_sync;

            // Check if the node is too far behind to fast sync
            if num_versions_behind < max_num_versions_behind {
                info!(LogSchema::new(LogEntry::Bootstrapper).message(&format!(
                    "The node is only {} versions behind, will skip bootstrapping.",
                    num_versions_behind
                )));
                // We've already bootstrapped to an initial state snapshot. If this a fullnode, the
                // continuous syncer will take control and get the node up-to-date. If this is a
                // validator, consensus will take control and sync depending on how it sees fit.
                self.bootstrapping_complete().await
            } else {
                panic!("You are currently {:?} versions behind the latest snapshot version ({:?}). This is \
                        more than the maximum allowed for fast sync ({:?}). If you want to fast sync to the \
                        latest state, delete your storage and restart your node. Otherwise, if you want to \
                        sync all the missing data, use intelligent syncing mode!",
                       num_versions_behind, highest_known_ledger_version, max_num_versions_behind);
            }
        }
    }

    /// Attempts to fetch a data notification from the active stream
    async fn fetch_next_data_notification(&mut self) -> Result<DataNotification, Error> {
        let max_stream_wait_time_ms = self.driver_configuration.config.max_stream_wait_time_ms;
        let max_num_stream_timeouts = self.driver_configuration.config.max_num_stream_timeouts;
        let result = utils::get_data_notification(
            max_stream_wait_time_ms,
            max_num_stream_timeouts,
            self.active_data_stream.as_mut(),
        )
        .await;
        if matches!(result, Err(Error::CriticalDataStreamTimeout(_))) {
            // If the stream has timed out too many times, we need to reset it
            warn!("Resetting the currently active data stream due to too many timeouts!");
            self.reset_active_stream(None).await?;
        }
        result
    }

    // TODO: don't drop the auxiliary infos here!

    /// Processes any notifications already pending on the active stream
    async fn process_active_stream_notifications(&mut self) -> Result<(), Error> {
        let state_sync_driver_config = &self.driver_configuration.config;
        for _ in 0..state_sync_driver_config.max_consecutive_stream_notifications {
            // Fetch and process any data notifications
            let data_notification = self.fetch_next_data_notification().await?;
            match data_notification.data_payload {
                DataPayload::StateValuesWithProof(state_value_chunk_with_proof) => {
                    self.process_state_values_payload(
                        data_notification.notification_id,
                        state_value_chunk_with_proof,
                    )
                    .await?;
                },
                DataPayload::EpochEndingLedgerInfos(epoch_ending_ledger_infos) => {
                    self.process_epoch_ending_payload(
                        data_notification.notification_id,
                        epoch_ending_ledger_infos,
                    )
                    .await?;
                },
                DataPayload::TransactionsWithProof(transactions_with_proof) => {
                    let payload_start_version =
                        transactions_with_proof.get_first_transaction_version();
                    let notification_metadata = NotificationMetadata::new(
                        data_notification.creation_time,
                        data_notification.notification_id,
                    );
                    self.process_transaction_or_output_payload(
                        notification_metadata,
                        Some(
                            transactions_with_proof
                                .get_transaction_list_with_proof()
                                .clone(),
                        ),
                        None,
                        payload_start_version,
                    )
                    .await?;
                },
                DataPayload::TransactionOutputsWithProof(transaction_outputs_with_proof) => {
                    let payload_start_version =
                        transaction_outputs_with_proof.get_first_output_version();
                    let notification_metadata = NotificationMetadata::new(
                        data_notification.creation_time,
                        data_notification.notification_id,
                    );
                    self.process_transaction_or_output_payload(
                        notification_metadata,
                        None,
                        Some(
                            transaction_outputs_with_proof
                                .get_output_list_with_proof()
                                .clone(),
                        ),
                        payload_start_version,
                    )
                    .await?;
                },
                _ => {
                    return self
                        .handle_end_of_stream_or_invalid_payload(data_notification)
                        .await
                },
            }
        }

        Ok(())
    }

    /// Fetches state values (as required to bootstrap the node)
    async fn fetch_missing_state_values(
        &mut self,
        target_ledger_info: LedgerInfoWithSignatures,
        existing_snapshot_progress: bool,
    ) -> Result<(), Error> {
        // Initialize the target ledger info and verify it never changes
        if let Some(ledger_info_to_sync) = &self.state_value_syncer.ledger_info_to_sync {
            if ledger_info_to_sync != &target_ledger_info {
                return Err(Error::UnexpectedError(format!(
                    "Mismatch in ledger info to sync! Given target: {:?}, stored target: {:?}",
                    target_ledger_info, ledger_info_to_sync
                )));
            }
        } else {
            info!(LogSchema::new(LogEntry::Bootstrapper).message(&format!(
                "Setting the target ledger info for fast sync! Target: {:?}",
                target_ledger_info
            )));

            self.state_value_syncer
                .set_ledger_info_to_sync(target_ledger_info.clone());
        }

        // Fetch the data that we're missing
        let target_ledger_info_version = target_ledger_info.ledger_info().version();
        let data_stream = if self.state_value_syncer.transaction_output_to_sync.is_none() {
            // Fetch the transaction info first, before the states
            self.streaming_client
                .get_all_transaction_outputs(
                    target_ledger_info_version,
                    target_ledger_info_version,
                    target_ledger_info_version,
                )
                .await?
        } else {
            // Identify the next state index to fetch
            let next_state_index_to_process = if existing_snapshot_progress {
                // The state snapshot receiver requires that after each reboot we
                // rewrite the last persisted index (again!). This is a limitation
                // of how the snapshot is persisted (i.e., in-memory sibling freezing).
                // Thus, on each stream reset, we overlap every chunk by a single item.
                self
                    .metadata_storage
                    .get_last_persisted_state_value_index(&target_ledger_info)
                    .map_err(|error| {
                        Error::StorageError(format!(
                            "Failed to get the last persisted state value index at version {:?}! Error: {:?}",
                            target_ledger_info_version, error
                        ))
                    })?
            } else {
                0 // We need to start the snapshot sync from index 0
            };

            // Fetch the missing state values
            self.state_value_syncer
                .update_next_state_index_to_process(next_state_index_to_process);
            self.streaming_client
                .get_all_state_values(
                    target_ledger_info_version,
                    Some(next_state_index_to_process),
                )
                .await?
        };
        self.active_data_stream = Some(data_stream);

        Ok(())
    }

    /// Fetches all missing transaction data in order to bootstrap the node
    async fn fetch_missing_transaction_data(
        &mut self,
        highest_synced_version: Version,
        highest_known_ledger_info: LedgerInfoWithSignatures,
    ) -> Result<(), Error> {
        let highest_known_ledger_version = highest_known_ledger_info.ledger_info().version();
        let next_version = highest_synced_version.checked_add(1).ok_or_else(|| {
            Error::IntegerOverflow("The next output version has overflown!".into())
        })?;
        let end_version = self
            .verified_epoch_states
            .next_epoch_ending_version(highest_synced_version)
            .ok_or_else(|| {
                Error::UnexpectedError("No higher epoch ending version known!".into())
            })?;
        let data_stream = match self.get_bootstrapping_mode() {
            BootstrappingMode::ApplyTransactionOutputsFromGenesis => {
                self.streaming_client
                    .get_all_transaction_outputs(
                        next_version,
                        end_version,
                        highest_known_ledger_version,
                    )
                    .await?
            },
            BootstrappingMode::ExecuteTransactionsFromGenesis => {
                self.streaming_client
                    .get_all_transactions(
                        next_version,
                        end_version,
                        highest_known_ledger_version,
                        false,
                    )
                    .await?
            },
            BootstrappingMode::ExecuteOrApplyFromGenesis => {
                if self.output_fallback_handler.in_fallback_mode() {
                    metrics::set_gauge(
                        &metrics::DRIVER_FALLBACK_MODE,
                        ExecutingComponent::Bootstrapper.get_label(),
                        1,
                    );
                    self.streaming_client
                        .get_all_transaction_outputs(
                            next_version,
                            end_version,
                            highest_known_ledger_version,
                        )
                        .await?
                } else {
                    metrics::set_gauge(
                        &metrics::DRIVER_FALLBACK_MODE,
                        ExecutingComponent::Bootstrapper.get_label(),
                        0,
                    );
                    self.streaming_client
                        .get_all_transactions_or_outputs(
                            next_version,
                            end_version,
                            highest_known_ledger_version,
                            false,
                        )
                        .await?
                }
            },
            bootstrapping_mode => {
                unreachable!("Bootstrapping mode not supported: {:?}", bootstrapping_mode)
            },
        };
        self.speculative_stream_state = Some(SpeculativeStreamState::new(
            utils::fetch_latest_epoch_state(self.storage.clone())?,
            Some(highest_known_ledger_info),
            highest_synced_version,
        ));
        self.active_data_stream = Some(data_stream);

        Ok(())
    }

    /// Fetches all epoch ending ledger infos (from the current epoch to the
    /// maximum that can be found by the data streaming service).
    async fn fetch_epoch_ending_ledger_infos(
        &mut self,
        global_data_summary: &GlobalDataSummary,
    ) -> Result<(), Error> {
        // Verify the waypoint can be satisfied
        self.verify_waypoint_is_satisfiable(global_data_summary)?;

        // Get the highest advertised epoch that has ended
        let highest_advertised_epoch_end = global_data_summary
            .advertised_data
            .highest_epoch_ending_ledger_info()
            .ok_or_else(|| {
                Error::AdvertisedDataError(
                    "No highest advertised epoch end found in the network!".into(),
                )
            })?;

        // Fetch the highest epoch end known locally
        let highest_known_ledger_info = self.get_highest_known_ledger_info()?;
        let highest_known_ledger_info = highest_known_ledger_info.ledger_info();
        let highest_local_epoch_end = if highest_known_ledger_info.ends_epoch() {
            highest_known_ledger_info.epoch()
        } else if highest_known_ledger_info.epoch() > 0 {
            highest_known_ledger_info
                .epoch()
                .checked_sub(1)
                .ok_or_else(|| {
                    Error::IntegerOverflow("The highest local epoch end has overflown!".into())
                })?
        } else {
            unreachable!("Genesis should always end the first epoch!");
        };

        // Compare the highest local epoch end to the highest advertised epoch end
        if highest_local_epoch_end < highest_advertised_epoch_end {
            info!(LogSchema::new(LogEntry::Bootstrapper).message(&format!(
                "Found higher epoch ending ledger infos in the network! Local: {:?}, advertised: {:?}",
                   highest_local_epoch_end, highest_advertised_epoch_end
            )));
            let next_epoch_end = highest_local_epoch_end.checked_add(1).ok_or_else(|| {
                Error::IntegerOverflow("The next epoch end has overflown!".into())
            })?;
            let epoch_ending_stream = self
                .streaming_client
                .get_all_epoch_ending_ledger_infos(next_epoch_end)
                .await?;
            self.active_data_stream = Some(epoch_ending_stream);
        } else if self.verified_epoch_states.verified_waypoint() {
            info!(LogSchema::new(LogEntry::Bootstrapper).message(
                "No new epoch ending ledger infos to fetch! All peers are in the same epoch!"
            ));
            self.verified_epoch_states
                .set_fetched_epoch_ending_ledger_infos();
        } else {
            return Err(Error::AdvertisedDataError(format!(
                "Our waypoint is unverified, but there's no higher epoch ending ledger infos \
                advertised! Highest local epoch end: {:?}, highest advertised epoch end: {:?}",
                highest_local_epoch_end, highest_advertised_epoch_end
            )));
        };

        Ok(())
    }

    /// Verifies that connected peers have advertised data beyond our waypoint
    /// or that our waypoint is trivially satisfiable.
    fn verify_waypoint_is_satisfiable(
        &mut self,
        global_data_summary: &GlobalDataSummary,
    ) -> Result<(), Error> {
        // If our storage has already synced beyond our waypoint, nothing needs to be checked
        let latest_ledger_info = utils::fetch_latest_synced_ledger_info(self.storage.clone())?;
        let waypoint_version = self.driver_configuration.waypoint.version();
        if latest_ledger_info.ledger_info().version() >= waypoint_version {
            self.verified_epoch_states
                .set_verified_waypoint(waypoint_version);
            return Ok(());
        }

        // Get the highest advertised synced ledger info version
        let highest_advertised_ledger_info = global_data_summary
            .advertised_data
            .highest_synced_ledger_info()
            .ok_or_else(|| {
                Error::UnsatisfiableWaypoint(
                    "Unable to check waypoint satisfiability! No highest advertised ledger info found in the network!".into(),
                )
            })?;
        let highest_advertised_version = highest_advertised_ledger_info.ledger_info().version();

        // Compare the highest advertised version with our waypoint
        if highest_advertised_version < waypoint_version {
            Err(Error::UnsatisfiableWaypoint(
                format!(
                    "The waypoint is not satisfiable! No advertised version higher than our waypoint! Highest version: {:?}, waypoint version: {:?}.",
                    highest_advertised_version, waypoint_version
                )
            ))
        } else {
            Ok(())
        }
    }

    /// Verifies the start and end indices in the given state value chunk
    async fn verify_states_values_indices(
        &mut self,
        notification_id: NotificationId,
        state_value_chunk_with_proof: &StateValueChunkWithProof,
    ) -> Result<(), Error> {
        // Verify the payload start index is valid
        let expected_start_index = self.state_value_syncer.next_state_index_to_process;
        if expected_start_index != state_value_chunk_with_proof.first_index {
            self.reset_active_stream(Some(NotificationAndFeedback::new(
                notification_id,
                NotificationFeedback::InvalidPayloadData,
            )))
            .await?;
            return Err(Error::VerificationError(format!(
                "The start index of the state values was invalid! Expected: {:?}, received: {:?}",
                expected_start_index, state_value_chunk_with_proof.first_index
            )));
        }

        // Verify the end index and number of state values is valid
        let expected_num_state_values = state_value_chunk_with_proof
            .last_index
            .checked_sub(state_value_chunk_with_proof.first_index)
            .and_then(|version| version.checked_add(1)) // expected_num_state_values = last_index - first_index + 1
            .ok_or_else(|| {
                Error::IntegerOverflow("The expected number of state values has overflown!".into())
            })?;
        let num_state_values = state_value_chunk_with_proof.raw_values.len() as u64;
        if expected_num_state_values != num_state_values {
            self.reset_active_stream(Some(NotificationAndFeedback::new(
                notification_id,
                NotificationFeedback::InvalidPayloadData,
            )))
            .await?;
            return Err(Error::VerificationError(format!(
                "The expected number of state values was invalid! Expected: {:?}, received: {:?}",
                expected_num_state_values, num_state_values,
            )));
        }

        Ok(())
    }

    /// Process a single state value chunk with proof payload
    async fn process_state_values_payload(
        &mut self,
        notification_id: NotificationId,
        state_value_chunk_with_proof: StateValueChunkWithProof,
    ) -> Result<(), Error> {
        // Verify that we're expecting state value payloads
        let bootstrapping_mode = self.get_bootstrapping_mode();
        if self.should_fetch_epoch_ending_ledger_infos() || !bootstrapping_mode.is_fast_sync() {
            self.reset_active_stream(Some(NotificationAndFeedback::new(
                notification_id,
                NotificationFeedback::InvalidPayloadData,
            )))
            .await?;
            return Err(Error::InvalidPayload(
                "Received an unexpected state values payload!".into(),
            ));
        }

        // Fetch the target ledger info and transaction info for bootstrapping
        let ledger_info_to_sync = self.get_ledger_info_to_sync()?;
        let transaction_output_to_sync = self.get_transaction_output_to_sync()?;

        // Initialize the state value synchronizer (if not already done)
        if !self.state_value_syncer.initialized_state_snapshot_receiver {
            // Fetch all verified epoch change proofs
            let version_to_sync = ledger_info_to_sync.ledger_info().version();
            let epoch_change_proofs = if version_to_sync == GENESIS_TRANSACTION_VERSION {
                vec![ledger_info_to_sync.clone()] // Sync to genesis
            } else {
                self.verified_epoch_states.all_epoch_ending_ledger_infos() // Sync beyond genesis
            };

            // Initialize the state value synchronizer
            let _join_handle = self.storage_synchronizer.initialize_state_synchronizer(
                epoch_change_proofs,
                ledger_info_to_sync,
                transaction_output_to_sync.clone(),
            )?;
            self.state_value_syncer.initialized_state_snapshot_receiver = true;
        }

        // Verify the state values payload start and end indices
        self.verify_states_values_indices(notification_id, &state_value_chunk_with_proof)
            .await?;

        // Verify the chunk root hash matches the expected root hash
        let first_transaction_info = transaction_output_to_sync
            .proof
            .transaction_infos
            .first()
            .ok_or_else(|| {
                Error::UnexpectedError("Target transaction info does not exist!".into())
            })?;
        let expected_root_hash = first_transaction_info
            .ensure_state_checkpoint_hash()
            .map_err(|error| {
                Error::UnexpectedError(format!("State checkpoint must exist! Error: {:?}", error))
            })?;
        if state_value_chunk_with_proof.root_hash != expected_root_hash {
            self.reset_active_stream(Some(NotificationAndFeedback::new(
                notification_id,
                NotificationFeedback::InvalidPayloadData,
            )))
            .await?;
            return Err(Error::VerificationError(format!(
                "The states chunk with proof root hash: {:?} didn't match the expected hash: {:?}!",
                state_value_chunk_with_proof.root_hash, expected_root_hash,
            )));
        }

        // Process the state values chunk and proof
        let last_state_value_index = state_value_chunk_with_proof.last_index;
        if let Err(error) = self
            .storage_synchronizer
            .save_state_values(notification_id, state_value_chunk_with_proof)
            .await
        {
            self.reset_active_stream(Some(NotificationAndFeedback::new(
                notification_id,
                NotificationFeedback::InvalidPayloadData,
            )))
            .await?;
            return Err(Error::InvalidPayload(format!(
                "The states chunk with proof was invalid! Error: {:?}",
                error,
            )));
        }

        // Update the next state value index to process
        self.state_value_syncer.next_state_index_to_process =
            last_state_value_index.checked_add(1).ok_or_else(|| {
                Error::IntegerOverflow(
                    "The next state value index to process has overflown!".into(),
                )
            })?;

        Ok(())
    }

    /// Process a single epoch ending payload
    async fn process_epoch_ending_payload(
        &mut self,
        notification_id: NotificationId,
        epoch_ending_ledger_infos: Vec<LedgerInfoWithSignatures>,
    ) -> Result<(), Error> {
        // Verify that we're expecting epoch ending ledger info payloads
        if !self.should_fetch_epoch_ending_ledger_infos() {
            self.reset_active_stream(Some(NotificationAndFeedback::new(
                notification_id,
                NotificationFeedback::InvalidPayloadData,
            )))
            .await?;
            return Err(Error::InvalidPayload(
                "Received an unexpected epoch ending payload!".into(),
            ));
        }

        // Verify the payload isn't empty
        if epoch_ending_ledger_infos.is_empty() {
            self.reset_active_stream(Some(NotificationAndFeedback::new(
                notification_id,
                NotificationFeedback::EmptyPayloadData,
            )))
            .await?;
            return Err(Error::VerificationError(
                "The epoch ending payload was empty!".into(),
            ));
        }

        // Verify the epoch change proofs, update our latest epoch state and
        // verify our waypoint.
        for epoch_ending_ledger_info in epoch_ending_ledger_infos {
            if let Err(error) = self.verified_epoch_states.update_verified_epoch_states(
                &epoch_ending_ledger_info,
                &self.driver_configuration.waypoint,
            ) {
                self.reset_active_stream(Some(NotificationAndFeedback::new(
                    notification_id,
                    NotificationFeedback::PayloadProofFailed,
                )))
                .await?;
                return Err(error);
            }
        }

        // TODO(joshlind): do we want to preemptively notify certain components
        // of the new reconfigurations?

        Ok(())
    }

    /// Process a single transaction or transaction output data payload
    async fn process_transaction_or_output_payload(
        &mut self,
        notification_metadata: NotificationMetadata,
        transaction_list_with_proof: Option<TransactionListWithProof>,
        transaction_outputs_with_proof: Option<TransactionOutputListWithProof>,
        payload_start_version: Option<Version>,
    ) -> Result<(), Error> {
        // Verify that we're expecting transaction or output payloads
        let bootstrapping_mode = self.get_bootstrapping_mode();
        if self.should_fetch_epoch_ending_ledger_infos()
            || (bootstrapping_mode.is_fast_sync()
                && self.state_value_syncer.transaction_output_to_sync.is_some())
        {
            self.reset_active_stream(Some(NotificationAndFeedback::new(
                notification_metadata.notification_id,
                NotificationFeedback::InvalidPayloadData,
            )))
            .await?;
            return Err(Error::InvalidPayload(
                "Received an unexpected transaction or output payload!".into(),
            ));
        }

        // If we're fast syncing, we expect a single transaction info
        if bootstrapping_mode.is_fast_sync() {
            return self
                .verify_transaction_info_to_sync(
                    notification_metadata.notification_id,
                    transaction_outputs_with_proof,
                    payload_start_version,
                )
                .await;
        }

        // Verify the payload starting version
        let expected_start_version = self
            .get_speculative_stream_state()?
            .expected_next_version()?;
        let payload_start_version = self
            .verify_payload_start_version(
                notification_metadata.notification_id,
                payload_start_version,
                expected_start_version,
            )
            .await?;

        // Get the expected proof ledger info for the payload
        let proof_ledger_info = self
            .get_speculative_stream_state()?
            .get_proof_ledger_info()?;

        // Get the end of epoch ledger info if the payload ends the epoch
        let end_of_epoch_ledger_info = self
            .get_end_of_epoch_ledger_info(
                notification_metadata.notification_id,
                payload_start_version,
                transaction_list_with_proof.as_ref(),
                transaction_outputs_with_proof.as_ref(),
            )
            .await?;

        // Execute/apply and commit the transactions/outputs
        let num_transactions_or_outputs = match bootstrapping_mode {
            BootstrappingMode::ApplyTransactionOutputsFromGenesis => {
                if let Some(transaction_outputs_with_proof) = transaction_outputs_with_proof {
                    utils::apply_transaction_outputs(
                        &mut self.storage_synchronizer,
                        notification_metadata,
                        proof_ledger_info,
                        end_of_epoch_ledger_info,
                        transaction_outputs_with_proof,
                    )
                    .await?
                } else {
                    self.reset_active_stream(Some(NotificationAndFeedback::new(
                        notification_metadata.notification_id,
                        NotificationFeedback::PayloadTypeIsIncorrect,
                    )))
                    .await?;
                    return Err(Error::InvalidPayload(
                        "Did not receive transaction outputs with proof!".into(),
                    ));
                }
            },
            BootstrappingMode::ExecuteTransactionsFromGenesis => {
                if let Some(transaction_list_with_proof) = transaction_list_with_proof {
                    utils::execute_transactions(
                        &mut self.storage_synchronizer,
                        notification_metadata,
                        proof_ledger_info,
                        end_of_epoch_ledger_info,
                        transaction_list_with_proof,
                    )
                    .await?
                } else {
                    self.reset_active_stream(Some(NotificationAndFeedback::new(
                        notification_metadata.notification_id,
                        NotificationFeedback::PayloadTypeIsIncorrect,
                    )))
                    .await?;
                    return Err(Error::InvalidPayload(
                        "Did not receive transactions with proof!".into(),
                    ));
                }
            },
            BootstrappingMode::ExecuteOrApplyFromGenesis => {
                if let Some(transaction_list_with_proof) = transaction_list_with_proof {
                    utils::execute_transactions(
                        &mut self.storage_synchronizer,
                        notification_metadata,
                        proof_ledger_info,
                        end_of_epoch_ledger_info,
                        transaction_list_with_proof,
                    )
                    .await?
                } else if let Some(transaction_outputs_with_proof) = transaction_outputs_with_proof
                {
                    utils::apply_transaction_outputs(
                        &mut self.storage_synchronizer,
                        notification_metadata,
                        proof_ledger_info,
                        end_of_epoch_ledger_info,
                        transaction_outputs_with_proof,
                    )
                    .await?
                } else {
                    self.reset_active_stream(Some(NotificationAndFeedback::new(
                        notification_metadata.notification_id,
                        NotificationFeedback::PayloadTypeIsIncorrect,
                    )))
                    .await?;
                    return Err(Error::InvalidPayload(
                        "Did not receive transactions or outputs with proof!".into(),
                    ));
                }
            },
            bootstrapping_mode => {
                unreachable!("Bootstrapping mode not supported: {:?}", bootstrapping_mode)
            },
        };
        let synced_version = payload_start_version
            .checked_add(num_transactions_or_outputs as u64)
            .and_then(|version| version.checked_sub(1)) // synced_version = start + num txns/outputs - 1
            .ok_or_else(|| Error::IntegerOverflow("The synced version has overflown!".into()))?;
        self.get_speculative_stream_state()?
            .update_synced_version(synced_version);

        Ok(())
    }

    /// Verifies the payload contains the transaction info we require to
    /// download all state values.
    async fn verify_transaction_info_to_sync(
        &mut self,
        notification_id: NotificationId,
        transaction_outputs_with_proof: Option<TransactionOutputListWithProof>,
        payload_start_version: Option<Version>,
    ) -> Result<(), Error> {
        // Verify the payload starting version
        let ledger_info_to_sync = self.get_ledger_info_to_sync()?;
        let expected_start_version = ledger_info_to_sync.ledger_info().version();
        let _ = self
            .verify_payload_start_version(
                notification_id,
                payload_start_version,
                expected_start_version,
            )
            .await?;

        // Verify the payload proof (the ledger info has already been verified)
        // and save the transaction output with proof.
        if let Some(transaction_outputs_with_proof) = transaction_outputs_with_proof {
            if transaction_outputs_with_proof.proof.transaction_infos.len() == 1 {
                match transaction_outputs_with_proof.verify(
                    ledger_info_to_sync.ledger_info(),
                    Some(expected_start_version),
                ) {
                    Ok(()) => {
                        self.state_value_syncer
                            .set_transaction_output_to_sync(transaction_outputs_with_proof);
                    },
                    Err(error) => {
                        self.reset_active_stream(Some(NotificationAndFeedback::new(
                            notification_id,
                            NotificationFeedback::PayloadProofFailed,
                        )))
                        .await?;
                        return Err(Error::VerificationError(format!(
                            "Transaction outputs with proof is invalid! Error: {:?}",
                            error
                        )));
                    },
                }
            } else {
                self.reset_active_stream(Some(NotificationAndFeedback::new(
                    notification_id,
                    NotificationFeedback::InvalidPayloadData,
                )))
                .await?;
                return Err(Error::InvalidPayload(
                    "Payload does not contain a single transaction info!".into(),
                ));
            }
        } else {
            self.reset_active_stream(Some(NotificationAndFeedback::new(
                notification_id,
                NotificationFeedback::PayloadTypeIsIncorrect,
            )))
            .await?;
            return Err(Error::InvalidPayload(
                "Did not receive transaction outputs with proof!".into(),
            ));
        }

        Ok(())
    }

    /// Verifies the first payload version matches the version we wish to sync
    async fn verify_payload_start_version(
        &mut self,
        notification_id: NotificationId,
        payload_start_version: Option<Version>,
        expected_start_version: Version,
    ) -> Result<Version, Error> {
        if let Some(payload_start_version) = payload_start_version {
            if payload_start_version != expected_start_version {
                self.reset_active_stream(Some(NotificationAndFeedback::new(
                    notification_id,
                    NotificationFeedback::InvalidPayloadData,
                )))
                .await?;
                Err(Error::VerificationError(format!(
                    "The payload start version does not match the expected version! Start: {:?}, expected: {:?}",
                    payload_start_version, expected_start_version
                )))
            } else {
                Ok(payload_start_version)
            }
        } else {
            self.reset_active_stream(Some(NotificationAndFeedback::new(
                notification_id,
                NotificationFeedback::EmptyPayloadData,
            )))
            .await?;
            Err(Error::VerificationError(
                "The playload starting version is missing!".into(),
            ))
        }
    }

    /// Returns the end of epoch ledger info for the given payload. If
    /// calculation fails, the active stream is terminated. Assumes the
    /// `payload_start_version` exists.
    async fn get_end_of_epoch_ledger_info(
        &mut self,
        notification_id: NotificationId,
        payload_start_version: Version,
        transaction_list_with_proof: Option<&TransactionListWithProof>,
        transaction_outputs_with_proof: Option<&TransactionOutputListWithProof>,
    ) -> Result<Option<LedgerInfoWithSignatures>, Error> {
        // Calculate the payload end version
        let num_versions = match self.get_bootstrapping_mode() {
            BootstrappingMode::ApplyTransactionOutputsFromGenesis => {
                if let Some(transaction_outputs_with_proof) = transaction_outputs_with_proof {
                    transaction_outputs_with_proof.get_num_outputs()
                } else {
                    self.reset_active_stream(Some(NotificationAndFeedback::new(
                        notification_id,
                        NotificationFeedback::PayloadTypeIsIncorrect,
                    )))
                    .await?;
                    return Err(Error::InvalidPayload(
                        "Did not receive transaction outputs with proof!".into(),
                    ));
                }
            },
            BootstrappingMode::ExecuteTransactionsFromGenesis => {
                if let Some(transaction_list_with_proof) = transaction_list_with_proof {
                    transaction_list_with_proof.get_num_transactions()
                } else {
                    self.reset_active_stream(Some(NotificationAndFeedback::new(
                        notification_id,
                        NotificationFeedback::PayloadTypeIsIncorrect,
                    )))
                    .await?;
                    return Err(Error::InvalidPayload(
                        "Did not receive transactions with proof!".into(),
                    ));
                }
            },
            BootstrappingMode::ExecuteOrApplyFromGenesis => {
                if let Some(transaction_list_with_proof) = transaction_list_with_proof {
                    transaction_list_with_proof.get_num_transactions()
                } else if let Some(output_list_with_proof) = transaction_outputs_with_proof {
                    output_list_with_proof.get_num_outputs()
                } else {
                    self.reset_active_stream(Some(NotificationAndFeedback::new(
                        notification_id,
                        NotificationFeedback::PayloadTypeIsIncorrect,
                    )))
                    .await?;
                    return Err(Error::InvalidPayload(
                        "Did not receive transactions or outputs with proof!".into(),
                    ));
                }
            },
            bootstrapping_mode => {
                unimplemented!("Bootstrapping mode not supported: {:?}", bootstrapping_mode)
            },
        };
        let payload_end_version = payload_start_version
            .checked_add(num_versions as u64)
            .and_then(|v| v.checked_sub(1))
            .ok_or_else(|| {
                Error::IntegerOverflow("The payload end version has overflown!".into())
            })?; // payload_end_version = payload_start_version + num_versions - 1

        // Find any epoch ending ledger info at the payload end version
        Ok(self
            .verified_epoch_states
            .get_epoch_ending_ledger_info(payload_end_version))
    }

    /// Returns the highest known ledger info (including the newly fetch ones)
    fn get_highest_known_ledger_info(&self) -> Result<LedgerInfoWithSignatures, Error> {
        // Fetch the highest synced ledger info from storage
        let mut highest_known_ledger_info =
            utils::fetch_latest_synced_ledger_info(self.storage.clone())?;

        // Fetch the highest verified ledger info (from the network) and take
        // the maximum.
        if let Some(verified_ledger_info) =
            self.verified_epoch_states.get_highest_known_ledger_info()?
        {
            if verified_ledger_info.ledger_info().version()
                > highest_known_ledger_info.ledger_info().version()
            {
                highest_known_ledger_info = verified_ledger_info;
            }
        }
        Ok(highest_known_ledger_info)
    }

    /// Handles the end of stream notification or an invalid payload by
    /// terminating the stream appropriately.
    async fn handle_end_of_stream_or_invalid_payload(
        &mut self,
        data_notification: DataNotification,
    ) -> Result<(), Error> {
        // Calculate the feedback based on the notification
        let notification_feedback = match data_notification.data_payload {
            DataPayload::EndOfStream => NotificationFeedback::EndOfStream,
            _ => NotificationFeedback::PayloadTypeIsIncorrect,
        };
        let notification_and_feedback =
            NotificationAndFeedback::new(data_notification.notification_id, notification_feedback);

        // Reset the stream
        self.reset_active_stream(Some(notification_and_feedback))
            .await?;

        // Return an error if the payload was invalid
        match data_notification.data_payload {
            DataPayload::EndOfStream => Ok(()),
            _ => Err(Error::InvalidPayload("Unexpected payload type!".into())),
        }
    }

    /// Returns the speculative stream state
    fn get_speculative_stream_state(&mut self) -> Result<&mut SpeculativeStreamState, Error> {
        self.speculative_stream_state.as_mut().ok_or_else(|| {
            Error::UnexpectedError("Speculative stream state does not exist!".into())
        })
    }

    /// Returns the ledger info to sync
    fn get_ledger_info_to_sync(&mut self) -> Result<LedgerInfoWithSignatures, Error> {
        self.state_value_syncer
            .ledger_info_to_sync
            .clone()
            .ok_or_else(|| Error::UnexpectedError("The ledger info to sync is missing!".into()))
    }

    /// Returns the transaction output to sync
    fn get_transaction_output_to_sync(&mut self) -> Result<TransactionOutputListWithProof, Error> {
        self.state_value_syncer
            .transaction_output_to_sync
            .clone()
            .ok_or_else(|| {
                Error::UnexpectedError("The transaction output to sync is missing!".into())
            })
    }

    /// Handles the storage synchronizer error sent by the driver
    pub async fn handle_storage_synchronizer_error(
        &mut self,
        notification_and_feedback: NotificationAndFeedback,
    ) -> Result<(), Error> {
        // Reset the active stream
        self.reset_active_stream(Some(notification_and_feedback))
            .await?;

        // Fallback to output syncing if we need to
        if let BootstrappingMode::ExecuteOrApplyFromGenesis = self.get_bootstrapping_mode() {
            self.output_fallback_handler.fallback_to_outputs();
            metrics::set_gauge(
                &metrics::DRIVER_FALLBACK_MODE,
                ExecutingComponent::Bootstrapper.get_label(),
                1,
            );
        }

        Ok(())
    }

    /// Resets the currently active data stream and speculative state
    pub async fn reset_active_stream(
        &mut self,
        notification_and_feedback: Option<NotificationAndFeedback>,
    ) -> Result<(), Error> {
        if let Some(active_data_stream) = &self.active_data_stream {
            let data_stream_id = active_data_stream.data_stream_id;
            utils::terminate_stream_with_feedback(
                &mut self.streaming_client,
                data_stream_id,
                notification_and_feedback,
            )
            .await?;
        }

        self.active_data_stream = None;
        self.speculative_stream_state = None;
        Ok(())
    }

    /// Returns the verified epoch states struct for testing purposes
    #[cfg(test)]
    pub(crate) fn get_verified_epoch_states(&mut self) -> &mut VerifiedEpochStates {
        &mut self.verified_epoch_states
    }

    /// Returns the state value syncer struct for testing purposes
    #[cfg(test)]
    pub(crate) fn get_state_value_syncer(&mut self) -> &mut StateValueSyncer {
        &mut self.state_value_syncer
    }

    /// Manually sets the waypoint for testing purposes
    #[cfg(test)]
    pub(crate) fn set_waypoint(&mut self, waypoint: Waypoint) {
        self.driver_configuration.waypoint = waypoint;
    }
}

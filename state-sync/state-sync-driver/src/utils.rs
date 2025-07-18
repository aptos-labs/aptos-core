// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    driver::DriverConfiguration,
    error::Error,
    logging::{LogEntry, LogSchema},
    metrics,
    notification_handlers::{
        CommitNotification, CommittedTransactions, MempoolNotificationHandler,
        StorageServiceNotificationHandler,
    },
    storage_synchronizer::{NotificationMetadata, StorageSynchronizerInterface},
};
use aptos_data_streaming_service::{
    data_notification::DataNotification,
    data_stream::{DataStreamId, DataStreamListener},
    streaming_client::{DataStreamingClient, NotificationAndFeedback},
};
use aptos_event_notifications::EventSubscriptionService;
use aptos_infallible::Mutex;
use aptos_logger::prelude::*;
use aptos_mempool_notifications::MempoolNotificationSender;
use aptos_storage_interface::DbReader;
use aptos_storage_service_notifications::StorageServiceNotificationSender;
use aptos_time_service::{TimeService, TimeServiceTrait};
use aptos_types::{
    epoch_change::Verifier,
    epoch_state::EpochState,
    ledger_info::LedgerInfoWithSignatures,
    transaction::{TransactionListWithProofV2, TransactionOutputListWithProofV2, Version},
};
use futures::StreamExt;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::time::timeout;

pub const PENDING_DATA_LOG_FREQ_SECS: u64 = 3;

// TODO(joshlind): add unit tests to the speculative stream state.

/// The speculative state that tracks a data stream of transactions or outputs.
/// This assumes all data is valid and allows the driver to speculatively verify
/// payloads flowing along the stream without having to block on the executor or
/// storage. Thus, increasing syncing performance.
pub struct SpeculativeStreamState {
    epoch_state: EpochState,
    proof_ledger_info: Option<LedgerInfoWithSignatures>,
    synced_version: Version,
}

impl SpeculativeStreamState {
    pub fn new(
        epoch_state: EpochState,
        proof_ledger_info: Option<LedgerInfoWithSignatures>,
        synced_version: Version,
    ) -> Self {
        Self {
            epoch_state,
            proof_ledger_info,
            synced_version,
        }
    }

    /// Returns the next version that we expect along the stream
    pub fn expected_next_version(&self) -> Result<Version, Error> {
        self.synced_version.checked_add(1).ok_or_else(|| {
            Error::IntegerOverflow("The expected next version has overflown!".into())
        })
    }

    /// Returns the proof ledger info that all data along the stream should have
    /// proofs relative to. This assumes the proof ledger info exists!
    pub fn get_proof_ledger_info(&self) -> Result<LedgerInfoWithSignatures, Error> {
        self.proof_ledger_info
            .clone()
            .ok_or_else(|| Error::UnexpectedError("The proof ledger info is missing!".into()))
    }

    /// Updates the currently synced version of the stream
    pub fn update_synced_version(&mut self, synced_version: Version) {
        self.synced_version = synced_version;
    }

    /// Updates the epoch state if we've hit the specified target ledger
    /// info version and the ledger info has a new epoch state.
    pub fn maybe_update_epoch_state(
        &mut self,
        ledger_info_with_signatures: LedgerInfoWithSignatures,
    ) {
        if let Some(epoch_state) = ledger_info_with_signatures.ledger_info().next_epoch_state() {
            if ledger_info_with_signatures.ledger_info().version() == self.synced_version {
                self.epoch_state = epoch_state.clone();
            }
        }
    }

    /// Verifies the given ledger info with signatures against the current epoch state
    pub fn verify_ledger_info_with_signatures(
        &mut self,
        ledger_info_with_signatures: &LedgerInfoWithSignatures,
    ) -> Result<(), Error> {
        self.epoch_state
            .verify(ledger_info_with_signatures)
            .map_err(|error| {
                Error::VerificationError(format!("Ledger info failed verification: {:?}", error))
            })
    }
}

/// A simple struct that holds all information relevant for managing
/// fallback behaviour to output syncing.
#[derive(Clone)]
pub struct OutputFallbackHandler {
    // The configuration for the state sync driver
    driver_configuration: DriverConfiguration,

    // The most recent time at which we fell back to output syncing
    fallback_start_time: Arc<Mutex<Option<Instant>>>,

    // The time service
    time_service: TimeService,
}

impl OutputFallbackHandler {
    pub fn new(driver_configuration: DriverConfiguration, time_service: TimeService) -> Self {
        let fallback_start_time = Arc::new(Mutex::new(None));
        Self {
            driver_configuration,
            fallback_start_time,
            time_service,
        }
    }

    /// Initiates a fallback to output syncing (if we haven't already)
    pub fn fallback_to_outputs(&mut self) {
        let missing_fallback_start_time = self.fallback_start_time.lock().is_none();
        if missing_fallback_start_time {
            self.set_fallback_start_time(self.time_service.now());
            info!(LogSchema::new(LogEntry::Driver).message(&format!(
                "Falling back to output syncing for at least {:?} seconds!",
                self.get_fallback_duration().as_secs()
            )));
        }
    }

    /// Returns true iff we're currently in fallback mode
    pub fn in_fallback_mode(&mut self) -> bool {
        let fallback_start_time = self.fallback_start_time.lock().take();
        if let Some(fallback_start_time) = fallback_start_time {
            if let Some(fallback_deadline) =
                fallback_start_time.checked_add(self.get_fallback_duration())
            {
                // Check if we elapsed the max fallback duration
                if self.time_service.now() >= fallback_deadline {
                    info!(LogSchema::new(LogEntry::AutoBootstrapping)
                        .message("Passed the output fallback deadline! Disabling fallback mode!"));
                    false
                } else {
                    // Reinsert the fallback deadline (not enough time has passed)
                    self.set_fallback_start_time(fallback_start_time);
                    true
                }
            } else {
                warn!(LogSchema::new(LogEntry::Driver)
                    .message("The fallback deadline overflowed! Disabling fallback mode!"));
                false
            }
        } else {
            false
        }
    }

    /// Returns the fallback duration as defined by the config
    fn get_fallback_duration(&self) -> Duration {
        Duration::from_secs(
            self.driver_configuration
                .config
                .fallback_to_output_syncing_secs,
        )
    }

    /// Sets the fallback start time internally
    fn set_fallback_start_time(&mut self, fallback_start_time: Instant) {
        if let Some(old_start_time) = self.fallback_start_time.lock().replace(fallback_start_time) {
            warn!(LogSchema::new(LogEntry::Driver).message(&format!(
                "Overwrote the old fallback start time ({:?}) with the new one ({:?})!",
                old_start_time, fallback_start_time
            )));
        }
    }
}

/// Fetches a data notification from the given data stream listener. Returns an
/// error if the data stream times out after `max_stream_wait_time_ms`. Also,
/// tracks the number of consecutive timeouts to identify when the stream has
/// timed out too many times.
pub async fn get_data_notification(
    max_stream_wait_time_ms: u64,
    max_num_stream_timeouts: u64,
    active_data_stream: Option<&mut DataStreamListener>,
) -> Result<DataNotification, Error> {
    let active_data_stream = active_data_stream
        .ok_or_else(|| Error::UnexpectedError("The active data stream does not exist!".into()))?;

    let timeout_ms = Duration::from_millis(max_stream_wait_time_ms);
    if let Ok(data_notification) = timeout(timeout_ms, active_data_stream.select_next_some()).await
    {
        // Update the metrics for the data notification receive latency
        metrics::observe_duration(
            &metrics::DATA_NOTIFICATION_LATENCIES,
            metrics::NOTIFICATION_CREATE_TO_RECEIVE,
            data_notification.creation_time,
        );

        // Reset the number of consecutive timeouts for the data stream
        active_data_stream.num_consecutive_timeouts = 0;
        Ok(data_notification)
    } else {
        // Increase the number of consecutive timeouts for the data stream
        active_data_stream.num_consecutive_timeouts += 1;

        // Check if we've timed out too many times
        if active_data_stream.num_consecutive_timeouts >= max_num_stream_timeouts {
            Err(Error::CriticalDataStreamTimeout(format!(
                "{:?}",
                max_num_stream_timeouts
            )))
        } else {
            Err(Error::DataStreamNotificationTimeout(format!(
                "{:?}",
                timeout_ms
            )))
        }
    }
}

/// Terminates the stream with the provided notification ID and feedback
pub async fn terminate_stream_with_feedback<StreamingClient: DataStreamingClient + Clone>(
    streaming_client: &mut StreamingClient,
    data_stream_id: DataStreamId,
    notification_and_feedback: Option<NotificationAndFeedback>,
) -> Result<(), Error> {
    info!(LogSchema::new(LogEntry::Driver).message(&format!(
        "Terminating the data stream with ID: {:?}, notification and feedback: {:?}",
        data_stream_id, notification_and_feedback
    )));

    streaming_client
        .terminate_stream_with_feedback(data_stream_id, notification_and_feedback)
        .await
        .map_err(|error| error.into())
}

/// Fetches the latest epoch state from the specified storage
pub fn fetch_latest_epoch_state(storage: Arc<dyn DbReader>) -> Result<EpochState, Error> {
    storage.get_latest_epoch_state().map_err(|error| {
        Error::StorageError(format!(
            "Failed to get the latest epoch state from storage: {:?}",
            error
        ))
    })
}

/// Fetches the latest synced ledger info from the specified storage
pub fn fetch_latest_synced_ledger_info(
    storage: Arc<dyn DbReader>,
) -> Result<LedgerInfoWithSignatures, Error> {
    storage.get_latest_ledger_info().map_err(|error| {
        Error::StorageError(format!(
            "Failed to get the latest ledger info from storage: {:?}",
            error
        ))
    })
}

/// Fetches the latest synced version from the specified storage
pub fn fetch_pre_committed_version(storage: Arc<dyn DbReader>) -> Result<Version, Error> {
    storage.ensure_pre_committed_version().map_err(|e| {
        Error::StorageError(format!("Failed to get latest version from storage: {e:?}"))
    })
}

/// Initializes all relevant metric gauges (e.g., after a reboot
/// or after a state snapshot has been restored).
pub fn initialize_sync_gauges(storage: Arc<dyn DbReader>) -> Result<(), Error> {
    // Update the latest synced versions
    let highest_synced_version = fetch_pre_committed_version(storage.clone())?;
    let metrics = [
        metrics::StorageSynchronizerOperations::AppliedTransactionOutputs,
        metrics::StorageSynchronizerOperations::ExecutedTransactions,
        metrics::StorageSynchronizerOperations::Synced,
        metrics::StorageSynchronizerOperations::SyncedIncremental,
    ];
    for metric in metrics {
        metrics::set_gauge(
            &metrics::STORAGE_SYNCHRONIZER_OPERATIONS,
            metric.get_label(),
            highest_synced_version,
        );
    }

    // Update the latest synced epochs
    let highest_synced_epoch = fetch_latest_epoch_state(storage)?.epoch;
    let metrics = [
        metrics::StorageSynchronizerOperations::SyncedEpoch,
        metrics::StorageSynchronizerOperations::SyncedEpochIncremental,
    ];
    for metric in metrics {
        metrics::set_gauge(
            &metrics::STORAGE_SYNCHRONIZER_OPERATIONS,
            metric.get_label(),
            highest_synced_epoch,
        );
    }

    Ok(())
}

/// Handles a notification for committed transactions by
/// notifying mempool, the event subscription service and
/// the storage service.
pub async fn handle_committed_transactions<
    M: MempoolNotificationSender,
    S: StorageServiceNotificationSender,
>(
    committed_transactions: CommittedTransactions,
    storage: Arc<dyn DbReader>,
    mempool_notification_handler: MempoolNotificationHandler<M>,
    event_subscription_service: Arc<Mutex<EventSubscriptionService>>,
    storage_service_notification_handler: StorageServiceNotificationHandler<S>,
) {
    // Fetch the latest synced version and ledger info from storage
    let (latest_synced_version, latest_synced_ledger_info) =
        match fetch_pre_committed_version(storage.clone()) {
            Ok(latest_synced_version) => match fetch_latest_synced_ledger_info(storage.clone()) {
                Ok(latest_synced_ledger_info) => (latest_synced_version, latest_synced_ledger_info),
                Err(error) => {
                    error!(LogSchema::new(LogEntry::SynchronizerNotification)
                        .error(&error)
                        .message("Failed to fetch latest synced ledger info!"));
                    return;
                },
            },
            Err(error) => {
                error!(LogSchema::new(LogEntry::SynchronizerNotification)
                    .error(&error)
                    .message("Failed to fetch latest synced version!"));
                return;
            },
        };

    // Handle the commit notification
    if let Err(error) = CommitNotification::handle_transaction_notification(
        committed_transactions.events,
        committed_transactions.transactions,
        latest_synced_version,
        latest_synced_ledger_info,
        mempool_notification_handler,
        event_subscription_service,
        storage_service_notification_handler,
    )
    .await
    {
        error!(LogSchema::new(LogEntry::SynchronizerNotification)
            .error(&error)
            .message("Failed to handle a transaction commit notification!"));
    }
}

/// Updates the metrics to handle an epoch change event
pub fn update_new_epoch_metrics(storage: Arc<dyn DbReader>, reconfiguration_occurred: bool) {
    // Update the epoch metric (by reading directly from storage)
    let highest_synced_epoch = match fetch_latest_epoch_state(storage.clone()) {
        Ok(epoch_state) => epoch_state.epoch,
        Err(error) => {
            error!(LogSchema::new(LogEntry::Driver).message(&format!(
                "Failed to fetch the latest epoch state from storage! Error: {:?}",
                error
            )));
            return;
        },
    };
    metrics::set_gauge(
        &metrics::STORAGE_SYNCHRONIZER_OPERATIONS,
        metrics::StorageSynchronizerOperations::SyncedEpoch.get_label(),
        highest_synced_epoch,
    );

    // Update the incremental epoch metric (by incrementing the current value)
    if reconfiguration_occurred {
        metrics::increment_gauge(
            &metrics::STORAGE_SYNCHRONIZER_OPERATIONS,
            metrics::StorageSynchronizerOperations::SyncedEpochIncremental.get_label(),
            1,
        );
    }
}

/// Updates the metrics to handle newly synced transactions
pub fn update_new_synced_metrics(storage: Arc<dyn DbReader>, num_synced_transactions: usize) {
    // Update the version metric (by reading directly from storage)
    let highest_synced_version = match fetch_pre_committed_version(storage.clone()) {
        Ok(highest_synced_version) => highest_synced_version,
        Err(error) => {
            error!(LogSchema::new(LogEntry::Driver).message(&format!(
                "Failed to fetch the pre committed version from storage! Error: {:?}",
                error
            )));
            return;
        },
    };
    metrics::set_gauge(
        &metrics::STORAGE_SYNCHRONIZER_OPERATIONS,
        metrics::StorageSynchronizerOperations::Synced.get_label(),
        highest_synced_version,
    );

    // Update the incremental version metric (by incrementing the current value)
    metrics::increment_gauge(
        &metrics::STORAGE_SYNCHRONIZER_OPERATIONS,
        metrics::StorageSynchronizerOperations::SyncedIncremental.get_label(),
        num_synced_transactions as u64,
    );
}

/// Executes the given list of transactions and
/// returns the number of transactions in the list.
pub async fn execute_transactions<StorageSyncer: StorageSynchronizerInterface>(
    storage_synchronizer: &mut StorageSyncer,
    notification_metadata: NotificationMetadata,
    proof_ledger_info: LedgerInfoWithSignatures,
    end_of_epoch_ledger_info: Option<LedgerInfoWithSignatures>,
    transaction_list_with_proof: TransactionListWithProofV2,
) -> Result<usize, Error> {
    let num_transactions = transaction_list_with_proof.get_num_transactions();
    storage_synchronizer
        .execute_transactions(
            notification_metadata,
            transaction_list_with_proof,
            proof_ledger_info,
            end_of_epoch_ledger_info,
        )
        .await?;
    Ok(num_transactions)
}

/// Applies the given list of transaction outputs and
/// returns the number of outputs in the list.
pub async fn apply_transaction_outputs<StorageSyncer: StorageSynchronizerInterface>(
    storage_synchronizer: &mut StorageSyncer,
    notification_metadata: NotificationMetadata,
    proof_ledger_info: LedgerInfoWithSignatures,
    end_of_epoch_ledger_info: Option<LedgerInfoWithSignatures>,
    transaction_outputs_with_proof: TransactionOutputListWithProofV2,
) -> Result<usize, Error> {
    let num_transaction_outputs = transaction_outputs_with_proof.get_num_outputs();
    storage_synchronizer
        .apply_transaction_outputs(
            notification_metadata,
            transaction_outputs_with_proof,
            proof_ledger_info,
            end_of_epoch_ledger_info,
        )
        .await?;
    Ok(num_transaction_outputs)
}

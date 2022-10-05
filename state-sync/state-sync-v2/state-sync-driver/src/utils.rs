// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::Error,
    logging::{LogEntry, LogSchema},
    metrics,
    notification_handlers::{
        CommitNotification, CommittedTransactions, MempoolNotificationHandler,
    },
};
use aptos_infallible::Mutex;
use aptos_logger::prelude::*;
use aptos_types::{
    epoch_change::Verifier, epoch_state::EpochState, ledger_info::LedgerInfoWithSignatures,
    transaction::Version,
};
use data_streaming_service::data_stream::DataStreamId;
use data_streaming_service::streaming_client::NotificationAndFeedback;
use data_streaming_service::{
    data_notification::DataNotification, data_stream::DataStreamListener,
    streaming_client::DataStreamingClient,
};
use event_notifications::EventSubscriptionService;
use futures::StreamExt;
use mempool_notifications::MempoolNotificationSender;
use std::{sync::Arc, time::Duration};
use storage_interface::DbReader;
use tokio::time::timeout;

// TODO(joshlind): make these configurable!
const MAX_NUM_DATA_STREAM_TIMEOUTS: u64 = 3;
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
    pub fn get_proof_ledger_info(&self) -> LedgerInfoWithSignatures {
        self.proof_ledger_info
            .as_ref()
            .expect("Proof ledger info is missing!")
            .clone()
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

/// Fetches a data notification from the given data stream listener. Returns an
/// error if the data stream times out after `max_stream_wait_time_ms`. Also,
/// tracks the number of consecutive timeouts to identify when the stream has
/// timed out too many times.
///
/// Note: this assumes the `active_data_stream` exists.
pub async fn get_data_notification(
    max_stream_wait_time_ms: u64,
    active_data_stream: Option<&mut DataStreamListener>,
) -> Result<DataNotification, Error> {
    let active_data_stream = active_data_stream.expect("The active data stream should exist!");

    let timeout_ms = Duration::from_millis(max_stream_wait_time_ms);
    if let Ok(data_notification) = timeout(timeout_ms, active_data_stream.select_next_some()).await
    {
        // Reset the number of consecutive timeouts for the data stream
        active_data_stream.num_consecutive_timeouts = 0;
        Ok(data_notification)
    } else {
        // Increase the number of consecutive timeouts for the data stream
        active_data_stream.num_consecutive_timeouts += 1;

        // Check if we've timed out too many times
        if active_data_stream.num_consecutive_timeouts >= MAX_NUM_DATA_STREAM_TIMEOUTS {
            Err(Error::CriticalDataStreamTimeout(format!(
                "{:?}",
                MAX_NUM_DATA_STREAM_TIMEOUTS
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
pub fn fetch_latest_synced_version(storage: Arc<dyn DbReader>) -> Result<Version, Error> {
    let latest_transaction_info =
        storage
            .get_latest_transaction_info_option()
            .map_err(|error| {
                Error::StorageError(format!(
                    "Failed to get the latest transaction info from storage: {:?}",
                    error
                ))
            })?;
    latest_transaction_info
        .ok_or_else(|| Error::StorageError("Latest transaction info is missing!".into()))
        .map(|(latest_synced_version, _)| latest_synced_version)
}

/// Initializes all relevant metric gauges (e.g., after a reboot
/// or after a state snapshot has been restored).
pub fn initialize_sync_gauges(storage: Arc<dyn DbReader>) -> Result<(), Error> {
    // Update the latest synced versions
    let highest_synced_version = fetch_latest_synced_version(storage.clone())?;
    let metrics = [
        metrics::StorageSynchronizerOperations::AppliedTransactionOutputs,
        metrics::StorageSynchronizerOperations::ExecutedTransactions,
        metrics::StorageSynchronizerOperations::Synced,
    ];
    for metric in metrics {
        metrics::set_gauge(
            &metrics::STORAGE_SYNCHRONIZER_OPERATIONS,
            metric.get_label(),
            highest_synced_version,
        );
    }

    // Update the latest synced epoch
    let highest_synced_epoch = fetch_latest_epoch_state(storage)?.epoch;
    metrics::set_gauge(
        &metrics::STORAGE_SYNCHRONIZER_OPERATIONS,
        metrics::StorageSynchronizerOperations::SyncedEpoch.get_label(),
        highest_synced_epoch,
    );

    Ok(())
}

/// Handles a notification for committed transactions by
/// notifying mempool and the event subscription service.
pub async fn handle_committed_transactions<M: MempoolNotificationSender>(
    committed_transactions: CommittedTransactions,
    storage: Arc<dyn DbReader>,
    mempool_notification_handler: MempoolNotificationHandler<M>,
    event_subscription_service: Arc<Mutex<EventSubscriptionService>>,
) {
    // Fetch the latest synced version and ledger info from storage
    let (latest_synced_version, latest_synced_ledger_info) =
        match fetch_latest_synced_version(storage.clone()) {
            Ok(latest_synced_version) => match fetch_latest_synced_ledger_info(storage.clone()) {
                Ok(latest_synced_ledger_info) => (latest_synced_version, latest_synced_ledger_info),
                Err(error) => {
                    error!(LogSchema::new(LogEntry::SynchronizerNotification)
                        .error(&error)
                        .message("Failed to fetch latest synced ledger info!"));
                    return;
                }
            },
            Err(error) => {
                error!(LogSchema::new(LogEntry::SynchronizerNotification)
                    .error(&error)
                    .message("Failed to fetch latest synced version!"));
                return;
            }
        };

    // Handle the commit notification
    if let Err(error) = CommitNotification::handle_transaction_notification(
        committed_transactions.events,
        committed_transactions.transactions,
        latest_synced_version,
        latest_synced_ledger_info,
        mempool_notification_handler,
        event_subscription_service,
    )
    .await
    {
        error!(LogSchema::new(LogEntry::SynchronizerNotification)
            .error(&error)
            .message("Failed to handle a transaction commit notification!"));
    }
}

/// Updates the metrics to handle an epoch change event
pub fn update_new_epoch_metrics() {
    // Increment the epoch
    metrics::increment_gauge(
        &metrics::STORAGE_SYNCHRONIZER_OPERATIONS,
        metrics::StorageSynchronizerOperations::SyncedEpoch.get_label(),
        1,
    );
}

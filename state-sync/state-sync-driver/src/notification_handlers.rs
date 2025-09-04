// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::Error,
    logging::{LogEntry, LogSchema},
};
use aptos_consensus_notifications::{
    ConsensusCommitNotification, ConsensusNotification, ConsensusNotificationListener,
    ConsensusSyncDurationNotification, ConsensusSyncTargetNotification,
};
use aptos_data_streaming_service::data_notification::NotificationId;
use aptos_event_notifications::{EventNotificationSender, EventSubscriptionService};
use aptos_infallible::Mutex;
use aptos_logger::prelude::*;
use aptos_mempool_notifications::MempoolNotificationSender;
use aptos_storage_service_notifications::StorageServiceNotificationSender;
use aptos_time_service::{TimeService, TimeServiceTrait};
use aptos_types::{
    contract_event::ContractEvent,
    ledger_info::LedgerInfoWithSignatures,
    transaction::{Transaction, Version},
};
use futures::{channel::mpsc, stream::FusedStream, Stream};
use serde::Serialize;
use std::{
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Instant,
};

/// A notification for new data that has been committed to storage
#[derive(Clone, Debug)]
pub enum CommitNotification {
    CommittedStateSnapshot(CommittedStateSnapshot),
}

/// A commit notification for the new state snapshot
#[derive(Clone, Debug)]
pub struct CommittedStateSnapshot {
    pub committed_transaction: CommittedTransactions,
    pub last_committed_state_index: u64,
    pub version: Version,
}

/// A commit notification for new transactions
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommittedTransactions {
    pub events: Vec<ContractEvent>,
    pub transactions: Vec<Transaction>,
}

impl CommitNotification {
    pub fn new_committed_state_snapshot(
        events: Vec<ContractEvent>,
        transactions: Vec<Transaction>,
        last_committed_state_index: u64,
        version: Version,
    ) -> Self {
        let committed_transaction = CommittedTransactions {
            events,
            transactions,
        };
        let committed_states = CommittedStateSnapshot {
            committed_transaction,
            last_committed_state_index,
            version,
        };
        CommitNotification::CommittedStateSnapshot(committed_states)
    }

    /// Handles the commit notification by notifying mempool, the event
    /// subscription service and the storage service.
    pub async fn handle_transaction_notification<
        M: MempoolNotificationSender,
        S: StorageServiceNotificationSender,
    >(
        events: Vec<ContractEvent>,
        transactions: Vec<Transaction>,
        latest_synced_version: Version,
        latest_synced_ledger_info: LedgerInfoWithSignatures,
        mut mempool_notification_handler: MempoolNotificationHandler<M>,
        event_subscription_service: Arc<Mutex<EventSubscriptionService>>,
        mut storage_service_notification_handler: StorageServiceNotificationHandler<S>,
    ) -> Result<(), Error> {
        // Log the highest synced version and timestamp
        let blockchain_timestamp_usecs = latest_synced_ledger_info.ledger_info().timestamp_usecs();
        debug!(
            LogSchema::new(LogEntry::NotificationHandler).message(&format!(
                "Notifying the storage service, mempool and the event subscription service of version: {:?} and timestamp: {:?}.",
                latest_synced_version, blockchain_timestamp_usecs
            ))
        );

        // Notify the storage service of the committed transactions
        storage_service_notification_handler
            .notify_storage_service_of_committed_transactions(latest_synced_version)
            .await?;

        // Notify mempool of the committed transactions
        mempool_notification_handler
            .notify_mempool_of_committed_transactions(transactions, blockchain_timestamp_usecs)
            .await?;

        // Notify the event subscription service of the events
        event_subscription_service
            .lock()
            .notify_events(latest_synced_version, events)?;

        Ok(())
    }
}

/// A simple wrapper for a commit notification listener
pub struct CommitNotificationListener {
    // The listener for commit notifications
    commit_notification_listener: mpsc::UnboundedReceiver<CommitNotification>,
}

impl CommitNotificationListener {
    pub fn new() -> (mpsc::UnboundedSender<CommitNotification>, Self) {
        // Create a channel to send and receive commit notifications
        let (commit_notification_sender, commit_notification_listener) = mpsc::unbounded();

        // Create and return the sender and listener
        let commit_notification_listener = Self {
            commit_notification_listener,
        };
        (commit_notification_sender, commit_notification_listener)
    }
}

impl Stream for CommitNotificationListener {
    type Item = CommitNotification;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.get_mut().commit_notification_listener).poll_next(cx)
    }
}

impl FusedStream for CommitNotificationListener {
    fn is_terminated(&self) -> bool {
        self.commit_notification_listener.is_terminated()
    }
}

/// A consensus sync request for a specified target ledger info or duration
pub enum ConsensusSyncRequest {
    SyncDuration(Instant, ConsensusSyncDurationNotification), // The start time and duration to sync for
    SyncTarget(ConsensusSyncTargetNotification),              // The target ledger info to sync to
}

impl ConsensusSyncRequest {
    /// Returns a new sync target request
    pub fn new_with_target(sync_target_notification: ConsensusSyncTargetNotification) -> Self {
        ConsensusSyncRequest::SyncTarget(sync_target_notification)
    }

    /// Returns a new sync duration request
    pub fn new_with_duration(
        start_time: Instant,
        sync_duration_notification: ConsensusSyncDurationNotification,
    ) -> Self {
        ConsensusSyncRequest::SyncDuration(start_time, sync_duration_notification)
    }

    /// Returns the sync target (if one exists)
    pub fn get_sync_target(&self) -> Option<LedgerInfoWithSignatures> {
        match self {
            ConsensusSyncRequest::SyncTarget(sync_target_notification) => {
                Some(sync_target_notification.get_target().clone())
            },
            _ => None,
        }
    }

    /// Returns true iff the sync request is a duration request
    pub fn is_sync_duration_request(&self) -> bool {
        matches!(self, ConsensusSyncRequest::SyncDuration(_, _))
    }

    /// Returns true iff the sync request has been satisfied
    pub fn sync_request_satisfied(
        &self,
        latest_synced_ledger_info: &LedgerInfoWithSignatures,
        time_service: TimeService,
    ) -> bool {
        match self {
            ConsensusSyncRequest::SyncDuration(start_time, sync_duration_notification) => {
                // Get the duration and the current time
                let sync_duration = sync_duration_notification.get_duration();
                let current_time = time_service.now();

                // Check if the duration has been reached
                current_time.duration_since(*start_time) >= sync_duration
            },
            ConsensusSyncRequest::SyncTarget(sync_target_notification) => {
                // Get the sync target version and latest synced version
                let sync_target = sync_target_notification.get_target();
                let sync_target_version = sync_target.ledger_info().version();
                let latest_synced_version = latest_synced_ledger_info.ledger_info().version();

                // Check if we've satisfied the target
                latest_synced_version >= sync_target_version
            },
        }
    }
}

/// A simple handler for consensus or consensus observer notifications
pub struct ConsensusNotificationHandler {
    // The listener for notifications from consensus
    consensus_listener: ConsensusNotificationListener,

    // The latest consensus sync request that has been received
    consensus_sync_request: Arc<Mutex<Option<ConsensusSyncRequest>>>,

    // The time service
    time_service: TimeService,
}

impl ConsensusNotificationHandler {
    pub fn new(
        consensus_listener: ConsensusNotificationListener,
        time_service: TimeService,
    ) -> Self {
        Self {
            consensus_listener,
            consensus_sync_request: Arc::new(Mutex::new(None)),
            time_service,
        }
    }

    /// Returns true iff there is a sync request currently blocking consensus
    pub fn active_sync_request(&self) -> bool {
        self.consensus_sync_request.lock().is_some()
    }

    /// Returns the active sync request that consensus is waiting on
    pub fn get_sync_request(&self) -> Arc<Mutex<Option<ConsensusSyncRequest>>> {
        self.consensus_sync_request.clone()
    }

    /// Initializes the sync duration request received from consensus
    pub async fn initialize_sync_duration_request(
        &mut self,
        sync_duration_notification: ConsensusSyncDurationNotification,
    ) -> Result<(), Error> {
        // Get the current time
        let start_time = self.time_service.now();

        // Save the request so we can notify consensus once we've hit the duration
        let consensus_sync_request =
            ConsensusSyncRequest::new_with_duration(start_time, sync_duration_notification);
        self.consensus_sync_request = Arc::new(Mutex::new(Some(consensus_sync_request)));

        Ok(())
    }

    /// Initializes the sync target request received from consensus
    pub async fn initialize_sync_target_request(
        &mut self,
        sync_target_notification: ConsensusSyncTargetNotification,
        latest_pre_committed_version: Version,
        latest_synced_ledger_info: LedgerInfoWithSignatures,
    ) -> Result<(), Error> {
        // Get the target sync version and latest committed version
        let sync_target_version = sync_target_notification
            .get_target()
            .ledger_info()
            .version();
        let latest_committed_version = latest_synced_ledger_info.ledger_info().version();

        // If the target version is old, return an error to consensus (something is wrong!)
        if sync_target_version < latest_committed_version
            || sync_target_version < latest_pre_committed_version
        {
            let error = Err(Error::OldSyncRequest(
                sync_target_version,
                latest_pre_committed_version,
                latest_committed_version,
            ));
            self.respond_to_sync_target_notification(sync_target_notification, error.clone())?;
            return error;
        }

        // If the committed version is at the target, return successfully
        if sync_target_version == latest_committed_version {
            info!(
                LogSchema::new(LogEntry::NotificationHandler).message(&format!(
                    "We're already at the requested sync target version: {} \
                (pre-committed version: {}, committed version: {})!",
                    sync_target_version, latest_pre_committed_version, latest_committed_version
                ))
            );
            let result = Ok(());
            self.respond_to_sync_target_notification(sync_target_notification, result.clone())?;
            return result;
        }

        // If the pre-committed version is already at the target, something has else gone wrong
        if sync_target_version == latest_pre_committed_version {
            let error = Err(Error::InvalidSyncRequest(
                sync_target_version,
                latest_pre_committed_version,
            ));
            self.respond_to_sync_target_notification(sync_target_notification, error.clone())?;
            return error;
        }

        // Save the request so we can notify consensus once we've hit the target
        let consensus_sync_request =
            ConsensusSyncRequest::new_with_target(sync_target_notification);
        self.consensus_sync_request = Arc::new(Mutex::new(Some(consensus_sync_request)));

        Ok(())
    }

    /// Notifies consensus of a satisfied sync request, and removes the active request.
    /// Note: this assumes that the sync request has already been checked for satisfaction.
    pub async fn handle_satisfied_sync_request(
        &mut self,
        latest_synced_ledger_info: LedgerInfoWithSignatures,
    ) -> Result<(), Error> {
        // Remove the active sync request
        let mut sync_request_lock = self.consensus_sync_request.lock();
        let consensus_sync_request = sync_request_lock.take();

        // Notify consensus of the satisfied request
        match consensus_sync_request {
            Some(ConsensusSyncRequest::SyncDuration(_, sync_duration_notification)) => {
                self.respond_to_sync_duration_notification(
                    sync_duration_notification,
                    Ok(()),
                    Some(latest_synced_ledger_info),
                )?;
            },
            Some(ConsensusSyncRequest::SyncTarget(sync_target_notification)) => {
                // Get the sync target version and latest synced version
                let sync_target = sync_target_notification.get_target();
                let sync_target_version = sync_target.ledger_info().version();
                let latest_synced_version = latest_synced_ledger_info.ledger_info().version();

                // Check if we've synced beyond the target. If so, notify consensus with an error.
                if latest_synced_version > sync_target_version {
                    let error = Err(Error::SyncedBeyondTarget(
                        latest_synced_version,
                        sync_target_version,
                    ));
                    self.respond_to_sync_target_notification(
                        sync_target_notification,
                        error.clone(),
                    )?;
                    return error;
                }

                // Otherwise, notify consensus that the target has been reached
                self.respond_to_sync_target_notification(sync_target_notification, Ok(()))?;
            },
            None => { /* Nothing needs to be done */ },
        }

        Ok(())
    }

    /// Responds to consensus for a sync duration notification using the specified result
    pub fn respond_to_sync_duration_notification(
        &self,
        sync_duration_notification: ConsensusSyncDurationNotification,
        result: Result<(), Error>,
        latest_synced_ledger_info: Option<LedgerInfoWithSignatures>,
    ) -> Result<(), Error> {
        // Wrap the result in an error that consensus can process
        let result = result.map_err(|error| {
            aptos_consensus_notifications::Error::UnexpectedErrorEncountered(format!("{:?}", error))
        });

        // Send the result
        info!(
            LogSchema::new(LogEntry::NotificationHandler).message(&format!(
                "Responding to consensus sync duration notification with message: {:?}",
                result
            ))
        );
        self.consensus_listener
            .respond_to_sync_duration_notification(
                sync_duration_notification,
                result,
                latest_synced_ledger_info,
            )
            .map_err(|error| {
                Error::CallbackSendFailed(format!(
                    "Consensus sync duration response error: {:?}",
                    error
                ))
            })
    }

    /// Responds to consensus for a sync notification using the specified result
    pub fn respond_to_sync_target_notification(
        &self,
        sync_target_notification: ConsensusSyncTargetNotification,
        result: Result<(), Error>,
    ) -> Result<(), Error> {
        // Wrap the result in an error that consensus can process
        let result = result.map_err(|error| {
            aptos_consensus_notifications::Error::UnexpectedErrorEncountered(format!("{:?}", error))
        });

        // Send the result
        info!(
            LogSchema::new(LogEntry::NotificationHandler).message(&format!(
                "Responding to consensus sync target notification with message: {:?}",
                result
            ))
        );
        self.consensus_listener
            .respond_to_sync_target_notification(sync_target_notification, result)
            .map_err(|error| {
                Error::CallbackSendFailed(format!(
                    "Consensus sync target response error: {:?}",
                    error
                ))
            })
    }

    /// Responds successfully to consensus for a commit notification
    pub fn respond_to_commit_notification(
        &self,
        commit_notification: ConsensusCommitNotification,
        result: Result<(), Error>,
    ) -> Result<(), Error> {
        // Wrap the result in an error that consensus can process
        let result = result.map_err(|error| {
            aptos_consensus_notifications::Error::UnexpectedErrorEncountered(format!("{:?}", error))
        });

        // Send the result
        debug!(
            LogSchema::new(LogEntry::NotificationHandler).message(&format!(
                "Responding to consensus commit notification with message: {:?}",
                result
            ))
        );
        self.consensus_listener
            .respond_to_commit_notification(commit_notification, result)
            .map_err(|error| {
                Error::CallbackSendFailed(format!("Consensus commit response error: {:?}", error))
            })
    }
}

impl Stream for ConsensusNotificationHandler {
    type Item = ConsensusNotification;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.get_mut().consensus_listener).poll_next(cx)
    }
}

impl FusedStream for ConsensusNotificationHandler {
    fn is_terminated(&self) -> bool {
        self.consensus_listener.is_terminated()
    }
}

/// A notification for error transactions and events that have been committed to
/// storage.
#[derive(Clone, Debug, Serialize)]
pub struct ErrorNotification {
    pub error: Error,
    pub notification_id: NotificationId,
}

/// A simple wrapper for an error notification listener
pub struct ErrorNotificationListener {
    // The listener for error notifications
    error_notification_listener: mpsc::UnboundedReceiver<ErrorNotification>,
}

impl ErrorNotificationListener {
    pub fn new() -> (mpsc::UnboundedSender<ErrorNotification>, Self) {
        // Create a channel to send and receive error notifications
        let (error_notification_sender, error_notification_listener) = mpsc::unbounded();

        // Create and return the sender and listener
        let error_notification_listener = Self {
            error_notification_listener,
        };
        (error_notification_sender, error_notification_listener)
    }
}

impl Stream for ErrorNotificationListener {
    type Item = ErrorNotification;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.get_mut().error_notification_listener).poll_next(cx)
    }
}

impl FusedStream for ErrorNotificationListener {
    fn is_terminated(&self) -> bool {
        self.error_notification_listener.is_terminated()
    }
}

/// A simple handler for sending notifications to mempool
#[derive(Clone)]
pub struct MempoolNotificationHandler<M> {
    mempool_notification_sender: M,
}

impl<M: MempoolNotificationSender> MempoolNotificationHandler<M> {
    pub fn new(mempool_notification_sender: M) -> Self {
        Self {
            mempool_notification_sender,
        }
    }

    /// Notifies mempool that transactions have been committed.
    pub async fn notify_mempool_of_committed_transactions(
        &mut self,
        committed_transactions: Vec<Transaction>,
        block_timestamp_usecs: u64,
    ) -> Result<(), Error> {
        let result = self
            .mempool_notification_sender
            .notify_new_commit(committed_transactions, block_timestamp_usecs)
            .await;

        if let Err(error) = result {
            let error = Error::NotifyMempoolError(format!("{:?}", error));
            error!(LogSchema::new(LogEntry::NotificationHandler)
                .error(&error)
                .message("Failed to notify mempool of committed transactions!"));
            Err(error)
        } else {
            Ok(())
        }
    }
}

/// A simple handler for sending notifications to the storage service
#[derive(Clone)]
pub struct StorageServiceNotificationHandler<M> {
    storage_service_notification_sender: M,
}

impl<M: StorageServiceNotificationSender> StorageServiceNotificationHandler<M> {
    pub fn new(storage_service_notification_sender: M) -> Self {
        Self {
            storage_service_notification_sender,
        }
    }

    /// Notifies the storage service that transactions have been committed
    pub async fn notify_storage_service_of_committed_transactions(
        &mut self,
        highest_synced_version: u64,
    ) -> Result<(), Error> {
        // Notify the storage service
        let result = self
            .storage_service_notification_sender
            .notify_new_commit(highest_synced_version)
            .await;

        // Log any errors
        if let Err(error) = result {
            let error = Error::NotifyStorageServiceError(format!("{:?}", error));
            error!(LogSchema::new(LogEntry::NotificationHandler)
                .error(&error)
                .message("Failed to notify the storage service of committed transactions!"));
            Err(error)
        } else {
            Ok(())
        }
    }
}

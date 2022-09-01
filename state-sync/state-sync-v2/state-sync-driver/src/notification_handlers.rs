// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::Error,
    logging::{LogEntry, LogSchema},
};
use aptos_infallible::Mutex;
use aptos_logger::prelude::*;
use aptos_types::{
    contract_event::ContractEvent,
    ledger_info::LedgerInfoWithSignatures,
    transaction::{Transaction, Version},
};
use consensus_notifications::{
    ConsensusCommitNotification, ConsensusNotification, ConsensusNotificationListener,
    ConsensusSyncNotification,
};
use data_streaming_service::data_notification::NotificationId;
use event_notifications::{EventNotificationSender, EventSubscriptionService};
use futures::{channel::mpsc, stream::FusedStream, Stream};
use mempool_notifications::MempoolNotificationSender;
use serde::Serialize;
use std::{
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

// TODO(joshlind): make these configurable!
const MEMPOOL_COMMIT_ACK_TIMEOUT_MS: u64 = 5000; // 5 seconds

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

    /// Handles the commit notification by notifying mempool and the event
    /// subscription service.
    pub async fn handle_transaction_notification<M: MempoolNotificationSender>(
        events: Vec<ContractEvent>,
        transactions: Vec<Transaction>,
        latest_synced_version: Version,
        latest_synced_ledger_info: LedgerInfoWithSignatures,
        mut mempool_notification_handler: MempoolNotificationHandler<M>,
        event_subscription_service: Arc<Mutex<EventSubscriptionService>>,
    ) -> Result<(), Error> {
        // Notify mempool of the committed transactions
        debug!(
            LogSchema::new(LogEntry::NotificationHandler).message(&format!(
                "Notifying mempool of transactions at version: {:?}",
                latest_synced_version
            ))
        );
        let blockchain_timestamp_usecs = latest_synced_ledger_info.ledger_info().timestamp_usecs();
        mempool_notification_handler
            .notify_mempool_of_committed_transactions(
                transactions.clone(),
                blockchain_timestamp_usecs,
            )
            .await?;

        // Notify the event subscription service of the events
        debug!(
            LogSchema::new(LogEntry::NotificationHandler).message(&format!(
                "Notifying the event subscription service of events at version: {:?}",
                latest_synced_version
            ))
        );
        event_subscription_service
            .lock()
            .notify_events(latest_synced_version, events.clone())
            .map_err(|error| error.into())
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

/// A consensus sync request for a specified target ledger info
pub struct ConsensusSyncRequest {
    consensus_sync_notification: ConsensusSyncNotification,
}

impl ConsensusSyncRequest {
    pub fn new(consensus_sync_notification: ConsensusSyncNotification) -> Self {
        Self {
            consensus_sync_notification,
        }
    }

    pub fn get_sync_target(&self) -> LedgerInfoWithSignatures {
        self.consensus_sync_notification.target.clone()
    }

    pub fn get_sync_target_version(&self) -> Version {
        self.consensus_sync_notification
            .target
            .ledger_info()
            .version()
    }
}

/// A simple handler for consensus notifications
pub struct ConsensusNotificationHandler {
    // The listener for notifications from consensus
    consensus_listener: ConsensusNotificationListener,

    // The latest consensus sync request that has been received
    consensus_sync_request: Arc<Mutex<Option<ConsensusSyncRequest>>>,
}

impl ConsensusNotificationHandler {
    pub fn new(consensus_listener: ConsensusNotificationListener) -> Self {
        Self {
            consensus_listener,
            consensus_sync_request: Arc::new(Mutex::new(None)),
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

    /// Initializes the sync request received from consensus
    pub async fn initialize_sync_request(
        &mut self,
        sync_notification: ConsensusSyncNotification,
        latest_synced_ledger_info: LedgerInfoWithSignatures,
    ) -> Result<(), Error> {
        // Get the latest committed version and the target sync version
        let sync_target_version = sync_notification.target.ledger_info().version();
        let latest_committed_version = latest_synced_ledger_info.ledger_info().version();

        // If the target version is old, return an error to consensus (something is wrong!)
        if sync_target_version < latest_committed_version {
            let error = Err(Error::OldSyncRequest(
                sync_target_version,
                latest_committed_version,
            ));
            self.respond_to_sync_notification(sync_notification, error.clone())
                .await?;
            return error;
        }

        // If we're now at the target, return successfully
        if sync_target_version == latest_committed_version {
            info!(LogSchema::new(LogEntry::NotificationHandler)
                .message("We're already at the requested sync target version! Returning early"));
            let result = Ok(());
            self.respond_to_sync_notification(sync_notification, result.clone())
                .await?;
            return result;
        }

        // Save the request so we can notify consensus once we've hit the target
        let consensus_sync_request = ConsensusSyncRequest::new(sync_notification);
        self.consensus_sync_request = Arc::new(Mutex::new(Some(consensus_sync_request)));

        Ok(())
    }

    /// Checks to see if the sync request has been successfully fulfilled
    pub async fn check_sync_request_progress(
        &mut self,
        latest_synced_ledger_info: LedgerInfoWithSignatures,
    ) -> Result<(), Error> {
        // Fetch the sync target version
        let consensus_sync_request = self.get_sync_request();
        let sync_target_version = consensus_sync_request.lock().as_ref().map(|sync_request| {
            sync_request
                .consensus_sync_notification
                .target
                .ledger_info()
                .version()
        });

        // Compare our local state to the target version
        if let Some(sync_target_version) = sync_target_version {
            let latest_committed_version = latest_synced_ledger_info.ledger_info().version();

            // Check if we've synced beyond the target
            if latest_committed_version > sync_target_version {
                return Err(Error::SyncedBeyondTarget(
                    latest_committed_version,
                    sync_target_version,
                ));
            }

            // Check if we've hit the target
            if latest_committed_version == sync_target_version {
                let consensus_sync_request = self.get_sync_request().lock().take();
                if let Some(consensus_sync_request) = consensus_sync_request {
                    self.respond_to_sync_notification(
                        consensus_sync_request.consensus_sync_notification,
                        Ok(()),
                    )
                    .await?;
                }
                return Ok(());
            }
        }

        Ok(())
    }

    /// Responds to consensus for a sync notification using the specified result
    pub async fn respond_to_sync_notification(
        &mut self,
        sync_notification: ConsensusSyncNotification,
        result: Result<(), Error>,
    ) -> Result<(), Error> {
        // Wrap the result in an error that consensus can process
        let message = result.map_err(|error| {
            consensus_notifications::Error::UnexpectedErrorEncountered(format!("{:?}", error))
        });

        info!(
            LogSchema::new(LogEntry::NotificationHandler).message(&format!(
                "Responding to consensus sync notification with message: {:?}",
                message
            ))
        );

        // Send the result
        self.consensus_listener
            .respond_to_sync_notification(sync_notification, message)
            .await
            .map_err(|error| {
                Error::CallbackSendFailed(format!(
                    "Consensus sync request response error: {:?}",
                    error
                ))
            })
    }

    /// Responds successfully to consensus for a commit notification
    pub async fn respond_to_commit_notification(
        &mut self,
        commit_notification: ConsensusCommitNotification,
        result: Result<(), Error>,
    ) -> Result<(), Error> {
        // Wrap the result in an error that consensus can process
        let message = result.map_err(|error| {
            consensus_notifications::Error::UnexpectedErrorEncountered(format!("{:?}", error))
        });

        debug!(
            LogSchema::new(LogEntry::NotificationHandler).message(&format!(
                "Responding to consensus commit notification with message: {:?}",
                message
            ))
        );

        // Send the result
        self.consensus_listener
            .respond_to_commit_notification(commit_notification, message)
            .await
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
            .notify_new_commit(
                committed_transactions,
                block_timestamp_usecs,
                MEMPOOL_COMMIT_ACK_TIMEOUT_MS,
            )
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

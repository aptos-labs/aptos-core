// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use velor_types::{
    contract_event::ContractEvent, ledger_info::LedgerInfoWithSignatures, transaction::Transaction,
};
use async_trait::async_trait;
use futures::{
    channel::{mpsc, oneshot},
    stream::FusedStream,
    SinkExt, Stream,
};
use serde::{Deserialize, Serialize};
use std::{
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};
use thiserror::Error;
use tokio::time::timeout;

#[derive(Clone, Debug, Deserialize, Error, PartialEq, Eq, Serialize)]
pub enum Error {
    #[error("Notification failed: {0}")]
    NotificationError(String),
    #[error("Hit the timeout waiting for state sync to respond to the notification!")]
    TimeoutWaitingForStateSync,
    #[error("Unexpected error encountered: {0}")]
    UnexpectedErrorEncountered(String),
}

/// The interface between state sync and consensus, or consensus observer.
/// This allows callers to send notifications to state sync.
#[async_trait]
pub trait ConsensusNotificationSender: Send + Sync {
    /// Notify state sync of newly committed transactions and subscribable events.
    async fn notify_new_commit(
        &self,
        transactions: Vec<Transaction>,
        subscribable_events: Vec<ContractEvent>,
    ) -> Result<(), Error>;

    /// Notifies state sync to synchronize storage for at least the specified duration,
    /// and returns the latest synced ledger info. Note that state sync may synchronize
    /// for much longer than the specified duration, e.g., if the node is very far behind.
    async fn sync_for_duration(
        &self,
        duration: Duration,
    ) -> Result<LedgerInfoWithSignatures, Error>;

    /// Notify state sync to synchronize storage to the specified target.
    async fn sync_to_target(&self, target: LedgerInfoWithSignatures) -> Result<(), Error>;
}

/// This method returns a (ConsensusNotifier, ConsensusNotificationListener) pair that
/// can be used to allow consensus, or consensus observer, to communicate with state sync.
pub fn new_consensus_notifier_listener_pair(
    timeout_ms: u64,
) -> (ConsensusNotifier, ConsensusNotificationListener) {
    let (notification_sender, notification_receiver) = mpsc::unbounded();

    let consensus_notifier = ConsensusNotifier::new(notification_sender, timeout_ms);
    let consensus_listener = ConsensusNotificationListener::new(notification_receiver);

    (consensus_notifier, consensus_listener)
}

/// The component responsible for sending notifications and requests to state sync
#[derive(Clone, Debug)]
pub struct ConsensusNotifier {
    notification_sender: mpsc::UnboundedSender<ConsensusNotification>,

    /// Timeout for state sync to respond when handling a commit notification
    commit_timeout_ms: u64,
}

impl ConsensusNotifier {
    fn new(
        notification_sender: mpsc::UnboundedSender<ConsensusNotification>,
        commit_timeout_ms: u64,
    ) -> Self {
        ConsensusNotifier {
            notification_sender,
            commit_timeout_ms,
        }
    }
}

#[async_trait]
impl ConsensusNotificationSender for ConsensusNotifier {
    async fn notify_new_commit(
        &self,
        transactions: Vec<Transaction>,
        subscribable_events: Vec<ContractEvent>,
    ) -> Result<(), Error> {
        // Only send a notification if transactions have been committed
        if transactions.is_empty() {
            return Ok(());
        }

        // Create a consensus commit notification
        let (notification, callback_receiver) =
            ConsensusCommitNotification::new(transactions, subscribable_events);
        let commit_notification = ConsensusNotification::NotifyCommit(notification);

        // Send the notification to state sync
        if let Err(error) = self
            .notification_sender
            .clone()
            .send(commit_notification)
            .await
        {
            return Err(Error::NotificationError(format!(
                "Failed to notify state sync of committed transactions! Error: {:?}",
                error
            )));
        }

        // Handle any responses or a timeout
        if let Ok(response) = timeout(
            Duration::from_millis(self.commit_timeout_ms),
            callback_receiver,
        )
        .await
        {
            match response {
                Ok(consensus_notification_response) => consensus_notification_response.get_result(),
                Err(error) => Err(Error::UnexpectedErrorEncountered(format!(
                    "Consensus commit notification failure: {:?}",
                    error
                ))),
            }
        } else {
            Err(Error::TimeoutWaitingForStateSync)
        }
    }

    async fn sync_for_duration(
        &self,
        duration: Duration,
    ) -> Result<LedgerInfoWithSignatures, Error> {
        // Create a consensus sync duration notification
        let (notification, callback_receiver) = ConsensusSyncDurationNotification::new(duration);
        let sync_duration_notification = ConsensusNotification::SyncForDuration(notification);

        // Send the notification to state sync
        if let Err(error) = self
            .notification_sender
            .clone()
            .send(sync_duration_notification)
            .await
        {
            return Err(Error::NotificationError(format!(
                "Failed to notify state sync of sync duration! Error: {:?}",
                error
            )));
        }

        // Process the response
        match callback_receiver.await {
            Ok(response) => match response.get_result() {
                Ok(_) => response.get_latest_synced_ledger_info().ok_or_else(|| {
                    Error::UnexpectedErrorEncountered(
                        "Sync for duration returned an empty latest synced ledger info!".into(),
                    )
                }),
                Err(error) => Err(Error::UnexpectedErrorEncountered(format!(
                    "Sync for duration returned an error: {:?}",
                    error
                ))),
            },
            Err(error) => Err(Error::UnexpectedErrorEncountered(format!(
                "Sync for duration failure: {:?}",
                error
            ))),
        }
    }

    async fn sync_to_target(&self, target: LedgerInfoWithSignatures) -> Result<(), Error> {
        // Create a consensus sync target notification
        let (notification, callback_receiver) = ConsensusSyncTargetNotification::new(target);
        let sync_target_notification = ConsensusNotification::SyncToTarget(notification);

        // Send the notification to state sync
        if let Err(error) = self
            .notification_sender
            .clone()
            .send(sync_target_notification)
            .await
        {
            return Err(Error::NotificationError(format!(
                "Failed to notify state sync of sync target! Error: {:?}",
                error
            )));
        }

        // Process the response
        match callback_receiver.await {
            Ok(response) => response.get_result(),
            Err(error) => Err(Error::UnexpectedErrorEncountered(format!(
                "Sync to target failure: {:?}",
                error
            ))),
        }
    }
}

/// The component responsible for handling consensus or consensus observer notifications
#[derive(Debug)]
pub struct ConsensusNotificationListener {
    notification_receiver: mpsc::UnboundedReceiver<ConsensusNotification>,
}

impl ConsensusNotificationListener {
    fn new(notification_receiver: mpsc::UnboundedReceiver<ConsensusNotification>) -> Self {
        ConsensusNotificationListener {
            notification_receiver,
        }
    }

    /// Sends the specified result to the given callback
    fn send_result_to_callback(
        &self,
        callback: oneshot::Sender<ConsensusNotificationResponse>,
        result: Result<(), Error>,
    ) -> Result<(), Error> {
        callback
            .send(ConsensusNotificationResponse::new(result))
            .map_err(|error| Error::UnexpectedErrorEncountered(format!("{:?}", error)))
    }

    /// Respond to the commit notification
    pub fn respond_to_commit_notification(
        &self,
        consensus_commit_notification: ConsensusCommitNotification,
        result: Result<(), Error>,
    ) -> Result<(), Error> {
        self.send_result_to_callback(consensus_commit_notification.callback, result)
    }

    /// Respond to the sync duration notification
    pub fn respond_to_sync_duration_notification(
        &self,
        sync_duration_notification: ConsensusSyncDurationNotification,
        result: Result<(), Error>,
        latest_synced_ledger_info: Option<LedgerInfoWithSignatures>,
    ) -> Result<(), Error> {
        // Create a new response with the result and latest synced ledger info
        let response =
            ConsensusNotificationResponse::new_with_ledger_info(result, latest_synced_ledger_info);

        // Send the response to the callback
        sync_duration_notification
            .callback
            .send(response)
            .map_err(|error| Error::UnexpectedErrorEncountered(format!("{:?}", error)))
    }

    /// Respond to the sync target notification
    pub fn respond_to_sync_target_notification(
        &self,
        sync_target_notification: ConsensusSyncTargetNotification,
        result: Result<(), Error>,
    ) -> Result<(), Error> {
        self.send_result_to_callback(sync_target_notification.callback, result)
    }
}

impl Stream for ConsensusNotificationListener {
    type Item = ConsensusNotification;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.get_mut().notification_receiver).poll_next(cx)
    }
}

impl FusedStream for ConsensusNotificationListener {
    fn is_terminated(&self) -> bool {
        self.notification_receiver.is_terminated()
    }
}

#[derive(Debug)]
pub enum ConsensusNotification {
    NotifyCommit(ConsensusCommitNotification),
    SyncForDuration(ConsensusSyncDurationNotification),
    SyncToTarget(ConsensusSyncTargetNotification),
}

/// A commit notification to notify state sync of new commits
#[derive(Debug)]
pub struct ConsensusCommitNotification {
    transactions: Vec<Transaction>,
    subscribable_events: Vec<ContractEvent>,
    callback: oneshot::Sender<ConsensusNotificationResponse>,
}

impl ConsensusCommitNotification {
    pub fn new(
        transactions: Vec<Transaction>,
        subscribable_events: Vec<ContractEvent>,
    ) -> (Self, oneshot::Receiver<ConsensusNotificationResponse>) {
        let (callback, callback_receiver) = oneshot::channel();
        let commit_notification = ConsensusCommitNotification {
            transactions,
            subscribable_events,
            callback,
        };

        (commit_notification, callback_receiver)
    }

    /// Returns a reference to the transactions
    pub fn get_transactions(&self) -> &Vec<Transaction> {
        &self.transactions
    }

    /// Returns a reference to the subscribable events
    pub fn get_subscribable_events(&self) -> &Vec<ContractEvent> {
        &self.subscribable_events
    }
}

/// The response returned by state sync for a consensus or consensus observer notification
#[derive(Debug)]
pub struct ConsensusNotificationResponse {
    result: Result<(), Error>,
    latest_synced_ledger_info: Option<LedgerInfoWithSignatures>,
}

impl ConsensusNotificationResponse {
    pub fn new(result: Result<(), Error>) -> Self {
        Self::new_with_ledger_info(result, None)
    }

    /// Returns a new response with the given result and latest synced ledger info
    pub fn new_with_ledger_info(
        result: Result<(), Error>,
        latest_synced_ledger_info: Option<LedgerInfoWithSignatures>,
    ) -> Self {
        Self {
            result,
            latest_synced_ledger_info,
        }
    }

    /// Returns a copy of the result
    pub fn get_result(&self) -> Result<(), Error> {
        self.result.clone()
    }

    /// Returns a copy of the latest synced ledger info
    pub fn get_latest_synced_ledger_info(&self) -> Option<LedgerInfoWithSignatures> {
        self.latest_synced_ledger_info.clone()
    }
}

/// A notification for state sync to synchronize for the specified duration
#[derive(Debug)]
pub struct ConsensusSyncDurationNotification {
    duration: Duration,
    callback: oneshot::Sender<ConsensusNotificationResponse>,
}

impl ConsensusSyncDurationNotification {
    pub fn new(duration: Duration) -> (Self, oneshot::Receiver<ConsensusNotificationResponse>) {
        let (callback, callback_receiver) = oneshot::channel();
        let notification = ConsensusSyncDurationNotification { duration, callback };

        (notification, callback_receiver)
    }

    /// Returns the duration of the notification
    pub fn get_duration(&self) -> Duration {
        self.duration
    }
}

/// A notification for state sync to synchronize to the given target
#[derive(Debug)]
pub struct ConsensusSyncTargetNotification {
    target: LedgerInfoWithSignatures,
    callback: oneshot::Sender<ConsensusNotificationResponse>,
}

impl ConsensusSyncTargetNotification {
    pub fn new(
        target: LedgerInfoWithSignatures,
    ) -> (Self, oneshot::Receiver<ConsensusNotificationResponse>) {
        let (callback, callback_receiver) = oneshot::channel();
        let notification = ConsensusSyncTargetNotification { target, callback };

        (notification, callback_receiver)
    }

    /// Returns a reference to the target of the notification
    pub fn get_target(&self) -> &LedgerInfoWithSignatures {
        &self.target
    }
}

#[cfg(test)]
mod tests {
    use crate::{ConsensusNotification, ConsensusNotificationSender, Error};
    use velor_crypto::{ed25519::Ed25519PrivateKey, HashValue, PrivateKey, SigningKey, Uniform};
    use velor_types::{
        account_address::AccountAddress,
        aggregate_signature::AggregateSignature,
        block_info::BlockInfo,
        chain_id::ChainId,
        contract_event::ContractEvent,
        event::EventKey,
        ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
        transaction::{RawTransaction, Script, SignedTransaction, Transaction, TransactionPayload},
    };
    use claims::{assert_err, assert_matches, assert_ok};
    use futures::{executor::block_on, FutureExt, StreamExt};
    use move_core_types::language_storage::TypeTag;
    use std::time::Duration;
    use tokio::runtime::Runtime;

    const CONSENSUS_NOTIFICATION_TIMEOUT: u64 = 1000;

    #[test]
    fn test_commit_state_sync_not_listening() {
        // Create a runtime and consensus notifier
        let runtime = create_runtime();
        let _enter = runtime.enter();
        let (consensus_notifier, mut consensus_listener) =
            crate::new_consensus_notifier_listener_pair(CONSENSUS_NOTIFICATION_TIMEOUT);

        // Send a commit notification and expect a timeout (no listener)
        let notify_result =
            block_on(consensus_notifier.notify_new_commit(vec![create_user_transaction()], vec![]));
        assert_matches!(notify_result, Err(Error::TimeoutWaitingForStateSync));

        // Drop the receiver and try again
        consensus_listener.notification_receiver.close();
        let notify_result =
            block_on(consensus_notifier.notify_new_commit(vec![create_user_transaction()], vec![]));
        assert_matches!(notify_result, Err(Error::NotificationError(_)));
    }

    #[test]
    fn test_commit_no_transactions() {
        // Create a runtime and consensus notifier
        let runtime = create_runtime();
        let _enter = runtime.enter();
        let (consensus_notifier, _consensus_listener) =
            crate::new_consensus_notifier_listener_pair(CONSENSUS_NOTIFICATION_TIMEOUT);

        // Send an empty commit notification
        let notify_result = block_on(consensus_notifier.notify_new_commit(vec![], vec![]));
        assert_ok!(notify_result);
    }

    #[test]
    fn test_consensus_notification_arrives() {
        // Create a runtime and consensus notifier
        let runtime = create_runtime();
        let _enter = runtime.enter();
        let (consensus_notifier, mut consensus_listener) =
            crate::new_consensus_notifier_listener_pair(CONSENSUS_NOTIFICATION_TIMEOUT);

        // Send a commit notification
        let transactions = vec![create_user_transaction()];
        let subscribable_events = vec![create_contract_event()];
        let _ = block_on(
            consensus_notifier.notify_new_commit(transactions.clone(), subscribable_events.clone()),
        );

        // Verify the notification arrives at the receiver
        match consensus_listener.select_next_some().now_or_never() {
            Some(consensus_notification) => match consensus_notification {
                ConsensusNotification::NotifyCommit(commit_notification) => {
                    assert_eq!(transactions, commit_notification.get_transactions().clone());
                    assert_eq!(
                        subscribable_events,
                        commit_notification.get_subscribable_events().clone()
                    );
                },
                result => panic!(
                    "Expected consensus commit notification but got: {:?}",
                    result
                ),
            },
            result => panic!("Expected consensus notification but got: {:?}", result),
        };

        // Send a sync target notification
        let notifier = consensus_notifier.clone();
        let _thread = std::thread::spawn(move || {
            let _result = block_on(notifier.sync_to_target(create_ledger_info()));
        });

        // Give the thread enough time to spawn and send the notification
        std::thread::sleep(Duration::from_millis(1000));

        // Verify the notification arrives at the receiver
        match consensus_listener.select_next_some().now_or_never() {
            Some(consensus_notification) => match consensus_notification {
                ConsensusNotification::SyncToTarget(sync_notification) => {
                    assert_eq!(create_ledger_info(), sync_notification.get_target().clone());
                },
                result => panic!(
                    "Expected consensus sync target notification but got: {:?}",
                    result
                ),
            },
            result => panic!("Expected consensus notification but got: {:?}", result),
        };

        // Send a sync duration notification
        let _thread = std::thread::spawn(move || {
            let _result = block_on(consensus_notifier.sync_for_duration(Duration::from_secs(10)));
        });

        // Give the thread enough time to spawn and send the notification
        std::thread::sleep(Duration::from_millis(1000));

        // Verify the notification arrives at the receiver
        match consensus_listener.select_next_some().now_or_never() {
            Some(consensus_notification) => match consensus_notification {
                ConsensusNotification::SyncForDuration(sync_notification) => {
                    assert_eq!(Duration::from_secs(10), sync_notification.duration);
                },
                result => panic!(
                    "Expected consensus sync duration notification but got: {:?}",
                    result
                ),
            },
            result => panic!("Expected consensus notification but got: {:?}", result),
        };
    }

    #[test]
    fn test_consensus_notification_responses() {
        // Create a runtime and consensus notifier
        let runtime = create_runtime();
        let _enter = runtime.enter();
        let (consensus_notifier, mut consensus_listener) =
            crate::new_consensus_notifier_listener_pair(CONSENSUS_NOTIFICATION_TIMEOUT);

        // Spawn a new thread to handle any messages on the receiver
        let _handler = std::thread::spawn(move || loop {
            match consensus_listener.select_next_some().now_or_never() {
                Some(ConsensusNotification::NotifyCommit(commit_notification)) => {
                    let _result = consensus_listener
                        .respond_to_commit_notification(commit_notification, Ok(()));
                },
                Some(ConsensusNotification::SyncToTarget(sync_notification)) => {
                    let _result = consensus_listener.respond_to_sync_target_notification(
                        sync_notification,
                        Err(Error::UnexpectedErrorEncountered(
                            "Oops! Sync to target failed!".into(),
                        )),
                    );
                },
                Some(ConsensusNotification::SyncForDuration(sync_notification)) => {
                    let _result = consensus_listener.respond_to_sync_duration_notification(
                        sync_notification,
                        Err(Error::UnexpectedErrorEncountered(
                            "Oops! Sync for duration failed!".into(),
                        )),
                        None,
                    );
                },
                _ => { /* Do nothing */ },
            }
        });

        // Send a commit notification and verify a successful response
        let notify_result =
            block_on(consensus_notifier.notify_new_commit(vec![create_user_transaction()], vec![]));
        assert_ok!(notify_result);

        // Send a sync target notification and verify an error response
        let notify_result = block_on(consensus_notifier.sync_to_target(create_ledger_info()));
        assert_err!(notify_result);

        // Send a sync duration notification and verify an error response
        let notify_result = block_on(consensus_notifier.sync_for_duration(Duration::from_secs(10)));
        assert_err!(notify_result);
    }

    fn create_user_transaction() -> Transaction {
        let private_key = Ed25519PrivateKey::generate_for_testing();
        let public_key = private_key.public_key();

        let transaction_payload = TransactionPayload::Script(Script::new(vec![], vec![], vec![]));
        let raw_transaction = RawTransaction::new(
            AccountAddress::random(),
            0,
            transaction_payload,
            0,
            0,
            0,
            ChainId::new(10),
        );
        let signed_transaction = SignedTransaction::new(
            raw_transaction.clone(),
            public_key,
            private_key.sign(&raw_transaction).unwrap(),
        );

        Transaction::UserTransaction(signed_transaction)
    }

    fn create_contract_event() -> ContractEvent {
        ContractEvent::new_v1(
            EventKey::new(0, AccountAddress::random()),
            0,
            TypeTag::Bool,
            b"some event bytes".to_vec(),
        )
        .unwrap()
    }

    fn create_ledger_info() -> LedgerInfoWithSignatures {
        LedgerInfoWithSignatures::new(
            LedgerInfo::new(BlockInfo::empty(), HashValue::zero()),
            AggregateSignature::empty(),
        )
    }

    fn create_runtime() -> Runtime {
        velor_runtimes::spawn_named_runtime("test".into(), None)
    }
}

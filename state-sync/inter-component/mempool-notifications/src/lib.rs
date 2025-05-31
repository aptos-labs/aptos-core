// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use aptos_types::{
    account_address::AccountAddress,
    transaction::{
        use_case::{UseCaseAwareTransaction, UseCaseKey},
        ReplayProtector, Transaction,
    },
};
use async_trait::async_trait;
use futures::{channel::mpsc, stream::FusedStream, SinkExt, Stream};
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    pin::Pin,
    task::{Context, Poll},
};
use thiserror::Error;

#[derive(Clone, Debug, Deserialize, Error, PartialEq, Eq, Serialize)]
pub enum Error {
    #[error("Commit notification failed: {0}")]
    CommitNotificationError(String),
    #[error("Hit the timeout waiting for mempool to respond to the notification!")]
    TimeoutWaitingForMempool,
    #[error("Unexpected error encountered: {0}")]
    UnexpectedErrorEncountered(String),
}

/// The interface between state sync and mempool, allowing state sync to notify
/// mempool of events (e.g., newly committed transactions).
#[async_trait]
pub trait MempoolNotificationSender: Send + Clone + Sync + 'static {
    /// Notify mempool of the newly committed transactions at the specified block timestamp.
    async fn notify_new_commit(
        &self,
        committed_transactions: Vec<Transaction>,
        block_timestamp_usecs: u64,
    ) -> Result<(), Error>;
}

/// This method returns a (MempoolNotifier, MempoolNotificationListener) pair that can be used
/// to allow state sync and mempool to communicate.
///
/// Note: state sync should take the notifier and mempool should take the listener.
pub fn new_mempool_notifier_listener_pair(
    max_pending_mempool_notifications: u64,
) -> (MempoolNotifier, MempoolNotificationListener) {
    let (notification_sender, notification_receiver) =
        mpsc::channel(max_pending_mempool_notifications as usize);

    let mempool_notifier = MempoolNotifier::new(notification_sender);
    let mempool_listener = MempoolNotificationListener::new(notification_receiver);

    (mempool_notifier, mempool_listener)
}

/// The state sync component responsible for notifying mempool.
#[derive(Clone, Debug)]
pub struct MempoolNotifier {
    notification_sender: mpsc::Sender<MempoolCommitNotification>,
}

impl MempoolNotifier {
    fn new(notification_sender: mpsc::Sender<MempoolCommitNotification>) -> Self {
        Self {
            notification_sender,
        }
    }
}

#[async_trait]
impl MempoolNotificationSender for MempoolNotifier {
    async fn notify_new_commit(
        &self,
        transactions: Vec<Transaction>,
        block_timestamp_usecs: u64,
    ) -> Result<(), Error> {
        // Get only user transactions from committed transactions
        let user_transactions: Vec<CommittedTransaction> = transactions
            .iter()
            .filter_map(|transaction| {
                transaction
                    .try_as_signed_user_txn()
                    .map(|signed_txn| CommittedTransaction {
                        sender: signed_txn.sender(),
                        replay_protector: signed_txn.replay_protector(),
                        use_case: signed_txn.parse_use_case(),
                    })
            })
            .collect();

        // Mempool needs to be notified about all transactions (user and non-user transactions).
        // See https://github.com/aptos-labs/aptos-core/issues/1882 for more details.
        let commit_notification = MempoolCommitNotification {
            transactions: user_transactions,
            block_timestamp_usecs,
        };

        // Send the notification to mempool
        if let Err(error) = self
            .notification_sender
            .clone()
            .send(commit_notification)
            .await
        {
            return Err(Error::CommitNotificationError(format!(
                "Failed to notify mempool of committed transactions! Error: {:?}",
                error
            )));
        }

        Ok(())
    }
}

/// The mempool component responsible for responding to state sync notifications.
#[derive(Debug)]
pub struct MempoolNotificationListener {
    notification_receiver: mpsc::Receiver<MempoolCommitNotification>,
}

impl MempoolNotificationListener {
    fn new(notification_receiver: mpsc::Receiver<MempoolCommitNotification>) -> Self {
        MempoolNotificationListener {
            notification_receiver,
        }
    }
}

impl Stream for MempoolNotificationListener {
    type Item = MempoolCommitNotification;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.get_mut().notification_receiver).poll_next(cx)
    }
}

impl FusedStream for MempoolNotificationListener {
    fn is_terminated(&self) -> bool {
        self.notification_receiver.is_terminated()
    }
}

/// A notification for newly committed transactions sent by state sync to mempool.
#[derive(Debug)]
pub struct MempoolCommitNotification {
    pub transactions: Vec<CommittedTransaction>,
    pub block_timestamp_usecs: u64, // The timestamp of the committed block.
}

impl fmt::Display for MempoolCommitNotification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MempoolCommitNotification [block_timestamp_usecs: {}, txns: {:?}]",
            self.block_timestamp_usecs, self.transactions
        )
    }
}

/// A successfully executed and committed user transaction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommittedTransaction {
    pub sender: AccountAddress,
    pub replay_protector: ReplayProtector,
    pub use_case: UseCaseKey,
}

impl fmt::Display for CommittedTransaction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}:{:?}",
            self.sender, self.replay_protector, self.use_case
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::{CommittedTransaction, Error, MempoolNotificationSender};
    use aptos_crypto::{ed25519::Ed25519PrivateKey, HashValue, PrivateKey, SigningKey, Uniform};
    use aptos_types::{
        account_address::AccountAddress,
        block_metadata::BlockMetadata,
        chain_id::ChainId,
        transaction::{
            use_case::UseCaseAwareTransaction, ChangeSet, RawTransaction, Script,
            SignedTransaction, Transaction, TransactionPayload, WriteSetPayload,
        },
        write_set::WriteSetMut,
    };
    use claims::{assert_matches, assert_ok};
    use futures::{FutureExt, StreamExt};
    use std::time::Duration;
    use tokio::time::timeout;

    #[tokio::test]
    async fn test_mempool_not_listening() {
        // Create runtime and mempool notifier
        let (mempool_notifier, mut mempool_listener) =
            crate::new_mempool_notifier_listener_pair(100);

        // Send a notification and expect no failures
        let notify_result = mempool_notifier
            .notify_new_commit(vec![create_user_transaction()], 0)
            .await;
        assert_ok!(notify_result);

        // Drop the receiver and try again (this time we expect a failure)
        mempool_listener.notification_receiver.close();
        let notify_result = mempool_notifier
            .notify_new_commit(vec![create_user_transaction()], 0)
            .await;
        assert_matches!(notify_result, Err(Error::CommitNotificationError(_)));
    }

    #[tokio::test]
    async fn test_mempool_channel_blocked() {
        // Create runtime and mempool notifier (with a max of 1 pending notifications)
        let (mempool_notifier, _mempool_listener) = crate::new_mempool_notifier_listener_pair(1);

        // Send a notification and expect no failures
        let notify_result = mempool_notifier
            .notify_new_commit(vec![create_user_transaction()], 0)
            .await;
        assert_ok!(notify_result);

        // Send another notification (which should block!)
        let result = timeout(
            Duration::from_secs(5),
            mempool_notifier.notify_new_commit(vec![create_user_transaction()], 0),
        )
        .await;

        // Verify the channel is blocked
        if let Ok(result) = result {
            panic!(
                "We expected the channel to be blocked, but it's not? Result: {:?}",
                result
            );
        }
    }

    #[tokio::test]
    async fn test_no_transaction_filtering() {
        // Create runtime and mempool notifier
        let (mempool_notifier, _mempool_listener) = crate::new_mempool_notifier_listener_pair(100);

        // Create several non-user transactions
        let mut transactions = vec![];
        for _ in 0..5 {
            transactions.push(create_block_metadata_transaction());
            transactions.push(create_genesis_transaction());
        }

        // Send a notification and verify no failures
        let notify_result = mempool_notifier
            .notify_new_commit(transactions.clone(), 0)
            .await;
        assert_ok!(notify_result);

        // Send another notification with a single user transaction now included
        transactions.push(create_user_transaction());
        let notify_result = mempool_notifier.notify_new_commit(transactions, 0).await;
        assert_ok!(notify_result);
    }

    #[tokio::test]
    async fn test_commit_notification_arrives() {
        // Create runtime and mempool notifier
        let (mempool_notifier, mut mempool_listener) =
            crate::new_mempool_notifier_listener_pair(100);

        // Send a notification
        let user_transaction = create_user_transaction();
        let transactions = vec![user_transaction.clone()];
        let block_timestamp_usecs = 101;
        let _ = mempool_notifier
            .notify_new_commit(transactions, block_timestamp_usecs)
            .await;

        // Verify the notification arrives at the receiver
        match mempool_listener.select_next_some().now_or_never() {
            Some(mempool_commit_notification) => match user_transaction {
                Transaction::UserTransaction(signed_transaction) => {
                    assert_eq!(mempool_commit_notification.transactions, vec![
                        CommittedTransaction {
                            sender: signed_transaction.sender(),
                            replay_protector: signed_transaction.replay_protector(),
                            use_case: signed_transaction.parse_use_case(),
                        }
                    ]);
                    assert_eq!(
                        mempool_commit_notification.block_timestamp_usecs,
                        block_timestamp_usecs
                    );
                },
                result => panic!("Expected user transaction but got: {:?}", result),
            },
            result => panic!("Expected mempool commit notification but got: {:?}", result),
        };
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

    fn create_block_metadata_transaction() -> Transaction {
        Transaction::BlockMetadata(BlockMetadata::new(
            HashValue::new([0; HashValue::LENGTH]),
            0,
            300000001,
            AccountAddress::random(),
            vec![0],
            vec![],
            1,
        ))
    }

    fn create_genesis_transaction() -> Transaction {
        Transaction::GenesisTransaction(WriteSetPayload::Direct(ChangeSet::new(
            WriteSetMut::new(vec![])
                .freeze()
                .expect("freeze cannot fail"),
            vec![],
        )))
    }
}

// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use velor_channels::{self, velor_channel, message_queues::QueueStyle};
use async_trait::async_trait;
use futures::{stream::FusedStream, Stream};
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    pin::Pin,
    task::{Context, Poll},
};
use thiserror::Error;

// Note: we limit the queue depth to 1 because it doesn't make sense for the storage service
// to execute for every notification (because it reads the latest version in the DB). Thus,
// if there are X pending notifications, the first one will refresh using the latest DB and
// the next X-1 will execute with an unchanged DB (thus, becoming a no-op and wasting the CPU).
const STORAGE_SERVICE_NOTIFICATION_CHANNEL_SIZE: usize = 1;

#[derive(Clone, Debug, Deserialize, Error, PartialEq, Eq, Serialize)]
pub enum Error {
    #[error("Commit notification failed: {0}")]
    CommitNotificationError(String),
}

/// The interface between the state sync driver and the storage service, allowing the driver
/// to notify the storage service of events (e.g., newly committed transactions).
#[async_trait]
pub trait StorageServiceNotificationSender: Send + Clone + Sync + 'static {
    /// Notify the storage service of newly committed transactions
    /// at the specified version.
    async fn notify_new_commit(&self, highest_synced_version: u64) -> Result<(), Error>;
}

/// This method returns a (StorageServiceNotifier, StorageServiceNotificationListener) pair
/// that can be used to allow state sync and the storage service to communicate.
///
/// Note: the driver should take the notifier and the storage service should take the listener.
pub fn new_storage_service_notifier_listener_pair(
) -> (StorageServiceNotifier, StorageServiceNotificationListener) {
    // Create a dedicated channel for notifications
    let (notification_sender, notification_receiver) = velor_channel::new(
        QueueStyle::LIFO,
        STORAGE_SERVICE_NOTIFICATION_CHANNEL_SIZE,
        None,
    );

    // Create a notification sender and listener
    let storage_service_notifier = StorageServiceNotifier::new(notification_sender);
    let storage_service_listener = StorageServiceNotificationListener::new(notification_receiver);

    (storage_service_notifier, storage_service_listener)
}

/// The state sync driver component responsible for notifying the storage service
#[derive(Clone, Debug)]
pub struct StorageServiceNotifier {
    notification_sender: velor_channel::Sender<(), StorageServiceCommitNotification>,
}

impl StorageServiceNotifier {
    fn new(
        notification_sender: velor_channel::Sender<(), StorageServiceCommitNotification>,
    ) -> Self {
        Self {
            notification_sender,
        }
    }
}

#[async_trait]
impl StorageServiceNotificationSender for StorageServiceNotifier {
    async fn notify_new_commit(&self, highest_synced_version: u64) -> Result<(), Error> {
        // Create a new commit notification
        let commit_notification = StorageServiceCommitNotification {
            highest_synced_version,
        };

        // Send the notification to the storage service
        if let Err(error) = self
            .notification_sender
            .clone()
            .push((), commit_notification)
        {
            return Err(Error::CommitNotificationError(format!(
                "Failed to notify the storage service of committed transactions! Error: {:?}",
                error
            )));
        }

        Ok(())
    }
}

/// The storage service component responsible for handling state sync notifications
#[derive(Debug)]
pub struct StorageServiceNotificationListener {
    notification_receiver: velor_channel::Receiver<(), StorageServiceCommitNotification>,
}

impl StorageServiceNotificationListener {
    fn new(
        notification_receiver: velor_channel::Receiver<(), StorageServiceCommitNotification>,
    ) -> Self {
        StorageServiceNotificationListener {
            notification_receiver,
        }
    }
}

impl Stream for StorageServiceNotificationListener {
    type Item = StorageServiceCommitNotification;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.get_mut().notification_receiver).poll_next(cx)
    }
}

impl FusedStream for StorageServiceNotificationListener {
    fn is_terminated(&self) -> bool {
        self.notification_receiver.is_terminated()
    }
}

/// A notification for newly committed transactions sent
/// by the state sync driver to the storage service.
#[derive(Debug)]
pub struct StorageServiceCommitNotification {
    pub highest_synced_version: u64, // The new highest synced version
}

impl fmt::Display for StorageServiceCommitNotification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "StorageServiceCommitNotification [highest_synced_version: {}]",
            self.highest_synced_version,
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        new_storage_service_notifier_listener_pair, Error, StorageServiceNotificationSender,
    };
    use claims::assert_matches;
    use futures::StreamExt;

    #[tokio::test]
    async fn test_storage_service_notification() {
        // Create a storage service notifier and listener pair
        let (storage_service_notifier, mut storage_service_listener) =
            new_storage_service_notifier_listener_pair();

        // Notify the storage service of a new commit
        let highest_synced_version = 500;
        storage_service_notifier
            .notify_new_commit(highest_synced_version)
            .await
            .unwrap();

        // Verify the storage service received the notification
        let commit_notification = storage_service_listener.next().await.unwrap();
        assert_eq!(
            commit_notification.highest_synced_version,
            highest_synced_version
        );

        // Drop the receiver, send a notification and verify an error is returned
        drop(storage_service_listener);
        let error = storage_service_notifier
            .notify_new_commit(highest_synced_version)
            .await
            .unwrap_err();
        assert_matches!(error, Error::CommitNotificationError(_));
    }
}

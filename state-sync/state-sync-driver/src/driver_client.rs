// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::error::Error;
use futures::{
    channel::{mpsc, oneshot},
    future::Future,
    stream::FusedStream,
    SinkExt, Stream,
};
use std::{
    pin::Pin,
    task::{Context, Poll},
};

/// Notifications that can be sent to the state sync driver
pub enum DriverNotification {
    NotifyOnceBootstrapped(oneshot::Sender<Result<(), Error>>),
}

/// A client for sending notifications to the state sync driver
pub struct DriverClient {
    notification_sender: mpsc::UnboundedSender<DriverNotification>,
}

impl DriverClient {
    pub fn new(notification_sender: mpsc::UnboundedSender<DriverNotification>) -> Self {
        Self {
            notification_sender,
        }
    }

    /// Notifies the caller once the driver has successfully bootstrapped the node
    pub fn notify_once_bootstrapped(&self) -> impl Future<Output = Result<(), Error>> + use<> {
        let mut notification_sender = self.notification_sender.clone();
        let (callback_sender, callback_receiver) = oneshot::channel();

        async move {
            notification_sender
                .send(DriverNotification::NotifyOnceBootstrapped(callback_sender))
                .await?;
            callback_receiver.await?
        }
    }
}

/// A simple listener for client notifications
pub struct ClientNotificationListener {
    // The listener for notifications from clients
    client_notifications: mpsc::UnboundedReceiver<DriverNotification>,
}

impl ClientNotificationListener {
    pub fn new(client_notifications: mpsc::UnboundedReceiver<DriverNotification>) -> Self {
        Self {
            client_notifications,
        }
    }
}

impl Stream for ClientNotificationListener {
    type Item = DriverNotification;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.get_mut().client_notifications).poll_next(cx)
    }
}

impl FusedStream for ClientNotificationListener {
    fn is_terminated(&self) -> bool {
        self.client_notifications.is_terminated()
    }
}

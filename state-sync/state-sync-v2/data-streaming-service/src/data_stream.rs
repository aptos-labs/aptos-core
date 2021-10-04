// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use futures::{stream::FusedStream, Stream};
use std::{
    pin::Pin,
    task::{Context, Poll},
};

/// A unique ID used to identify each notification.
pub type NotificationId = u64;

/// A single data notification.
/// TODO(joshlind): complete me!
#[derive(Debug)]
pub struct DataNotification {
    pub notification_id: NotificationId,
}

/// Allows listening to data streams (i.e., streams of data notifications).
///
/// Note: when the data stream is finished (i.e., empty) a data notification
/// containing an `EndOfDataStream` payload is sent to the listener.
#[derive(Debug)]
pub struct DataStreamListener {
    notification_receiver: channel::diem_channel::Receiver<(), DataNotification>,
}

impl DataStreamListener {
    #[allow(dead_code)]
    pub fn new(
        notification_receiver: channel::diem_channel::Receiver<(), DataNotification>,
    ) -> Self {
        Self {
            notification_receiver,
        }
    }
}

impl Stream for DataStreamListener {
    type Item = DataNotification;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.get_mut().notification_receiver).poll_next(cx)
    }
}

impl FusedStream for DataStreamListener {
    fn is_terminated(&self) -> bool {
        self.notification_receiver.is_terminated()
    }
}

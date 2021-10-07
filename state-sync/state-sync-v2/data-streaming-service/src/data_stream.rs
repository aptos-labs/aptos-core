// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{data_notification::DataNotification, stream_progress_tracker::StreamProgressTracker};
use channel::{diem_channel, message_queues::QueueStyle};
use futures::{stream::FusedStream, Stream};
use std::{
    pin::Pin,
    task::{Context, Poll},
};

// Maximum channel sizes for each stream listener. If messages are not
// consumed, they will be dropped (oldest messages first). The remaining
// messages will be retrieved using FIFO ordering.
const DATA_STREAM_CHANNEL_SIZE: usize = 1000;

/// A unique ID used to identify each stream.
pub type DataStreamId = u64;

/// Each data stream holds the original data request from the client and tracks
/// the progress of the data stream to satisfy that request (e.g., the data that
/// has already been sent along the stream to the client and the in-flight
/// data requests that have been sent to the network).
///
/// Note that it is the responsibility of the data stream to send data
/// notifications along the stream in sequential order (e.g., transactions and
/// proofs must be sent with monotonically increasing versions).
#[derive(Debug)]
pub struct DataStream {
    // The fulfillment progress tracker for this data stream
    pub stream_progress_tracker: StreamProgressTracker,

    // The channel on which to send data notifications when they are ready.
    pub notification_sender: channel::diem_channel::Sender<(), DataNotification>,
}

impl DataStream {
    pub fn new(stream_progress_tracker: StreamProgressTracker) -> (Self, DataStreamListener) {
        // Create a new data stream listener
        let (notification_sender, notification_receiver) =
            diem_channel::new(QueueStyle::KLAST, DATA_STREAM_CHANNEL_SIZE, None);
        let data_stream_listener = DataStreamListener::new(notification_receiver);

        // Create a new data stream
        let data_stream = Self {
            stream_progress_tracker,
            notification_sender,
        };

        (data_stream, data_stream_listener)
    }
}

/// Allows listening to data streams (i.e., streams of data notifications).
#[derive(Debug)]
pub struct DataStreamListener {
    notification_receiver: channel::diem_channel::Receiver<(), DataNotification>,
}

impl DataStreamListener {
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

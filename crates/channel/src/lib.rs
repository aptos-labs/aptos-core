// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

//! Provides an mpsc (multi-producer single-consumer) channel wrapped in an
//! [`IntGauge`] that counts the number of currently
//! queued items. While there is only one [`Receiver`], there can be
//! many [`Sender`]s, which are also cheap to clone.
//!
//! This channel differs from our other channel implementation, [`velor_channel`],
//! in that it is just a single queue (vs. different queues for different keys)
//! with backpressure (senders will block if the queue is full instead of evicting
//! another item in the queue) that only implements FIFO (vs. LIFO or KLAST).

use velor_metrics_core::IntGauge;
use futures::{
    channel::mpsc,
    sink::Sink,
    stream::{FusedStream, Stream},
    task::{Context, Poll},
};
use std::pin::Pin;

#[cfg(test)]
mod test;

pub mod velor_channel;
#[cfg(test)]
mod velor_channel_test;

pub mod message_queues;
#[cfg(test)]
mod message_queues_test;

/// An [`mpsc::Sender`] with an [`IntGauge`]
/// counting the number of currently queued items.
pub struct Sender<T> {
    inner: mpsc::Sender<T>,
    gauge: IntGauge,
}

/// An [`mpsc::Receiver`] with an [`IntGauge`]
/// counting the number of currently queued items.
pub struct Receiver<T> {
    inner: mpsc::Receiver<T>,
    gauge: IntGauge,
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            gauge: self.gauge.clone(),
        }
    }
}

/// `Sender` implements `Sink` in the same way as `mpsc::Sender`, but it increments the
/// associated `IntGauge` when it sends a message successfully.
impl<T> Sink<T> for Sender<T> {
    type Error = mpsc::SendError;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        (self).inner.poll_ready(cx)
    }

    fn start_send(mut self: Pin<&mut Self>, msg: T) -> Result<(), Self::Error> {
        (self).inner.start_send(msg).map(|_| self.gauge.inc())
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.inner).poll_close(cx)
    }
}

impl<T> Sender<T> {
    pub fn try_send(&mut self, msg: T) -> Result<(), mpsc::SendError> {
        self.inner
            .try_send(msg)
            .map(|_| self.gauge.inc())
            .map_err(mpsc::TrySendError::into_send_error)
    }
}

impl<T> FusedStream for Receiver<T>
where
    T: std::fmt::Debug,
{
    fn is_terminated(&self) -> bool {
        self.inner.is_terminated()
    }
}

/// `Receiver` implements `Stream` in the same way as `mpsc::Stream`, but it decrements the
/// associated `IntGauge` when it gets polled successfully.
impl<T> Stream for Receiver<T> {
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let next = Pin::new(&mut self.inner).poll_next(cx);
        if let Poll::Ready(Some(_)) = next {
            self.gauge.dec();
        }
        next
    }
}

pub fn new_test<T>(size: usize) -> (Sender<T>, Receiver<T>) {
    let gauge = IntGauge::new("TEST_COUNTER", "test").unwrap();
    new(size, &gauge)
}

/// Similar to `mpsc::channel`, `new` creates a pair of `Sender` and `Receiver`
pub fn new<T>(size: usize, gauge: &IntGauge) -> (Sender<T>, Receiver<T>) {
    gauge.set(0);
    let (sender, receiver) = mpsc::channel(size);
    (
        Sender {
            inner: sender,
            gauge: gauge.clone(),
        },
        Receiver {
            inner: receiver,
            gauge: gauge.clone(),
        },
    )
}

pub fn new_unbounded_test<T>() -> (UnboundedSender<T>, UnboundedReceiver<T>) {
    let gauge = IntGauge::new("TEST_COUNTER", "test").unwrap();
    new_unbounded(&gauge)
}

pub fn new_unbounded<T>(gauge: &IntGauge) -> (UnboundedSender<T>, UnboundedReceiver<T>) {
    gauge.set(0);
    let (sender, receiver) = mpsc::unbounded();
    (
        UnboundedSender {
            inner: sender,
            gauge: gauge.clone(),
        },
        UnboundedReceiver {
            inner: receiver,
            gauge: gauge.clone(),
        },
    )
}

pub struct UnboundedSender<T> {
    inner: mpsc::UnboundedSender<T>,
    gauge: IntGauge,
}

/// An [`mpsc::UnboundedReceiver`] with an [`IntGauge`]
/// counting the number of currently queued items.
pub struct UnboundedReceiver<T> {
    inner: mpsc::UnboundedReceiver<T>,
    gauge: IntGauge,
}

impl<T> Clone for UnboundedSender<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            gauge: self.gauge.clone(),
        }
    }
}

/// `UnboundedSender` implements `Sink` in the same way as `mpsc::Sender`, but it increments the
/// associated `IntGauge` when it sends a message successfully.
impl<T> Sink<T> for UnboundedSender<T> {
    type Error = mpsc::SendError;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        (self).inner.poll_ready(cx)
    }

    fn start_send(mut self: Pin<&mut Self>, msg: T) -> Result<(), Self::Error> {
        (self).inner.start_send(msg).map(|_| self.gauge.inc())
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.inner).poll_close(cx)
    }
}

impl<T> FusedStream for UnboundedReceiver<T>
where
    T: std::fmt::Debug,
{
    fn is_terminated(&self) -> bool {
        self.inner.is_terminated()
    }
}

/// `UnboundedReceiver` implements `Stream` in the same way as `mpsc::Stream`, but it decrements the
/// associated `IntGauge` when it gets polled successfully.
impl<T> Stream for UnboundedReceiver<T> {
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let next = Pin::new(&mut self.inner).poll_next(cx);
        if let Poll::Ready(Some(_)) = next {
            self.gauge.dec();
        }
        next
    }
}

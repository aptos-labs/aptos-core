// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

//! Provides an mpsc (multi-producer single-consumer) channel wrapped in an
//! [`IntGauge`] that counts the number of currently
//! queued items. While there is only one [`Receiver`], there can be
//! many [`Sender`]s, which are also cheap to clone.
//! This is backed by a tokio::sync::mpsc channel, and provides a minimal subset
//! of its methods.
//!
//! This channel differs from our other channel implementation, [`aptos_channel`],
//! in that it is just a single queue (vs. different queues for different keys)
//! with backpressure (senders will block if the queue is full instead of evicting
//! another item in the queue) that only implements FIFO (vs. LIFO or KLAST).

use aptos_metrics_core::IntGauge;
use tokio::sync::mpsc;
/// An [`mpsc::Sender`](futures::channel::mpsc::Sender) with an [`IntGauge`]
/// counting the number of currently queued items.
pub struct Sender<T> {
    inner: mpsc::Sender<T>,
    gauge: IntGauge,
}

/// An [`mpsc::Receiver`](futures::channel::mpsc::Receiver) with an [`IntGauge`]
/// counting the number of currently queued items.
pub struct Receiver<T> {
    inner: mpsc::Receiver<T>,
    gauge: IntGauge,
}

impl<T> Sender<T> {
    pub async fn send(&self, value: T) -> Result<(), mpsc::error::SendError<T>> {
        self.inner.send(value).await.map(|_| self.gauge.inc())
    }

    pub fn try_send(&self, value: T) -> Result<(), mpsc::error::TrySendError<T>> {
        self.inner.try_send(value).map(|_| self.gauge.inc())
    }

    pub async fn send_timeout(
        &self,
        value: T,
        timeout: std::time::Duration,
    ) -> Result<(), mpsc::error::SendTimeoutError<T>> {
        self.inner
            .send_timeout(value, timeout)
            .await
            .map(|_| self.gauge.inc())
    }

    pub fn blocking_send(&self, value: T) -> Result<(), mpsc::error::SendError<T>> {
        self.inner.blocking_send(value).map(|_| self.gauge.inc())
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            gauge: self.gauge.clone(),
        }
    }
}

impl<T> Receiver<T> {
    pub async fn recv(&mut self) -> Option<T> {
        self.inner.recv().await.map(|v| {
            self.gauge.dec();
            v
        })
    }

    pub fn try_recv(&mut self) -> Result<T, mpsc::error::TryRecvError> {
        self.inner.try_recv().map(|v| {
            self.gauge.dec();
            v
        })
    }

    pub fn blocking_recv(&mut self) -> Option<T> {
        self.inner.blocking_recv().map(|v| {
            self.gauge.dec();
            v
        })
    }
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

pub fn new_test<T>(size: usize) -> (Sender<T>, Receiver<T>) {
    let gauge = IntGauge::new("TEST_COUNTER", "test").unwrap();
    new(size, &gauge)
}

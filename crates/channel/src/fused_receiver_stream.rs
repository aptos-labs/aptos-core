// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::pin::Pin;
use std::task::{Context, Poll};
use futures::Stream;
use futures::stream::FusedStream;
use tokio::sync::mpsc::Receiver;

/// Like tokio_stream::wrappers::ReceiverStream, but also FusedStream
/// A wrapper around [`tokio::sync::mpsc::Receiver`] that implements [`Stream`].
///
/// [`tokio::sync::mpsc::Receiver`]: struct@tokio::sync::mpsc::Receiver
/// [`Stream`]: trait@crate::Stream
#[derive(Debug)]
pub struct FusedReceiverStream<T> {
    inner: Receiver<T>,
    done: bool,
}

impl<T> FusedReceiverStream<T> {
    /// Create a new `FusedReceiverStream`.
    pub fn new(recv: Receiver<T>) -> Self {
        Self { inner: recv, done: false }
    }

    /// Get back the inner `Receiver`.
    pub fn into_inner(self) -> Receiver<T> {
        self.inner
    }

    /// Closes the receiving half of a channel without dropping it.
    ///
    /// This prevents any further messages from being sent on the channel while
    /// still enabling the receiver to drain messages that are buffered. Any
    /// outstanding [`Permit`] values will still be able to send messages.
    ///
    /// To guarantee no messages are dropped, after calling `close()`, you must
    /// receive all items from the stream until `None` is returned.
    ///
    /// [`Permit`]: struct@tokio::sync::mpsc::Permit
    pub fn close(&mut self) {
        self.inner.close()
    }
}

impl<T> Stream for FusedReceiverStream<T> {
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.done {
            return Poll::Ready(None);
        }
        let ret = self.inner.poll_recv(cx);
        match &ret {
            Poll::Ready(maybe) => match maybe {
                None => { self.done = true}
                Some(_) => {}
            }
            Poll::Pending => {}
        }
        return ret;
    }
}

impl<T> AsRef<Receiver<T>> for FusedReceiverStream<T> {
    fn as_ref(&self) -> &Receiver<T> {
        &self.inner
    }
}

impl<T> AsMut<Receiver<T>> for FusedReceiverStream<T> {
    fn as_mut(&mut self) -> &mut Receiver<T> {
        &mut self.inner
    }
}

impl<T> From<Receiver<T>> for FusedReceiverStream<T> {
    fn from(recv: Receiver<T>) -> Self {
        Self::new(recv)
    }
}

impl<T> FusedStream for FusedReceiverStream<T> {
    fn is_terminated(&self) -> bool {
        self.done
    }
}

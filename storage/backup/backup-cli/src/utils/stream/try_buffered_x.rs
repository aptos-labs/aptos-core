// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

///! This is a copy of `futures::try_stream::try_buffered` from `futures 0.3.16`, except that it uses
///! `FuturesOrderedX` which provides concurrency control. So we can buffer more results without
///! too many futures driven at the same time.
use crate::utils::stream::futures_ordered_x::FuturesOrderedX;
use core::pin::Pin;
use futures::{
    future::{IntoFuture, TryFuture, TryFutureExt},
    stream::{Fuse, IntoStream, Stream, StreamExt, TryStream},
    task::{Context, Poll},
    TryStreamExt,
};
use pin_project::pin_project;

/// Stream for the [`try_buffered`](super::TryStreamExt::try_buffered) method.
#[pin_project]
#[derive(Debug)]
#[must_use = "streams do nothing unless polled"]
pub struct TryBufferedX<St>
where
    St: TryStream,
    St::Ok: TryFuture,
{
    #[pin]
    stream: Fuse<IntoStream<St>>,
    in_progress_queue: FuturesOrderedX<IntoFuture<St::Ok>>,
    max: usize,
}

impl<St> TryBufferedX<St>
where
    St: TryStream,
    St::Ok: TryFuture,
{
    pub(super) fn new(stream: St, n: usize, max_in_progress: usize) -> Self {
        Self {
            stream: stream.into_stream().fuse(),
            in_progress_queue: FuturesOrderedX::new(max_in_progress),
            max: n,
        }
    }
}

impl<St> Stream for TryBufferedX<St>
where
    St: TryStream,
    St::Ok: TryFuture<Error = St::Error>,
{
    type Item = Result<<St::Ok as TryFuture>::Ok, St::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        // First up, try to spawn off as many futures as possible by filling up
        // our queue of futures. Propagate errors from the stream immediately.
        while this.in_progress_queue.len() < *this.max {
            match this.stream.as_mut().poll_next(cx)? {
                Poll::Ready(Some(fut)) => this.in_progress_queue.push(fut.into_future()),
                Poll::Ready(None) | Poll::Pending => break,
            }
        }

        // Attempt to pull the next value from the in_progress_queue
        match this.in_progress_queue.poll_next_unpin(cx) {
            x @ Poll::Pending | x @ Poll::Ready(Some(_)) => return x,
            Poll::Ready(None) => {}
        }

        // If more values are still coming from the stream, we're not done yet
        if this.stream.is_done() {
            Poll::Ready(None)
        } else {
            Poll::Pending
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::TryStreamX;
    use anyhow::Result;
    use futures::stream::TryStreamExt;
    use proptest::{collection::vec, prelude::*};
    use tokio::{runtime::Runtime, time::Duration};

    proptest! {
        #[test]
        fn test_run(
            sleeps_ms in vec(0u64..10, 0..100),
            buffer_size in 1usize..100,
            max_in_progress in 1usize..100,
        ) {
            let rt = Runtime::new().unwrap();
            rt.block_on(async {
                let num_sleeps = sleeps_ms.len();

                let outputs = futures::stream::iter(
                    sleeps_ms.into_iter().enumerate().map(|(n, sleep_ms)| Ok(async move {
                        tokio::time::sleep(Duration::from_millis(sleep_ms)).await;
                        Result::<_>::Ok(n)
                    }))
                ).try_buffered_x(buffer_size, max_in_progress)
                .try_collect::<Vec<_>>().await.unwrap();

                assert_eq!(
                    outputs,
                    (0..num_sleeps).collect::<Vec<_>>()
                );
            });
        }
    }
}

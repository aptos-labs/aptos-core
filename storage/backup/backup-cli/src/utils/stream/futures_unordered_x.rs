// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

/// This wraps around `futures::stream::futures_unorderd::FuturesUnordered` to provide similar
/// functionality except that there's limit on concurrency. This allows us to manage more futures
/// without activation too many of them at the same time.
use futures::{
    stream::{FusedStream, FuturesUnordered},
    task::{Context, Poll},
    Future, Stream, StreamExt,
};
use std::{collections::VecDeque, fmt::Debug, pin::Pin};

#[must_use = "streams do nothing unless polled"]
pub struct FuturesUnorderedX<T: Future> {
    queued: VecDeque<T>,
    in_progress: FuturesUnordered<T>,
    queued_outputs: VecDeque<T::Output>,
    max_in_progress: usize,
}

impl<T: Future> Unpin for FuturesUnorderedX<T> {}

impl<Fut: Future> FuturesUnorderedX<Fut> {
    /// Constructs a new, empty `FuturesOrderedX`
    ///
    /// The returned `FuturesOrderedX` does not contain any futures and, in this
    /// state, `FuturesOrdered::poll_next` will return `Poll::Ready(None)`.
    pub fn new(max_in_progress: usize) -> FuturesUnorderedX<Fut> {
        assert!(max_in_progress > 0);
        FuturesUnorderedX {
            queued: VecDeque::new(),
            in_progress: FuturesUnordered::new(),
            queued_outputs: VecDeque::new(),
            max_in_progress,
        }
    }

    /// Returns the number of futures contained in the queue.
    ///
    /// This represents the total number of in-flight futures, including those whose outputs queued
    /// for polling, those currently being processing and those in queued due to concurrency limit.
    pub fn len(&self) -> usize {
        self.queued.len() + self.in_progress.len() + self.queued_outputs.len()
    }

    /// Returns `true` if the queue contains no futures
    pub fn is_empty(&self) -> bool {
        self.queued.is_empty() && self.in_progress.is_empty() && self.queued_outputs.is_empty()
    }

    /// Push a future into the queue.
    ///
    /// This function submits the given future to the internal set for managing.
    /// This function will not call `poll` on the submitted future. The caller
    /// must ensure that `FuturesOrdered::poll` is called in order to receive
    /// task notifications.
    pub fn push(&mut self, future: Fut) {
        if self.in_progress.len() < self.max_in_progress {
            self.in_progress.push(future);
        } else {
            self.queued.push_back(future);
        }
    }
}

impl<Fut: Future> Stream for FuturesUnorderedX<Fut> {
    type Item = Fut::Output;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Collect outputs from newly finished futures from the underlying `FuturesUnordered`.
        while let Poll::Ready(Some(output)) = self.in_progress.poll_next_unpin(cx) {
            self.queued_outputs.push_back(output);
            // Concurrency is now below `self.max_in_progress`, kick off a queued one, if any.
            if let Some(future) = self.queued.pop_front() {
                self.in_progress.push(future)
            }
        }

        match self.queued_outputs.pop_front() {
            Some(output) => Poll::Ready(Some(output)),
            _ => {
                if self.in_progress.is_empty() {
                    Poll::Ready(None)
                } else {
                    Poll::Pending
                }
            },
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
}

impl<Fut: Future> Debug for FuturesUnorderedX<Fut> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FuturesOrderedX {{ ... }}")
    }
}

impl<Fut: Future> FusedStream for FuturesUnorderedX<Fut> {
    fn is_terminated(&self) -> bool {
        self.in_progress.is_terminated() && self.queued_outputs.is_empty()
    }
}

impl<Fut: Future> Extend<Fut> for FuturesUnorderedX<Fut> {
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = Fut>,
    {
        for item in iter.into_iter() {
            self.push(item);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FuturesUnorderedX;
    use futures::StreamExt;
    use proptest::prelude::*;
    use std::{
        cmp::min,
        sync::{
            atomic::{AtomicBool, AtomicUsize, Ordering},
            Arc,
        },
    };
    use tokio::{runtime::Runtime, time::Duration};

    proptest! {
        #[test]
        fn test_run(
            num_tasks in 0usize..100,
            max_in_progress in 1usize..100,
        ) {
            const MAX_WAIT_MS: usize = 1000;

            let rt = Runtime::new().unwrap();
            rt.block_on(async {
                let mut futures = FuturesUnorderedX::new(max_in_progress);
                assert!(futures.is_empty());

                let n_running = Arc::new(AtomicUsize::new(0));
                let seen_max_concurrency = Arc::new(AtomicBool::new(false));

                for n in 0..num_tasks {
                    let n_running = n_running.clone();
                    let seen_max_concurrency = seen_max_concurrency.clone();

                    futures.push(async move {
                        n_running.fetch_add(1, Ordering::Relaxed);

                        for _ in 0..MAX_WAIT_MS {
                            // yield
                            tokio::time::sleep(Duration::from_millis(1)).await;
                            if num_tasks < max_in_progress {
                                break
                            }
                            if seen_max_concurrency.load(Ordering::Relaxed) {
                                break
                            }
                            if n_running.load(Ordering::Relaxed) == max_in_progress {
                                seen_max_concurrency.store(true, Ordering::Relaxed);
                                break
                            }
                        }

                        let r = n_running.fetch_sub(1, Ordering::SeqCst);
                        assert!(r > 0 && r <= min(max_in_progress, num_tasks));
                        n
                    })
                }

                assert!(num_tasks > 0 || futures.is_empty());
                let mut outputs = futures.collect::<Vec<_>>().await;
                if max_in_progress <= num_tasks {
                    assert!(seen_max_concurrency.load(Ordering::Relaxed));
                }

                outputs.sort_unstable();
                assert_eq!(outputs, (0..num_tasks).collect::<Vec<_>>());
            });
        }
    }
}

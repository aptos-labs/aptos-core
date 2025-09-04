// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::BoundedExecutor;
use futures::{
    stream::{self, FusedStream},
    Future, FutureExt, Stream, StreamExt,
};

pub fn concurrent_map<St, Fut, F>(
    stream: St,
    executor: BoundedExecutor,
    mut mapper: F,
) -> impl FusedStream<Item = Fut::Output>
where
    St: Stream,
    F: FnMut(St::Item) -> Fut + Send,
    Fut: Future + Send + 'static,
    Fut::Output: Send + 'static,
{
    stream
        .flat_map_unordered(None, move |item| {
            let future = mapper(item);
            let executor = executor.clone();
            stream::once(
                #[allow(clippy::async_yields_async)]
                async move { executor.spawn(future).await }.boxed(),
            )
            .boxed()
        })
        .flat_map_unordered(None, |handle| {
            stream::once(async move { handle.await.expect("result") }.boxed()).boxed()
        })
        .fuse()
}

#[rustversion::since(1.75)]
#[allow(dead_code)]
pub trait ConcurrentStream: Stream {
    fn concurrent_map<Fut, F>(
        self,
        executor: BoundedExecutor,
        mapper: F,
    ) -> impl FusedStream<Item = Fut::Output>
    where
        F: FnMut(Self::Item) -> Fut + Send,
        Fut: Future + Send + 'static,
        Fut::Output: Send + 'static,
        Self: Sized,
    {
        concurrent_map(self, executor, mapper)
    }
}

#[rustversion::since(1.75)]
impl<T: ?Sized> ConcurrentStream for T where T: Stream {}

#[cfg(test)]
mod test {
    use crate::{concurrent_stream::concurrent_map, BoundedExecutor};
    use futures::{stream, FutureExt, StreamExt};
    use std::{
        sync::atomic::{AtomicU32, Ordering},
        time::Duration,
    };
    use tokio::runtime::Handle;

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_concurrent_stream() {
        const MAX_WORKERS: u32 = 20;
        const NUM_TASKS: u32 = 1000;
        static WORKERS: AtomicU32 = AtomicU32::new(0);
        static COMPLETED_TASKS: AtomicU32 = AtomicU32::new(0);

        let stream = stream::iter(0..NUM_TASKS).fuse();

        let executor = Handle::current();
        let executor = BoundedExecutor::new(MAX_WORKERS as usize, executor);

        let handle = tokio::spawn(async {
            concurrent_map(stream, executor, |_input| async {
                let prev_workers = WORKERS.fetch_add(1, Ordering::SeqCst);
                assert!(prev_workers < MAX_WORKERS);

                // yield back to the tokio scheduler
                tokio::time::sleep(Duration::from_millis(1))
                    .map(|_| ())
                    .await;

                let prev_workers = WORKERS.fetch_sub(1, Ordering::SeqCst);
                assert!(prev_workers > 0 && prev_workers <= MAX_WORKERS);

                COMPLETED_TASKS.fetch_add(1, Ordering::Relaxed);
            })
            .count()
            .await
        });

        // spin until completed
        loop {
            let completed = COMPLETED_TASKS.load(Ordering::Relaxed);
            if completed == NUM_TASKS {
                break;
            } else {
                std::hint::spin_loop()
            }
        }

        assert_eq!(handle.await.unwrap() as u32, NUM_TASKS);
    }

    #[rustversion::since(1.75)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_concurrent_stream_with_trait_impl() {
        use crate::concurrent_stream::ConcurrentStream;

        const MAX_WORKERS: u32 = 20;
        const NUM_TASKS: u32 = 1000;
        static WORKERS: AtomicU32 = AtomicU32::new(0);
        static COMPLETED_TASKS: AtomicU32 = AtomicU32::new(0);

        let stream = stream::iter(0..NUM_TASKS).fuse();

        let executor = Handle::current();
        let executor = BoundedExecutor::new(MAX_WORKERS as usize, executor);

        let handle = tokio::spawn(async {
            stream
                .concurrent_map(executor, |_input| async {
                    let prev_workers = WORKERS.fetch_add(1, Ordering::SeqCst);
                    assert!(prev_workers < MAX_WORKERS);

                    // yield back to the tokio scheduler
                    tokio::time::sleep(Duration::from_millis(1))
                        .map(|_| ())
                        .await;

                    let prev_workers = WORKERS.fetch_sub(1, Ordering::SeqCst);
                    assert!(prev_workers > 0 && prev_workers <= MAX_WORKERS);

                    COMPLETED_TASKS.fetch_add(1, Ordering::Relaxed);
                })
                .count()
                .await
        });

        // spin until completed
        loop {
            let completed = COMPLETED_TASKS.load(Ordering::Relaxed);
            if completed == NUM_TASKS {
                break;
            } else {
                std::hint::spin_loop()
            }
        }

        assert_eq!(handle.await.unwrap() as u32, NUM_TASKS);
    }
}

// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! A bounded tokio [`Handle`]. Only a bounded number of tasks can run
//! concurrently when spawned through this executor, defined by the initial
//! `capacity`.

use futures::future::{Future, FutureExt};
use std::sync::Arc;
use tokio::{
    runtime::Handle,
    sync::{OwnedSemaphorePermit, Semaphore},
    task::JoinHandle,
};

#[derive(Clone, Debug)]
pub struct BoundedExecutor {
    semaphore: Arc<Semaphore>,
    executor: Handle,
}

impl BoundedExecutor {
    /// Create a new `BoundedExecutor` from an existing tokio [`Handle`]
    /// with a maximum concurrent task capacity of `capacity`.
    pub fn new(capacity: usize, executor: Handle) -> Self {
        let semaphore = Arc::new(Semaphore::new(capacity));
        Self {
            semaphore,
            executor,
        }
    }

    async fn acquire_permit(&self) -> OwnedSemaphorePermit {
        self.semaphore.clone().acquire_owned().await.unwrap()
    }

    fn try_acquire_permit(&self) -> Option<OwnedSemaphorePermit> {
        self.semaphore.clone().try_acquire_owned().ok()
    }

    /// Spawn a [`Future`] on the `BoundedExecutor`. This function is async and
    /// will block if the executor is at capacity until one of the other spawned
    /// futures completes. This function returns a [`JoinHandle`] that the caller
    /// can `.await` on for the results of the [`Future`].
    pub async fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let permit = self.acquire_permit().await;
        self.executor.spawn(future_with_permit(future, permit))
    }

    /// Try to spawn a [`Future`] on the `BoundedExecutor`. If the `BoundedExecutor`
    /// is at capacity, this will return an `Err(F)`, passing back the future the
    /// caller attempted to spawn. Otherwise, this will spawn the future on the
    /// executor and send back a [`JoinHandle`] that the caller can `.await` on
    /// for the results of the [`Future`].
    pub fn try_spawn<F>(&self, future: F) -> Result<JoinHandle<F::Output>, F>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        match self.try_acquire_permit() {
            Some(permit) => Ok(self.executor.spawn(future_with_permit(future, permit))),
            None => Err(future),
        }
    }

    /// Like [`BoundedExecutor::spawn`] but spawns the given closure onto a
    /// blocking task (see [`tokio::task::spawn_blocking`] for details).
    pub async fn spawn_blocking<F, R>(&self, func: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let permit = self.acquire_permit().await;
        self.executor
            .spawn_blocking(function_with_permit(func, permit))
    }

    /// Like [`BoundedExecutor::try_spawn`] but spawns the given closure
    /// onto a blocking task (see [`tokio::task::spawn_blocking`] for details).
    pub async fn try_spawn_blocking<F, R>(&self, func: F) -> Result<JoinHandle<R>, F>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        match self.try_acquire_permit() {
            Some(permit) => Ok(self
                .executor
                .spawn_blocking(function_with_permit(func, permit))),
            None => Err(func),
        }
    }
}

/// Wrap a `Future` so it releases the spawn permit back to the semaphore when
/// it completes.
fn future_with_permit<F>(future: F, permit: OwnedSemaphorePermit) -> impl Future<Output = F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    future.map(move |ret| {
        drop(permit);
        ret
    })
}

fn function_with_permit<F, R>(
    func: F,
    permit: OwnedSemaphorePermit,
) -> impl FnOnce() -> R + Send + 'static
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    move || {
        let ret = func();
        drop(permit);
        ret
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use futures::{channel::oneshot, executor::block_on, future::Future};
    use std::{
        sync::atomic::{AtomicU32, Ordering},
        time::Duration,
    };
    use tokio::{runtime::Runtime, time::sleep};

    #[test]
    fn try_spawn() {
        let rt = Runtime::new().unwrap();
        let executor = rt.handle().clone();
        let executor = BoundedExecutor::new(1, executor);

        let (tx1, rx1) = oneshot::channel();
        let (tx2, rx2) = oneshot::channel();

        // executor has a free slot, spawn should succeed

        let f1 = executor.try_spawn(rx1).unwrap();

        // executor is full, try_spawn should return err and give back the task
        // we attempted to spawn

        let rx2 = executor.try_spawn(rx2).unwrap_err();

        // complete f1 future, should open a free slot in executor

        tx1.send(()).unwrap();
        block_on(f1).unwrap().unwrap();

        // should successfully spawn a new task now that the first is complete

        let f2 = executor.try_spawn(rx2).unwrap();

        // cleanup

        tx2.send(()).unwrap();
        block_on(f2).unwrap().unwrap();
    }

    fn yield_task() -> impl Future<Output = ()> {
        sleep(Duration::from_millis(1)).map(|_| ())
    }

    // spawn NUM_TASKS futures on a BoundedExecutor, ensuring that no more than
    // MAX_WORKERS ever enter the critical section.
    #[test]
    fn concurrent_bounded_executor() {
        const MAX_WORKERS: u32 = 20;
        const NUM_TASKS: u32 = 1000;
        static WORKERS: AtomicU32 = AtomicU32::new(0);
        static COMPLETED_TASKS: AtomicU32 = AtomicU32::new(0);

        let rt = Runtime::new().unwrap();
        let executor = rt.handle().clone();
        let executor = BoundedExecutor::new(MAX_WORKERS as usize, executor);

        for _ in 0..NUM_TASKS {
            block_on(executor.spawn(async move {
                // acquired permit, there should only ever be MAX_WORKERS in this
                // critical section

                let prev_workers = WORKERS.fetch_add(1, Ordering::SeqCst);
                assert!(prev_workers < MAX_WORKERS);

                // yield back to the tokio scheduler
                yield_task().await;

                let prev_workers = WORKERS.fetch_sub(1, Ordering::SeqCst);
                assert!(prev_workers > 0 && prev_workers <= MAX_WORKERS);

                COMPLETED_TASKS.fetch_add(1, Ordering::Relaxed);
            }));
        }

        // spin until completed
        loop {
            let completed = COMPLETED_TASKS.load(Ordering::Relaxed);
            if completed == NUM_TASKS {
                break;
            } else {
                std::hint::spin_loop()
            }
        }
    }
}

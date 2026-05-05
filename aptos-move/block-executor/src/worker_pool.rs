// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Worker thread pool that runs BlockSTM workers on plain `std::thread`s.
//!
//! Unlike a rayon thread pool, threads here do not participate in
//! cross-scope work stealing. BlockSTM workers can park on a `Condvar`
//! while waiting for a transaction dependency, and a rayon worker parked
//! inside a nested `par_iter()` could end up running an inner task spawned
//! by a different worker; the two could deadlock through a circular
//! dependency on the underlying transaction execution. Plain `std` threads
//! are not registered with rayon, so any nested `par_iter()` runs on
//! rayon's global pool while the worker simply blocks via OS primitives.
//!
//! The pool is process-global. It eagerly spawns `num_cpus::get()` worker
//! threads on first use and they run for the lifetime of the process.
//!
//! Internally `scope` widens the borrow of the user-provided closure to
//! `'static` and dispatches `num_tasks` boxed tasks through an MPMC
//! channel. The function then blocks until every task has signaled
//! completion via a [`Barrier`], which is what makes the lifetime
//! extension sound.

use crossbeam::channel::Sender;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::{
    any::Any,
    sync::{Arc, Barrier},
};

type Task = Box<dyn FnOnce() + Send + 'static>;

pub struct WorkerPool {
    sender: Sender<Task>,
}

static GLOBAL_POOL: Lazy<WorkerPool> = Lazy::new(WorkerPool::new);

impl WorkerPool {
    fn new() -> Self {
        let (sender, receiver) = crossbeam::channel::unbounded::<Task>();
        for id in 0..num_cpus::get() {
            let receiver = receiver.clone();
            std::thread::Builder::new()
                .name(format!("par_exec-{}", id))
                .spawn(move || {
                    while let Ok(task) = receiver.recv() {
                        task();
                    }
                })
                .expect("Failed to spawn par_exec worker thread");
        }
        Self { sender }
    }

    /// The process-wide pool used by BlockSTM.
    pub fn global() -> &'static WorkerPool {
        &GLOBAL_POOL
    }

    /// Run `num_tasks` invocations of `work`, each receiving its index in
    /// `0..num_tasks`. Blocks until every invocation has either returned
    /// or panicked. If any task panics, the first captured panic is
    /// resumed in the calling thread after every task has completed.
    pub fn scope<F>(&self, num_tasks: usize, work: F)
    where
        F: Fn(usize) + Sync,
    {
        if num_tasks == 0 {
            return;
        }

        // SAFETY: The closure is dispatched to worker threads through a
        // channel, which requires `'static`. We block on `barrier.wait()`
        // below until every task has reached the barrier, and the worker
        // threads only access `work_static` before reaching the barrier.
        // Any captured `&'static` reference held by the task closure
        // beyond that point is never dereferenced (a reference's drop is
        // a no-op), so it cannot outlive the borrow of `work` in any
        // observable way.
        let work_static: &'static (dyn Fn(usize) + Sync) = unsafe {
            let work_ref: &(dyn Fn(usize) + Sync) = &work;
            std::mem::transmute(work_ref)
        };

        let barrier = Arc::new(Barrier::new(num_tasks + 1));
        let panic_slot: Arc<Mutex<Option<Box<dyn Any + Send>>>> = Arc::new(Mutex::new(None));

        for i in 0..num_tasks {
            let barrier = Arc::clone(&barrier);
            let panic_slot = Arc::clone(&panic_slot);
            let task: Task = Box::new(move || {
                let result =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| work_static(i)));
                if let Err(p) = result {
                    let mut slot = panic_slot.lock();
                    if slot.is_none() {
                        *slot = Some(p);
                    }
                }
                barrier.wait();
            });
            self.sender
                .send(task)
                .expect("WorkerPool channel disconnected");
        }

        barrier.wait();

        if let Some(p) = panic_slot.lock().take() {
            std::panic::resume_unwind(p);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

    #[test]
    fn runs_tasks_with_indices() {
        let seen = (0..8).map(|_| AtomicU32::new(0)).collect::<Vec<_>>();
        WorkerPool::global().scope(8, |i| {
            seen[i].store(i as u32 + 1, Ordering::Relaxed);
        });
        for (i, slot) in seen.iter().enumerate() {
            assert_eq!(slot.load(Ordering::Relaxed), i as u32 + 1);
        }
    }

    #[test]
    fn zero_tasks_is_noop() {
        WorkerPool::global().scope(0, |_| panic!("should not run"));
    }

    #[test]
    fn propagates_panic() {
        let result = std::panic::catch_unwind(|| {
            WorkerPool::global().scope(4, |i| {
                if i == 2 {
                    panic!("boom from {}", i);
                }
            });
        });
        let payload = result.expect_err("scope should have panicked");
        let msg = payload
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| payload.downcast_ref::<&'static str>().copied())
            .unwrap_or("");
        assert!(msg.contains("boom"), "unexpected panic payload: {:?}", msg);
    }

    #[test]
    fn concurrent_scopes_make_progress() {
        // Two concurrent scopes whose tasks each wait for the other scope
        // to start. Requires the pool to have at least 8 worker threads;
        // on any modern host `num_cpus::get() >= 8`.
        let started_a = Arc::new(AtomicUsize::new(0));
        let started_b = Arc::new(AtomicUsize::new(0));
        let a_handle = {
            let started_a = Arc::clone(&started_a);
            let started_b = Arc::clone(&started_b);
            std::thread::spawn(move || {
                WorkerPool::global().scope(4, |_| {
                    started_a.fetch_add(1, Ordering::SeqCst);
                    while started_b.load(Ordering::SeqCst) < 4 {
                        std::thread::yield_now();
                    }
                });
            })
        };
        let b_handle = {
            let started_a = Arc::clone(&started_a);
            let started_b = Arc::clone(&started_b);
            std::thread::spawn(move || {
                WorkerPool::global().scope(4, |_| {
                    started_b.fetch_add(1, Ordering::SeqCst);
                    while started_a.load(Ordering::SeqCst) < 4 {
                        std::thread::yield_now();
                    }
                });
            })
        };
        a_handle.join().unwrap();
        b_handle.join().unwrap();
    }
}

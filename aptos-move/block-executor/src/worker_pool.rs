// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Worker thread pool that runs BlockSTM workers on plain `std::thread`s.
//!
//! Unlike a rayon thread pool, threads here do not participate in cross-scope work stealing.
//! BlockSTM workers can park on a `Condvar` while waiting for a transaction dependency. If the
//! workers ran on a rayon pool, a worker waiting inside a nested `par_iter()` would actively
//! work-steal and could pick up an inner task spawned by another worker; that inner task could then
//! park on the v1 dependency `Condvar` waiting on the very transaction the work-stealing thread is
//! supposed to be executing, deadlocking. Plain `std` threads are not registered with rayon, so any
//! nested `par_iter()` runs on rayon's global pool while the worker simply blocks via OS
//! primitives.

use crate::counters::PAR_EXEC_POOL_SIZE;
use aptos_logger::info;
use parking_lot::Mutex;
use std::{
    any::Any,
    sync::{Arc, Barrier},
};

type Task = Box<dyn FnOnce() + Send + 'static>;

struct State {
    // Sum of `num_tasks` across all in-flight `scope` calls.
    pending: usize,
    // Number of worker threads ever spawned (current live worker count).
    spawned: usize,
}

pub struct WorkerPool {
    sender: crossbeam::channel::Sender<Task>,
    receiver: crossbeam::channel::Receiver<Task>,
    state: Mutex<State>,
}

impl WorkerPool {
    pub(crate) fn new() -> Self {
        let (sender, receiver) = crossbeam::channel::unbounded();
        Self {
            sender,
            receiver,
            state: Mutex::new(State {
                pending: 0,
                spawned: 0,
            }),
        }
    }

    /// Run `num_tasks` invocations of `work`, each receiving its index in `0..num_tasks`. Blocks
    /// until every invocation has either returned or panicked. If any task panics, the first
    /// captured panic is resumed in the calling thread after every task has completed.
    ///
    /// `scope` MUST NOT be called recursively from within a task running on this pool: an inner
    /// scope would block one of the workers it itself depends on. Growth is unbounded so this is
    /// liveness, not soundness, but the pattern wastes threads.
    pub fn scope<F>(&self, num_tasks: usize, work: F)
    where
        F: Fn(usize) + Sync,
    {
        if num_tasks == 0 {
            return;
        }

        let _decrement = self.grow_and_bump_pending(num_tasks);

        // SAFETY: `work_static` is a fictitious `'static` borrow used only so the closure can be
        // sent through the MPMC channel. Sound because the caller's `barrier.wait()` below blocks
        // until every worker has stopped dereferencing it, before `work` is dropped. Inline
        // comments below justify each step.
        let work_static: &'static (dyn Fn(usize) + Sync) = unsafe {
            let work_ref: &(dyn Fn(usize) + Sync) = &work;
            std::mem::transmute(work_ref)
        };

        let barrier = Arc::new(Barrier::new(num_tasks + 1));
        let panic_slot: Arc<Mutex<Option<Box<dyn Any + Send>>>> = Arc::new(Mutex::new(None));

        // Build every task before sending any. If `Box::new` panics mid-loop (e.g. allocator OOM),
        // the partial `Vec` drops without anything being sent.
        let mut tasks: Vec<Task> = Vec::with_capacity(num_tasks);
        for i in 0..num_tasks {
            let barrier = Arc::clone(&barrier);
            let panic_slot = Arc::clone(&panic_slot);
            tasks.push(Box::new(move || {
                let result =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| work_static(i)));
                // The path from here to `barrier.wait()` must be panic-free: a worker that skips
                // the barrier would let the caller drop `work` while we still hold `work_static`.
                if let Err(p) = result {
                    let mut slot = panic_slot.lock();
                    if slot.is_none() {
                        *slot = Some(p);
                    }
                }
                barrier.wait();
            }));
        }

        // Infallible: `self.receiver` is held by `WorkerPool` for the lifetime of
        // `&self`, so the channel cannot disconnect.
        for task in tasks {
            self.sender
                .send(task)
                .expect("WorkerPool channel disconnected (unreachable)");
        }

        // Returns only after every worker has reached its own `barrier.wait()`, even if some of
        // them panic, so no worker still dereferences `work_static`.
        barrier.wait();

        if let Some(p) = panic_slot.lock().take() {
            std::panic::resume_unwind(p);
        }
    }

    /// Grows the pool to cover the new demand (rare) and reserves `num_tasks` worker slots.
    /// Returns a RAII guard that rolls back the reservation on drop.
    fn grow_and_bump_pending(&self, num_tasks: usize) -> PendingDecrement<'_> {
        let mut state = self.state.lock();
        let target = state.pending + num_tasks;
        if state.spawned < target {
            info!(
                "Growing par_exec worker pool from {} to {} thread(s)",
                state.spawned, target
            );
            while state.spawned < target {
                let receiver = self.receiver.clone();
                let id = state.spawned;
                std::thread::Builder::new()
                    .name(format!("par_exec-{}", id))
                    .spawn(move || {
                        while let Ok(task) = receiver.recv() {
                            task();
                        }
                        info!("par_exec worker {} exiting (channel disconnected)", id);
                    })
                    .expect("Failed to spawn par_exec worker thread");
                state.spawned += 1;
            }
            PAR_EXEC_POOL_SIZE.set(state.spawned as i64);
        }
        state.pending += num_tasks;
        PendingDecrement {
            state: &self.state,
            n: num_tasks,
        }
    }
}

struct PendingDecrement<'a> {
    state: &'a Mutex<State>,
    n: usize,
}

impl Drop for PendingDecrement<'_> {
    fn drop(&mut self) {
        self.state.lock().pending -= self.n;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

    #[test]
    fn runs_tasks_with_indices() {
        let pool = WorkerPool::new();
        let seen = (0..8).map(|_| AtomicU32::new(0)).collect::<Vec<_>>();
        pool.scope(8, |i| {
            seen[i].store(i as u32 + 1, Ordering::Relaxed);
        });
        for (i, slot) in seen.iter().enumerate() {
            assert_eq!(slot.load(Ordering::Relaxed), i as u32 + 1);
        }
        // PendingDecrement rolls `pending` back on the success path.
        assert_eq!(pool.state.lock().pending, 0);
    }

    #[test]
    fn zero_tasks_is_noop() {
        let pool = WorkerPool::new();
        pool.scope(0, |_| panic!("should not run"));
    }

    #[test]
    fn propagates_panic() {
        let pool = WorkerPool::new();
        let ran = AtomicUsize::new(0);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            pool.scope(4, |i| {
                if i == 2 {
                    panic!("boom from {}", i);
                }
                ran.fetch_add(1, Ordering::Relaxed);
            });
        }));
        let payload = result.expect_err("scope should have panicked");
        // `panic!("...{}", _)` always produces a `String` payload.
        let msg = payload
            .downcast_ref::<String>()
            .expect("panic payload should be a String");
        assert!(msg.contains("boom"), "unexpected panic payload: {:?}", msg);
        // The three non-panicking tasks must still complete — a panic in one
        // task must not prevent the others from finishing.
        assert_eq!(ran.load(Ordering::Relaxed), 3);
    }

    #[test]
    fn propagates_one_of_multiple_panics() {
        // Two tasks panic concurrently. The first to grab the panic_slot wins
        // (`if slot.is_none()` discards subsequent panics), the rest are
        // dropped, and the scope must still return rather than deadlock.
        let pool = WorkerPool::new();
        let started = AtomicUsize::new(0);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            pool.scope(4, |i| {
                started.fetch_add(1, Ordering::Relaxed);
                if i == 0 || i == 2 {
                    panic!("boom-from-{}", i);
                }
            });
        }));
        let payload = result.expect_err("scope should have panicked");
        let msg = payload
            .downcast_ref::<String>()
            .expect("panic payload should be a String");
        assert!(
            msg == "boom-from-0" || msg == "boom-from-2",
            "expected one of the two panics, got: {:?}",
            msg
        );
        assert_eq!(started.load(Ordering::Relaxed), 4);
    }

    #[test]
    fn pending_restored_after_panic() {
        let pool = WorkerPool::new();
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            pool.scope(4, |_| panic!("boom"));
        }));
        assert_eq!(pool.state.lock().pending, 0);
    }

    #[test]
    fn grows_beyond_num_cpus() {
        // Unbounded growth: a single scope can request more workers than
        // there are CPUs and still complete. Pick a target dramatically
        // larger than num_cpus so any future hard-cap regression (e.g.
        // capping at num_cpus or 2 * num_cpus) trips the assertion below.
        let pool = WorkerPool::new();
        let target = num_cpus::get() * 4 + 16;
        let counter = AtomicUsize::new(0);
        pool.scope(target, |_| {
            counter.fetch_add(1, Ordering::Relaxed);
        });
        assert_eq!(counter.load(Ordering::Relaxed), target);
        assert!(pool.state.lock().spawned >= target);
    }

    #[test]
    fn concurrent_scopes_overcommit_pool() {
        // Two scopes whose combined worker demand exceeds `num_cpus`,
        // each waiting for the other to start. With unbounded growth
        // both make progress regardless of host core count.
        let pool = WorkerPool::new();
        let per_scope = num_cpus::get() + 4;
        let started_a = AtomicUsize::new(0);
        let started_b = AtomicUsize::new(0);
        std::thread::scope(|s| {
            s.spawn(|| {
                pool.scope(per_scope, |_| {
                    started_a.fetch_add(1, Ordering::SeqCst);
                    while started_b.load(Ordering::SeqCst) < per_scope {
                        std::thread::yield_now();
                    }
                });
            });
            s.spawn(|| {
                pool.scope(per_scope, |_| {
                    started_b.fetch_add(1, Ordering::SeqCst);
                    while started_a.load(Ordering::SeqCst) < per_scope {
                        std::thread::yield_now();
                    }
                });
            });
        });
    }

    #[test]
    fn no_growth_when_capacity_already_sufficient() {
        let pool = WorkerPool::new();
        pool.scope(8, |_| {});
        let after_first = pool.state.lock().spawned;
        // Subsequent scopes within the established capacity do not grow.
        pool.scope(4, |_| {});
        assert_eq!(pool.state.lock().spawned, after_first);
    }
}

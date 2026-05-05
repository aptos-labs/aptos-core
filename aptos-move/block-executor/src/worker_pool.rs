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
//! The pool grows lazily and without an upper bound. Each `scope(K, _)`
//! call adds `K` to a `pending` counter; if `pending` exceeds the number
//! of currently-spawned threads, the pool spawns enough new threads to
//! cover the gap. Threads, once spawned, live for the lifetime of the
//! pool — we do not reap idle workers. The realistic ceiling is the peak
//! concurrent demand across all callers (single-process BlockSTM:
//! bounded by `num_cpus * num_concurrent_callers`), and reaping would
//! only churn threads through repeat-execution workloads that hit
//! steady-state quickly.

use crate::counters::PAR_EXEC_POOL_SIZE;
use parking_lot::Mutex;
use std::{
    any::Any,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Barrier,
    },
};

type Task = Box<dyn FnOnce() + Send + 'static>;

pub struct WorkerPool {
    sender: crossbeam::channel::Sender<Task>,
    // Held so the channel cannot become disconnected while the pool is
    // alive, even if every spawned worker exits. This is what makes
    // `sender.send(...)` infallible inside `scope` (see SAFETY note there).
    receiver: crossbeam::channel::Receiver<Task>,
    // Sum of `num_tasks` across all in-flight `scope` calls.
    pending: AtomicUsize,
    // Number of worker threads ever spawned. We never reap, so this is
    // also the current live worker count modulo workers that exited
    // early due to a panic in the worker loop itself (not in `work`).
    spawned: AtomicUsize,
    // Serializes spawn decisions so two concurrent `scope` calls don't
    // race to over-spawn. Reads on the fast path are unsynchronized; the
    // mutex re-checks under exclusion.
    spawn_mu: Mutex<()>,
}

impl WorkerPool {
    pub(crate) fn new() -> Self {
        let (sender, receiver) = crossbeam::channel::unbounded::<Task>();
        Self {
            sender,
            receiver,
            pending: AtomicUsize::new(0),
            spawned: AtomicUsize::new(0),
            spawn_mu: Mutex::new(()),
        }
    }

    fn ensure_capacity(&self) {
        // Fast path: no growth needed.
        if self.spawned.load(Ordering::Acquire) >= self.pending.load(Ordering::Acquire) {
            return;
        }
        let _guard = self.spawn_mu.lock();
        // Re-check under the lock; another caller may have grown the pool.
        let target = self.pending.load(Ordering::Acquire);
        let mut spawned = self.spawned.load(Ordering::Acquire);
        while spawned < target {
            let receiver = self.receiver.clone();
            std::thread::Builder::new()
                .name(format!("par_exec-{}", spawned))
                .spawn(move || {
                    while let Ok(task) = receiver.recv() {
                        task();
                    }
                })
                .expect("Failed to spawn par_exec worker thread");
            spawned += 1;
            // Publish each successful spawn so a panic part-way through
            // doesn't leave `spawned` overstating reality.
            self.spawned.store(spawned, Ordering::Release);
        }
        PAR_EXEC_POOL_SIZE.set(spawned as i64);
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

        self.pending.fetch_add(num_tasks, Ordering::AcqRel);
        // RAII so a panic in `ensure_capacity` or the send loop still
        // restores `pending`.
        let _decrement = PendingDecrement {
            pending: &self.pending,
            n: num_tasks,
        };
        self.ensure_capacity();

        // SAFETY: We extend the borrow of `work` to `'static` so that the closure can be sent
        // through the MPMC channel (which requires `'static`). This is sound under three
        // invariants, all upheld below:
        //
        // 1. Every spawned task reaches `barrier.wait()` before the caller's `barrier.wait()`
        //    returns. The only access to `work_static` happens inside `catch_unwind`, which
        //    completes before the worker reaches the barrier.
        //
        // 2. The path between `catch_unwind` and `barrier.wait()` cannot panic: `parking_lot::Mutex`
        //    is poison-free, `Option::is_none` plus an assignment cannot panic, and `Barrier::wait`
        //    does not panic per its contract. So a panic inside `work` cannot short-circuit the
        //    worker's barrier arrival.
        //
        // 3. `self.sender.send(...)` cannot fail because the channel cannot disconnect:
        //    `self.receiver` is held in `WorkerPool` for as long as `&self` is valid, so at least
        //    one `Receiver` is always alive. The send loop therefore always sends every task; no
        //    partial-send unwind can leave in-flight tasks holding a stale `work_static`.
        //
        // After the caller's `barrier.wait()` returns, no worker dereferences `work_static` again.
        // The captured `&'static` sitting inside the boxed closure is dropped by the worker, but
        // dropping a reference is a no-op, so no use-after-free is possible even though the box
        // outlives `work`.
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
            // See SAFETY note above: this send is infallible because the
            // pool itself holds a `Receiver`.
            self.sender
                .send(task)
                .expect("WorkerPool channel disconnected (unreachable)");
        }

        barrier.wait();

        if let Some(p) = panic_slot.lock().take() {
            std::panic::resume_unwind(p);
        }
    }
}

struct PendingDecrement<'a> {
    pending: &'a AtomicUsize,
    n: usize,
}

impl Drop for PendingDecrement<'_> {
    fn drop(&mut self) {
        self.pending.fetch_sub(self.n, Ordering::AcqRel);
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
    }

    #[test]
    fn zero_tasks_is_noop() {
        let pool = WorkerPool::new();
        pool.scope(0, |_| panic!("should not run"));
    }

    #[test]
    fn propagates_panic() {
        let pool = WorkerPool::new();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            pool.scope(4, |i| {
                if i == 2 {
                    panic!("boom from {}", i);
                }
            });
        }));
        let payload = result.expect_err("scope should have panicked");
        let msg = payload
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| payload.downcast_ref::<&'static str>().copied())
            .unwrap_or("");
        assert!(msg.contains("boom"), "unexpected panic payload: {:?}", msg);
    }

    #[test]
    fn pending_restored_after_panic() {
        let pool = WorkerPool::new();
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            pool.scope(4, |_| panic!("boom"));
        }));
        assert_eq!(pool.pending.load(Ordering::Acquire), 0);
    }

    #[test]
    fn grows_beyond_num_cpus() {
        // Unbounded growth: a single scope can request more workers than
        // there are CPUs and still complete.
        let pool = WorkerPool::new();
        let target = num_cpus::get() * 4 + 16;
        let counter = AtomicUsize::new(0);
        pool.scope(target, |_| {
            counter.fetch_add(1, Ordering::Relaxed);
        });
        assert_eq!(counter.load(Ordering::Relaxed), target);
        assert!(pool.spawned.load(Ordering::Acquire) >= target);
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
        let after_first = pool.spawned.load(Ordering::Acquire);
        // Subsequent scopes within the established capacity do not grow.
        pool.scope(4, |_| {});
        assert_eq!(pool.spawned.load(Ordering::Acquire), after_first);
    }
}

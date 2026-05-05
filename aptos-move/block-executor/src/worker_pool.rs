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
use std::sync::{Arc, Barrier};

type Task = Box<dyn FnOnce() + Send + 'static>;

struct State {
    // Sum of `num_tasks` across all in-flight `scope` calls.
    in_flight: usize,
    // Number of worker threads we have created.
    spawned: usize,
}

pub(crate) struct WorkerPool {
    sender: crossbeam::channel::Sender<Task>,
    receiver: crossbeam::channel::Receiver<Task>,
    state: Mutex<State>,
}

impl WorkerPool {
    pub fn new() -> Self {
        let (sender, receiver) = crossbeam::channel::unbounded();
        Self {
            sender,
            receiver,
            state: Mutex::new(State {
                in_flight: 0,
                spawned: 0,
            }),
        }
    }

    /// Run `num_tasks` invocations of `work`, each receiving its index in `0..num_tasks`. Blocks
    /// until every invocation has either returned or panicked. If any task panics, the first
    /// captured panic is resumed in the calling thread after every task has completed.
    pub fn scope<F>(&self, num_tasks: usize, work: F)
    where
        F: Fn(usize) + Sync,
    {
        if num_tasks == 0 {
            return;
        }

        let _decrement = self.grow_and_bump_in_flight(num_tasks);

        // SAFETY: we manually extend the lifetime of `work_ref` to `'static`, in order to send it
        // to be executed by threads. This is safe because this function blocks until every worker
        // has finished and stopped dereferencing it, and `work` is only dropped afterwards.
        let work_static: &'static (dyn Fn(usize) + Sync) = unsafe {
            let work_ref: &(dyn Fn(usize) + Sync) = &work;
            std::mem::transmute(work_ref)
        };

        let barrier = Arc::new(Barrier::new(num_tasks + 1));
        let panic_slot = Arc::new(Mutex::new(None));

        // Build every task before sending any. If `Box::new` panics mid-loop (e.g. allocator OOM),
        // the partial `Vec` drops without anything being sent.
        let mut tasks = Vec::with_capacity(num_tasks);
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

        // Infallible: `self.receiver` is held by `WorkerPool` for the lifetime of `&self`, so the
        // channel cannot disconnect.
        for task in tasks {
            self.sender
                .send(task)
                .expect("WorkerPool channel disconnected (unreachable)");
        }

        // Wait until EVERY worker has reached its own `barrier.wait()`, even if some of them panic,
        // so no worker still dereferences `work_static`.
        barrier.wait();

        if let Some(p) = panic_slot.lock().take() {
            std::panic::resume_unwind(p);
        }
    }

    /// Grows the pool to cover the new demand (rare) and reserves `num_tasks` worker slots.
    /// Returns a RAII guard that rolls back the reservation on drop.
    fn grow_and_bump_in_flight(&self, num_tasks: usize) -> InFlightDecrement<'_> {
        let mut state = self.state.lock();
        let target = state.in_flight + num_tasks;
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
        state.in_flight += num_tasks;
        InFlightDecrement {
            state: &self.state,
            n: num_tasks,
        }
    }
}

struct InFlightDecrement<'a> {
    state: &'a Mutex<State>,
    n: usize,
}

impl Drop for InFlightDecrement<'_> {
    fn drop(&mut self) {
        self.state.lock().in_flight -= self.n;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        panic::AssertUnwindSafe,
        sync::atomic::{AtomicBool, AtomicUsize, Ordering},
        time::Duration,
    };

    fn snapshot(pool: &WorkerPool) -> (usize, usize) {
        let s = pool.state.lock();
        (s.spawned, s.in_flight)
    }

    #[test]
    fn scope_zero_tasks_is_noop() {
        let pool = WorkerPool::new();
        let called = AtomicUsize::new(0);
        pool.scope(0, |_| {
            called.fetch_add(1, Ordering::Relaxed);
        });
        assert_eq!(called.load(Ordering::Relaxed), 0);
        // No worker threads spawned and in_flight stays at zero.
        assert_eq!(snapshot(&pool), (0, 0));
    }

    #[test]
    fn scope_single_task_runs_once_with_index_zero() {
        let pool = WorkerPool::new();
        let seen = Mutex::new(Vec::new());
        pool.scope(1, |i| seen.lock().push(i));
        assert_eq!(*seen.lock(), vec![0]);
    }

    #[test]
    fn scope_calls_work_with_each_index_exactly_once() {
        let pool = WorkerPool::new();
        const N: usize = 8;
        let counts: Vec<AtomicUsize> = (0..N).map(|_| AtomicUsize::new(0)).collect();
        pool.scope(N, |i| {
            counts[i].fetch_add(1, Ordering::Relaxed);
        });
        for c in &counts {
            assert_eq!(c.load(Ordering::Relaxed), 1);
        }
    }

    #[test]
    fn scope_blocks_until_slowest_task_completes() {
        let pool = WorkerPool::new();
        let done = AtomicBool::new(false);
        pool.scope(4, |i| {
            if i == 3 {
                std::thread::sleep(Duration::from_millis(50));
                done.store(true, Ordering::Release);
            }
        });
        // The slowest task must have set `done` by the time scope returns.
        assert!(done.load(Ordering::Acquire));
    }

    #[test]
    fn pool_grows_to_match_demand() {
        let pool = WorkerPool::new();
        assert_eq!(snapshot(&pool), (0, 0));
        pool.scope(4, |_| {});
        let (spawned, in_flight) = snapshot(&pool);
        assert_eq!(spawned, 4);
        assert_eq!(in_flight, 0);
    }

    #[test]
    fn pool_reuses_threads_for_smaller_or_equal_scope() {
        let pool = WorkerPool::new();
        pool.scope(6, |_| {});
        assert_eq!(snapshot(&pool).0, 6);
        // Smaller and equal scopes must not spawn more threads.
        pool.scope(3, |_| {});
        pool.scope(6, |_| {});
        assert_eq!(snapshot(&pool).0, 6);
    }

    #[test]
    fn pool_grows_further_for_larger_scope() {
        let pool = WorkerPool::new();
        pool.scope(2, |_| {});
        assert_eq!(snapshot(&pool).0, 2);
        pool.scope(5, |_| {});
        assert_eq!(snapshot(&pool).0, 5);
    }

    #[test]
    fn in_flight_returns_to_zero_after_scope() {
        let pool = WorkerPool::new();
        pool.scope(4, |_| {});
        assert_eq!(snapshot(&pool).1, 0);
        pool.scope(8, |_| {});
        assert_eq!(snapshot(&pool).1, 0);
    }

    #[test]
    fn panic_in_task_propagates_to_caller() {
        let pool = WorkerPool::new();
        let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
            pool.scope(4, |i| {
                if i == 2 {
                    panic!("boom from task {}", i);
                }
            });
        }));
        let err = result.expect_err("scope should have panicked");
        let msg = err
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| err.downcast_ref::<&'static str>().copied())
            .unwrap_or("");
        assert!(msg.contains("boom"), "unexpected panic payload: {msg:?}");
    }

    #[test]
    fn panic_does_not_skip_other_tasks() {
        // Every non-panicking task must still observe its index — proving the barrier waits for
        // them even when one task aborts.
        let pool = WorkerPool::new();
        const N: usize = 6;
        let observed: Vec<AtomicBool> = (0..N).map(|_| AtomicBool::new(false)).collect();
        let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
            pool.scope(N, |i| {
                observed[i].store(true, Ordering::Release);
                if i == 0 {
                    panic!("only task 0 panics");
                }
            });
        }));
        for (i, flag) in observed.iter().enumerate() {
            assert!(
                flag.load(Ordering::Acquire),
                "task {i} did not get a chance to run"
            );
        }
    }

    #[test]
    fn pool_is_usable_after_a_panicking_scope() {
        let pool = WorkerPool::new();
        let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
            pool.scope(3, |_| panic!("everyone panics"));
        }));
        // After unwinding, in_flight must be back to zero and the next scope must succeed.
        assert_eq!(snapshot(&pool).1, 0);
        let counter = AtomicUsize::new(0);
        pool.scope(5, |_| {
            counter.fetch_add(1, Ordering::Relaxed);
        });
        assert_eq!(counter.load(Ordering::Relaxed), 5);
    }

    #[test]
    fn concurrent_scopes_share_a_pool() {
        // Two threads call `scope` simultaneously; together they need 2*M workers in flight, so
        // the pool must grow to satisfy the combined demand.
        let pool = Arc::new(WorkerPool::new());
        const M: usize = 4;
        let start = Arc::new(Barrier::new(2));
        let entered = Arc::new(AtomicUsize::new(0));
        let release = Arc::new(Barrier::new(2 * M));

        let handles: Vec<_> = (0..2)
            .map(|_| {
                let pool = Arc::clone(&pool);
                let start = Arc::clone(&start);
                let entered = Arc::clone(&entered);
                let release = Arc::clone(&release);
                std::thread::Builder::new()
                    .name("test-driver".into())
                    .spawn(move || {
                        start.wait();
                        pool.scope(M, |_| {
                            entered.fetch_add(1, Ordering::Relaxed);
                            // Hold every worker until all 2*M are running concurrently. Will hang
                            // if the pool fails to grow past M.
                            release.wait();
                        });
                    })
                    .unwrap()
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(entered.load(Ordering::Relaxed), 2 * M);
        let (spawned, in_flight) = snapshot(&pool);
        assert!(
            spawned >= 2 * M,
            "pool must grow to support concurrent demand, spawned={spawned}"
        );
        assert_eq!(in_flight, 0);
    }

    #[test]
    fn tasks_run_in_parallel() {
        // A `Barrier::new(N)` deadlocks unless all N tasks run concurrently.
        let pool = WorkerPool::new();
        const N: usize = 4;
        let rendezvous = Arc::new(Barrier::new(N));
        pool.scope(N, |_| {
            rendezvous.wait();
        });
    }

    #[test]
    fn worker_threads_carry_par_exec_name() {
        let pool = WorkerPool::new();
        let names = Mutex::new(Vec::new());
        pool.scope(3, |_| {
            let name = std::thread::current().name().unwrap_or_default().to_owned();
            names.lock().push(name);
        });
        let names = names.into_inner();
        assert_eq!(names.len(), 3);
        for n in &names {
            assert!(
                n.starts_with("par_exec-"),
                "worker thread name should start with 'par_exec-', got {n:?}"
            );
        }
    }

    #[test]
    fn many_tasks_complete_correctly() {
        let pool = WorkerPool::new();
        const N: usize = 64;
        let sum = AtomicUsize::new(0);
        pool.scope(N, |i| {
            sum.fetch_add(i, Ordering::Relaxed);
        });
        assert_eq!(sum.load(Ordering::Relaxed), (0..N).sum::<usize>());
    }

    #[test]
    fn scope_passes_borrowed_state_through_unsafe_lifetime_extension() {
        // The closure borrows a stack-local `Vec`. If `scope` returned before workers stopped
        // touching it, this would be a use-after-free; running cleanly under Miri/ASAN is the
        // real check, but the assertion verifies the data round-trips.
        let pool = WorkerPool::new();
        let inputs: Vec<usize> = (0..10).map(|i| i * 7).collect();
        let outputs: Vec<AtomicUsize> = (0..inputs.len()).map(|_| AtomicUsize::new(0)).collect();
        pool.scope(inputs.len(), |i| {
            outputs[i].store(inputs[i] + 1, Ordering::Relaxed);
        });
        for (i, out) in outputs.iter().enumerate() {
            assert_eq!(out.load(Ordering::Relaxed), inputs[i] + 1);
        }
    }

    #[test]
    fn back_to_back_scopes_share_the_same_workers() {
        // Spawn count must be monotonic and stable when each scope's demand fits.
        let pool = WorkerPool::new();
        pool.scope(3, |_| {});
        let after_first = snapshot(&pool).0;
        for _ in 0..5 {
            pool.scope(3, |_| {});
        }
        assert_eq!(snapshot(&pool).0, after_first);
    }
}

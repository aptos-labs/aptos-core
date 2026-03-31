// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at
// https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! A lightweight persistent thread pool for BlockSTM parallel execution.
//!
//! Replaces rayon for `par_exec` workers. Threads are reused across block
//! executions and idle at zero CPU cost (blocked on channel recv). No
//! work-stealing overhead.

use once_cell::sync::Lazy;
use std::{
    panic::{self, AssertUnwindSafe},
    sync::{
        mpsc::{self, Sender},
        Mutex,
    },
    thread::{self, JoinHandle},
};

/// Global worker pool for BlockSTM parallel execution.
pub(crate) static WORKER_POOL: Lazy<WorkerPool> = Lazy::new(WorkerPool::new);

/// A persistent thread pool that dispatches work via per-worker channels.
///
/// Threads are spawned on demand and reused across invocations. Idle threads
/// block on their channel receiver, consuming zero CPU.
pub(crate) struct WorkerPool {
    state: Mutex<PoolState>,
}

struct PoolState {
    threads: Vec<JoinHandle<()>>,
    job_txs: Vec<Sender<WorkerJob>>,
}

struct WorkerJob {
    work: Box<dyn FnOnce() + Send>,
    done: Sender<thread::Result<()>>,
}

impl WorkerPool {
    pub(crate) fn new() -> Self {
        Self {
            state: Mutex::new(PoolState {
                threads: Vec::new(),
                job_txs: Vec::new(),
            }),
        }
    }

    /// Run `f(worker_id)` on `num_workers` persistent threads. Blocks until all
    /// workers complete.
    ///
    /// Threads are created on first use and reused across calls. The pool grows
    /// as needed but never shrinks.
    ///
    /// # Panics
    ///
    /// If any worker panics, all workers are still waited on, then the first
    /// panic is re-raised in the calling thread.
    pub(crate) fn run<F>(&self, num_workers: u32, f: F)
    where
        F: Fn(u32) + Send + Sync,
    {
        let n = num_workers as usize;
        let (done_tx, done_rx) = mpsc::channel();

        {
            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());

            // Grow pool if needed. New threads start blocking on recv immediately.
            while state.threads.len() < n {
                let idx = state.threads.len();
                let (tx, rx) = mpsc::channel::<WorkerJob>();
                let handle = thread::Builder::new()
                    .name(format!("par_exec-{}", idx))
                    .spawn(move || {
                        while let Ok(job) = rx.recv() {
                            let result = panic::catch_unwind(AssertUnwindSafe(job.work));
                            let _ = job.done.send(result);
                        }
                    })
                    .expect("Failed to spawn par_exec worker");
                state.job_txs.push(tx);
                state.threads.push(handle);
            }

            // Dispatch jobs. The closure borrows non-'static data from the
            // caller's scope. The transmute is safe because we block below until
            // all workers complete, so borrowed data outlives worker execution.
            // This is the same safety pattern used by rayon::scope and
            // std::thread::scope.
            let f_ref: &(dyn Fn(u32) + Send + Sync) = &f;
            for i in 0..n {
                let worker_id = i as u32;
                let closure: Box<dyn FnOnce() + Send + '_> = Box::new(move || f_ref(worker_id));
                // SAFETY: We wait for all workers to complete before returning,
                // so all data borrowed by f outlives worker execution.
                let closure: Box<dyn FnOnce() + Send + 'static> =
                    unsafe { std::mem::transmute(closure) };
                state.job_txs[i]
                    .send(WorkerJob {
                        work: closure,
                        done: done_tx.clone(),
                    })
                    .expect("Worker thread terminated unexpectedly");
            }
        } // Release state lock before waiting.
        drop(done_tx);

        // Wait for all workers, collecting the first panic if any.
        let mut panic_payload = None;
        for _ in 0..n {
            match done_rx
                .recv()
                .expect("Worker thread terminated unexpectedly")
            {
                Ok(()) => {},
                Err(payload) => {
                    panic_payload.get_or_insert(payload);
                },
            }
        }
        if let Some(payload) = panic_payload {
            panic::resume_unwind(payload);
        }
    }
}

impl Drop for WorkerPool {
    fn drop(&mut self) {
        let state = self.state.get_mut().unwrap_or_else(|e| e.into_inner());
        // Close all job channels, causing workers to exit their recv loop.
        state.job_txs.clear();
        for handle in state.threads.drain(..) {
            let _ = handle.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[test]
    fn basic_dispatch() {
        let pool = WorkerPool::new();
        let counter = AtomicU32::new(0);
        pool.run(4, |_worker_id| {
            counter.fetch_add(1, Ordering::Relaxed);
        });
        assert_eq!(counter.load(Ordering::Relaxed), 4);
    }

    #[test]
    fn correct_worker_ids() {
        let pool = WorkerPool::new();
        let seen = [
            AtomicU32::new(0),
            AtomicU32::new(0),
            AtomicU32::new(0),
            AtomicU32::new(0),
        ];
        pool.run(4, |worker_id| {
            seen[worker_id as usize].store(1, Ordering::Relaxed);
        });
        for s in &seen {
            assert_eq!(s.load(Ordering::Relaxed), 1);
        }
    }

    #[test]
    fn variable_worker_count() {
        let pool = WorkerPool::new();

        let counter = AtomicU32::new(0);
        pool.run(8, |_| {
            counter.fetch_add(1, Ordering::Relaxed);
        });
        assert_eq!(counter.load(Ordering::Relaxed), 8);

        let counter = AtomicU32::new(0);
        pool.run(3, |_| {
            counter.fetch_add(1, Ordering::Relaxed);
        });
        assert_eq!(counter.load(Ordering::Relaxed), 3);

        let counter = AtomicU32::new(0);
        pool.run(8, |_| {
            counter.fetch_add(1, Ordering::Relaxed);
        });
        assert_eq!(counter.load(Ordering::Relaxed), 8);
    }

    #[test]
    #[should_panic(expected = "worker panic")]
    fn panic_propagation() {
        let pool = WorkerPool::new();
        pool.run(4, |worker_id| {
            if worker_id == 2 {
                panic!("worker panic");
            }
        });
    }

    #[test]
    fn scoped_borrows() {
        let pool = WorkerPool::new();
        let data = vec![0u32; 8];
        // Each worker writes to its slot — verifies scoped borrowing works.
        pool.run(8, |worker_id| {
            // SAFETY: each worker writes to a distinct index.
            let ptr = data.as_ptr() as *mut u32;
            unsafe { ptr.add(worker_id as usize).write(worker_id + 1) };
        });
        assert_eq!(data, vec![1, 2, 3, 4, 5, 6, 7, 8]);
    }
}

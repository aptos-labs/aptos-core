// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Per-caller-thread rayon pools for Move native functions that spawn rayon
//! work (directly or transitively, e.g. via `ark_ec` MSM / pairing).
//!
//! Why this exists: block execution runs on the `par_exec` rayon pool. If a
//! native running on a `par_exec` worker invokes code that calls `par_iter` /
//! `rayon::scope` without installing its own pool, the sub-tasks land on
//! `par_exec`'s deques and rayon's `wait_until` work-steals other block-executor
//! jobs onto the same thread. Combined with writer-preferring RwLocks on
//! per-txn scheduler state, this can close into a deadlock.
//!
//! Each calling thread (typically a `par_exec` worker) lazily builds its own
//! private rayon pool on first use and reuses it thereafter. Because the pool
//! is per-thread, concurrent native calls from different `par_exec` workers
//! never queue behind each other for native worker slots.
//!
//! Natives that reach into rayon-using code must wrap the relevant section in
//! [`with_native_rayon`] so the work executes on this isolated pool instead.

use anyhow::{anyhow, Result};
use std::{
    cell::OnceCell,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, OnceLock,
    },
};

/// Fallback used when [`init_native_rayon_pool`] is never called (tests,
/// tooling). Production callers always initialize via the node config.
const DEFAULT_THREADS_PER_POOL: usize = 1;

/// Threads per per-caller native rayon pool. Set once at startup via
/// [`init_native_rayon_pool`]; falls back to [`DEFAULT_THREADS_PER_POOL`]
/// if uninitialized.
static THREADS_PER_POOL: OnceLock<usize> = OnceLock::new();
/// Monotonic id assigned to each per-caller pool, only used for thread naming.
static NEXT_POOL_ID: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    static PER_CALLER_POOL: OnceCell<Arc<rayon::ThreadPool>> = const { OnceCell::new() };
}

fn build_pool() -> Arc<rayon::ThreadPool> {
    let id = NEXT_POOL_ID.fetch_add(1, Ordering::Relaxed);
    let n = THREADS_PER_POOL
        .get()
        .copied()
        .unwrap_or(DEFAULT_THREADS_PER_POOL);
    Arc::new(
        rayon::ThreadPoolBuilder::new()
            .num_threads(n)
            .thread_name(move |i| format!("native-rayon-{}-{}", id, i))
            .build()
            .expect("failed to build native rayon thread pool"),
    )
}

/// Set the number of threads in each per-caller native rayon pool.
///
/// Intended to be called once at process startup from the same site that reads
/// the block-executor concurrency level. Returns an error on subsequent calls
/// — the size is read by every per-caller pool's lazy init and cannot be
/// retroactively applied to pools that have already been built.
pub fn init_native_rayon_pool(threads_per_pool: usize) -> Result<()> {
    THREADS_PER_POOL
        .set(std::cmp::max(1, threads_per_pool))
        .map_err(|_| anyhow!("native rayon pool size already initialized"))
}

/// Run `op` on the calling thread's dedicated native rayon pool.
///
/// Use this to wrap any Move native code path that invokes rayon directly or
/// via a third-party crate (ark_ec, ark_bls12_381, etc.). The call blocks the
/// current thread until `op` completes.
pub fn with_native_rayon<F, R>(op: F) -> R
where
    F: FnOnce() -> R + Send,
    R: Send,
{
    PER_CALLER_POOL.with(|cell| {
        let pool = cell.get_or_init(build_pool).clone();
        pool.install(op)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_runs_on_native_pool() {
        let name = with_native_rayon(|| {
            std::thread::current()
                .name()
                .map(|s| s.to_string())
                .unwrap_or_default()
        });
        assert!(
            name.starts_with("native-rayon-"),
            "expected native-rayon-* thread, got {:?}",
            name
        );
    }

    #[test]
    fn nested_par_iter_stays_on_native_pool() {
        use rayon::prelude::*;

        let outer_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(2)
            .thread_name(|i| format!("test-outer-{}", i))
            .build()
            .unwrap();

        let names = outer_pool.install(|| {
            with_native_rayon(|| {
                (0..64)
                    .into_par_iter()
                    .map(|_| {
                        std::thread::current()
                            .name()
                            .map(|s| s.to_string())
                            .unwrap_or_default()
                    })
                    .collect::<Vec<_>>()
            })
        });

        assert!(
            names.iter().all(|n| n.starts_with("native-rayon-")),
            "found tasks not on native pool: {:?}",
            names
        );
    }

    #[test]
    fn distinct_caller_threads_get_distinct_pools() {
        // Two threads concurrently entering with_native_rayon must end up on
        // different per-caller pools (different `native-rayon-{id}-*` prefix).
        let join_a = std::thread::spawn(|| {
            with_native_rayon(|| {
                std::thread::current()
                    .name()
                    .map(|s| s.to_string())
                    .unwrap_or_default()
            })
        });
        let join_b = std::thread::spawn(|| {
            with_native_rayon(|| {
                std::thread::current()
                    .name()
                    .map(|s| s.to_string())
                    .unwrap_or_default()
            })
        });
        let name_a = join_a.join().unwrap();
        let name_b = join_b.join().unwrap();
        assert!(name_a.starts_with("native-rayon-"));
        assert!(name_b.starts_with("native-rayon-"));
        // Strip the trailing thread-index, compare the per-caller pool prefix.
        let prefix_a = name_a.rsplit_once('-').unwrap().0;
        let prefix_b = name_b.rsplit_once('-').unwrap().0;
        assert_ne!(
            prefix_a, prefix_b,
            "expected distinct pool prefixes, got {} vs {}",
            prefix_a, prefix_b
        );
    }
}

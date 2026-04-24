// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Dedicated rayon thread pool for Move native functions that spawn rayon work
//! (directly or transitively, e.g. via `ark_ec` MSM / pairing or `bulletproofs`).
//!
//! Why this exists: block execution runs on the `par_exec` rayon pool. If a
//! native running on a `par_exec` worker invokes code that calls `par_iter` /
//! `rayon::scope` without installing its own pool, the sub-tasks land on
//! `par_exec`'s deques and rayon's `wait_until` work-steals other block-executor
//! jobs onto the same thread. Combined with writer-preferring RwLocks on
//! per-txn scheduler state, this can close into a deadlock.
//!
//! Natives that reach into rayon-using code must wrap the relevant section in
//! [`with_native_rayon`] so the work executes on this isolated pool instead.

use anyhow::{anyhow, Result};
use once_cell::sync::OnceCell;
use std::sync::Arc;

static NATIVE_RAYON_POOL: OnceCell<Arc<rayon::ThreadPool>> = OnceCell::new();

fn build_pool(num_threads: usize) -> Arc<rayon::ThreadPool> {
    Arc::new(
        rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .thread_name(|i| format!("native-rayon-{}", i))
            .build()
            .expect("failed to build native rayon thread pool"),
    )
}

/// Initialize the process-wide native rayon pool with the given thread count.
///
/// Intended to be called once at process startup from the same site that reads
/// the block-executor concurrency level. Returns an error if the pool is
/// already initialized — it cannot be resized after construction.
pub fn init_native_rayon_pool(num_threads: usize) -> Result<()> {
    let num_threads = std::cmp::max(1, num_threads);
    NATIVE_RAYON_POOL
        .set(build_pool(num_threads))
        .map_err(|_| anyhow!("native rayon pool already initialized"))
}

fn pool() -> &'static rayon::ThreadPool {
    NATIVE_RAYON_POOL.get_or_init(|| build_pool(1))
}

/// Run `op` on the dedicated native rayon pool.
///
/// Use this to wrap any Move native code path that invokes rayon directly or
/// via a third-party crate (ark_ec, ark_bls12_381, etc.). The call blocks the
/// current thread until `op` completes.
pub fn with_native_rayon<F, R>(op: F) -> R
where
    F: FnOnce() -> R + Send,
    R: Send,
{
    pool().install(op)
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
}

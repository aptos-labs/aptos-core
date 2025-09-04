// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use rayon::{ThreadPool, ThreadPoolBuilder};
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::runtime::{Builder, Runtime};

/// The max thread name length before the name will be truncated
/// when it's displayed. Note: the max display length is 15, but
/// we need to leave space for the thread IDs.
const MAX_THREAD_NAME_LENGTH: usize = 12;

/// Returns a tokio runtime with named threads.
/// This is useful for tracking threads when debugging.
pub fn spawn_named_runtime(thread_name: String, num_worker_threads: Option<usize>) -> Runtime {
    spawn_named_runtime_with_start_hook(thread_name, num_worker_threads, || {})
}

pub fn spawn_named_runtime_with_start_hook<F>(
    thread_name: String,
    num_worker_threads: Option<usize>,
    on_thread_start: F,
) -> Runtime
where
    F: Fn() + Send + Sync + 'static,
{
    const MAX_BLOCKING_THREADS: usize = 64;

    // Verify the given name has an appropriate length
    if thread_name.len() > MAX_THREAD_NAME_LENGTH {
        panic!(
            "The given runtime thread name is too long! Max length: {}, given name: {}",
            MAX_THREAD_NAME_LENGTH, thread_name
        );
    }

    // Create the runtime builder
    let atomic_id = AtomicUsize::new(0);
    let thread_name_clone = thread_name.clone();
    let mut builder = Builder::new_multi_thread();
    builder
        .thread_name_fn(move || {
            let id = atomic_id.fetch_add(1, Ordering::SeqCst);
            format!("{}-{}", thread_name_clone, id)
        })
        .on_thread_start(on_thread_start)
        .disable_lifo_slot()
        // Limit concurrent blocking tasks from spawn_blocking(), in case, for example, too many
        // Rest API calls overwhelm the node.
        .max_blocking_threads(MAX_BLOCKING_THREADS)
        .enable_all();
    if let Some(num_worker_threads) = num_worker_threads {
        builder.worker_threads(num_worker_threads);
    }

    // Spawn and return the runtime
    builder.build().unwrap_or_else(|error| {
        panic!(
            "Failed to spawn named runtime! Name: {:?}, Error: {:?}",
            thread_name, error
        )
    })
}

/// Returns a rayon threadpool with threads.
/// This is useful for tracking threads when debugging.
pub fn spawn_rayon_thread_pool(
    thread_name: String,
    num_worker_threads: Option<usize>,
) -> ThreadPool {
    spawn_rayon_thread_pool_with_start_hook(thread_name, num_worker_threads, || {})
}

pub fn spawn_rayon_thread_pool_with_start_hook<F>(
    thread_name: String,
    num_worker_threads: Option<usize>,
    on_thread_start: F,
) -> ThreadPool
where
    F: Fn() + Send + Sync + 'static,
{
    // Verify the given name has an appropriate length
    if thread_name.len() > MAX_THREAD_NAME_LENGTH {
        panic!(
            "The given runtime thread name is too long! Max length: {}, given name: {}",
            MAX_THREAD_NAME_LENGTH, thread_name
        );
    }

    let thread_name_clone = thread_name.clone();
    let mut builder = ThreadPoolBuilder::new()
        .thread_name(move |index| format!("{}-{index}", thread_name_clone))
        .start_handler(move |_| on_thread_start());

    if let Some(num_worker_threads) = num_worker_threads {
        builder = builder.num_threads(num_worker_threads);
    }

    // Spawn and return the threadpool
    builder.build().unwrap_or_else(|error| {
        panic!(
            "Failed to spawn named rayon thread pool! Name: {:?}, Error: {:?}",
            thread_name, error
        )
    })
}

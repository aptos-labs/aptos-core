// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::runtime::{Builder, Runtime};

/// The max thread name length before the name will be truncated
/// when it's displayed. Note: the max display length is 15, but
/// we need to leave space for the thread IDs.
const MAX_THREAD_NAME_LENGTH: usize = 12;

/// Returns a tokio runtime with named threads.
/// This is useful for tracking threads when debugging.
pub fn spawn_named_runtime(thread_name: String, num_worker_threads: Option<usize>) -> Runtime {
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
        .disable_lifo_slot()
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

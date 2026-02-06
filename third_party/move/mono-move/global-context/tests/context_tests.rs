// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for the global context.

use global_context::GlobalContext;
use std::{
    sync::{Arc, Barrier},
    thread,
    time::Duration,
};

#[test]
fn test_different_contexts() {
    let ctx = GlobalContext::new();

    {
        let _guard = ctx
            .execution_context()
            .expect("Execution context must be acquired");
    }
    {
        let _guard = ctx
            .maintenance_context()
            .expect("Maintenance context must be acquired");
    }
    {
        let _guard1 = ctx
            .execution_context()
            .expect("Execution context must be acquired");
        let _guard2 = ctx
            .execution_context()
            .expect("Execution context must be acquired");
        let _guard3 = ctx
            .execution_context()
            .expect("Execution context must be acquired");
    }
}

#[test]
fn test_concurrent_execution_contexts() {
    let ctx = Arc::new(GlobalContext::new());

    let num_threads = 4;
    let barrier = Arc::new(Barrier::new(num_threads));

    let handles: Vec<_> = (0..num_threads)
        .map(|_| {
            let ctx = Arc::clone(&ctx);
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                // Wait for all threads to be ready.
                barrier.wait();

                // All threads should be able to acquire execution context simultaneously.
                let _guard = ctx
                    .execution_context()
                    .expect("Execution context must be acquired");
                thread::sleep(Duration::from_millis(100));
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn test_maintenance_blocks_maintenance() {
    let ctx = Arc::new(GlobalContext::new());
    let barrier = Arc::new(Barrier::new(2));

    let handle = thread::spawn({
        let ctx = Arc::clone(&ctx);
        let barrier = Arc::clone(&barrier);
        move || {
            let _guard = ctx
                .maintenance_context()
                .expect("Maintenance context must be acquired");

            // Signal that we have the lock.
            barrier.wait();
            thread::sleep(Duration::from_millis(2000));
        }
    });

    // Wait for thread 1 to be in maintenance mode.
    barrier.wait();
    thread::sleep(Duration::from_millis(10));
    assert!(ctx.maintenance_context().is_none());

    handle.join().unwrap();
}

#[test]
fn test_maintenance_blocks_execution() {
    let ctx = Arc::new(GlobalContext::new());
    let barrier = Arc::new(Barrier::new(2));

    // Thread 1: Hold execution context for 100ms
    let handle1 = thread::spawn({
        let ctx = Arc::clone(&ctx);
        let barrier = Arc::clone(&barrier);
        move || {
            let _guard = ctx
                .maintenance_context()
                .expect("Maintenance context must be acquired");

            // Signal that we have the lock.
            barrier.wait();
            thread::sleep(Duration::from_millis(2000));
        }
    });

    // Wait for thread 1 to be in maintenance mode.
    barrier.wait();
    thread::sleep(Duration::from_millis(10));
    assert!(ctx.execution_context().is_none());

    handle1.join().unwrap();
}

#[test]
fn test_execution_blocks_maintenance() {
    let num_threads = 4;

    let ctx = Arc::new(GlobalContext::new());
    let barrier = Arc::new(Barrier::new(num_threads + 1)); // +1 for main thread

    // Spawn multiple threads holding execution contexts
    let handles: Vec<_> = (0..num_threads)
        .map(|_| {
            let ctx = Arc::clone(&ctx);
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                let _guard = ctx
                    .execution_context()
                    .expect("Execution context must be acquired");

                // Signal that we have the lock.
                barrier.wait();
                thread::sleep(Duration::from_millis(2000));
            })
        })
        .collect();

    barrier.wait();
    thread::sleep(Duration::from_millis(10));

    assert!(ctx.maintenance_context().is_none());

    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn test_block_execution_simulation() {
    let num_threads = 4;
    let num_iterations = 5;

    let ctx = Arc::new(GlobalContext::new());

    for _ in 0..num_iterations {
        // Execution phase: concurrent execution.
        let handles: Vec<_> = (0..num_threads)
            .map(|_| {
                let ctx = Arc::clone(&ctx);
                thread::spawn(move || {
                    let _guard = ctx.execution_context();
                    thread::sleep(Duration::from_millis(100));
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        // Maintenance phase: single thread with exclusive access.
        let _guard = ctx.maintenance_context();
        thread::sleep(Duration::from_millis(100));
    }
}

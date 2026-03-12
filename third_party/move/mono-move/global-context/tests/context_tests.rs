// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Integration tests for acquiring execution or maintenance guards from global
//! context.

use mono_move_global_context::GlobalContext;
use std::{
    sync::{Arc, Barrier},
    thread,
    time::Duration,
};

#[test]
fn test_contexts() {
    let ctx = GlobalContext::with_num_execution_workers(4);

    {
        let _guard = ctx.try_execution_context(0).unwrap();
    }
    {
        let _guard = ctx.try_maintenance_context().unwrap();
    }
    {
        let _guard1 = ctx.try_execution_context(0).unwrap();
        let _guard2 = ctx.try_execution_context(1).unwrap();
        let _guard3 = ctx.try_execution_context(2).unwrap();
        let _guard4 = ctx.try_execution_context(3).unwrap();
    }
}

#[test]
fn test_concurrent_execution_contexts() {
    let num_threads = 4;

    let ctx = Arc::new(GlobalContext::with_num_execution_workers(num_threads));
    let barrier = Arc::new(Barrier::new(num_threads));

    let handles: Vec<_> = (0..num_threads)
        .map(|worker_id| {
            let ctx: Arc<GlobalContext> = Arc::clone(&ctx);
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                // Wait for all threads to be ready.
                barrier.wait();

                // All threads should be able to acquire execution context simultaneously.
                let _guard = ctx.try_execution_context(worker_id).unwrap();
                thread::sleep(Duration::from_millis(1000));
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn test_maintenance_blocks_maintenance() {
    let num_threads = 2;

    let ctx = Arc::new(GlobalContext::with_num_execution_workers(num_threads));
    let barrier = Arc::new(Barrier::new(num_threads));

    let handle = thread::spawn({
        let ctx: Arc<GlobalContext> = Arc::clone(&ctx);
        let barrier = Arc::clone(&barrier);
        move || {
            let _guard = ctx.try_maintenance_context().unwrap();

            // Signal that we have the lock. Sleep for long enough to ensure
            // the main thread has enough time to try to acquire the guard.
            barrier.wait();
            thread::sleep(Duration::from_millis(2000));
        }
    });

    // Wait for thread 1 to be in maintenance mode.
    barrier.wait();
    thread::sleep(Duration::from_millis(10));
    assert!(ctx.try_maintenance_context().is_none());

    handle.join().unwrap();
}

#[test]
fn test_maintenance_blocks_execution() {
    let num_threads = 2;

    let ctx = Arc::new(GlobalContext::with_num_execution_workers(num_threads));
    let barrier = Arc::new(Barrier::new(num_threads));

    let handle = thread::spawn({
        let ctx: Arc<GlobalContext> = Arc::clone(&ctx);
        let barrier = Arc::clone(&barrier);
        move || {
            let _guard = ctx.try_maintenance_context().unwrap();

            // Signal that we have the lock. Sleep for long enough to ensure
            // the main thread has enough time to try to acquire the guard.
            barrier.wait();
            thread::sleep(Duration::from_millis(2000));
        }
    });

    // Wait for thread 1 to be in maintenance mode.
    barrier.wait();
    thread::sleep(Duration::from_millis(10));
    assert!(ctx.try_execution_context(0).is_none());

    handle.join().unwrap();
}

#[test]
fn test_execution_blocks_maintenance() {
    let num_threads = 4;

    let ctx = Arc::new(GlobalContext::with_num_execution_workers(num_threads));
    let barrier = Arc::new(Barrier::new(num_threads + 1));

    // Spawn multiple threads holding execution guards.
    let handles: Vec<_> = (0..num_threads)
        .map(|worker_id| {
            let ctx: Arc<GlobalContext> = Arc::clone(&ctx);
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                let _guard = ctx.try_execution_context(worker_id).unwrap();

                // Signal that we have the lock.
                barrier.wait();
                thread::sleep(Duration::from_millis(2000));
            })
        })
        .collect();

    barrier.wait();
    thread::sleep(Duration::from_millis(10));
    assert!(ctx.try_maintenance_context().is_none());

    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn test_block_execution_simulation() {
    let num_threads = 4;
    let ctx = Arc::new(GlobalContext::with_num_execution_workers(num_threads));

    for _ in 0..5 {
        // Execution phase: concurrent execution.
        let handles: Vec<_> = (0..num_threads)
            .map(|worker_id| {
                let ctx: Arc<GlobalContext> = Arc::clone(&ctx);
                thread::spawn(move || {
                    let _guard = ctx.try_execution_context(worker_id).unwrap();
                    thread::sleep(Duration::from_millis(100));
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        // Maintenance phase: single thread with exclusive access.
        let _guard = ctx.try_maintenance_context().unwrap();
        thread::sleep(Duration::from_millis(100));
    }
}

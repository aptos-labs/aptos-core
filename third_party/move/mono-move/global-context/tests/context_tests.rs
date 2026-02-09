// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for the global context.

use global_context::{GlobalContext, GlobalContextConfig};
use move_core_types::{account_address::AccountAddress, identifier::Identifier};
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
            .execution_context(0)
            .expect("Execution context must be acquired");
    }
    {
        let _guard = ctx
            .maintenance_context()
            .expect("Maintenance context must be acquired");
    }
    {
        let _guard1 = ctx
            .execution_context(0)
            .expect("Execution context must be acquired");
        let _guard2 = ctx
            .execution_context(1)
            .expect("Execution context must be acquired");
        let _guard3 = ctx
            .execution_context(2)
            .expect("Execution context must be acquired");
    }
}

#[test]
fn test_concurrent_execution_contexts() {
    let ctx = Arc::new(GlobalContext::new());

    let num_threads = 4;
    let barrier = Arc::new(Barrier::new(num_threads));

    let handles: Vec<_> = (0..num_threads)
        .map(|worker_id| {
            let ctx = Arc::clone(&ctx);
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                // Wait for all threads to be ready.
                barrier.wait();

                // All threads should be able to acquire execution context simultaneously.
                // Each thread gets its own arena (worker_id).
                let _guard = ctx
                    .execution_context(worker_id)
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
    assert!(ctx.execution_context(0).is_none());

    handle1.join().unwrap();
}

#[test]
fn test_execution_blocks_maintenance() {
    let num_threads = 4;

    let ctx = Arc::new(GlobalContext::new());
    let barrier = Arc::new(Barrier::new(num_threads + 1)); // +1 for main thread

    // Spawn multiple threads holding execution contexts
    let handles: Vec<_> = (0..num_threads)
        .map(|worker_id| {
            let ctx = Arc::clone(&ctx);
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                let _guard = ctx
                    .execution_context(worker_id)
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
            .map(|worker_id| {
                let ctx = Arc::clone(&ctx);
                thread::spawn(move || {
                    let _guard = ctx.execution_context(worker_id);
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

#[test]
fn test_memory_threshold_and_flush() {
    let ctx = Arc::new(GlobalContext::with_config(GlobalContextConfig {
        memory_threshold_bytes: 1024,
    }));

    {
        let execution_ctx = ctx.execution_context(0).unwrap();
        for i in 0..1024 {
            let addr = AccountAddress::random();
            let name = Identifier::new(format!("module_{}", i)).unwrap();
            execution_ctx.intern_address_name(&addr, &name);
        }
    }

    {
        let mut maintenance_ctx = ctx.maintenance_context().unwrap();

        assert!(maintenance_ctx.interner_arena_allocated_bytes() > 1024);
        assert_eq!(maintenance_ctx.interned_executable_ids_count(), 1024);

        assert!(maintenance_ctx.check_memory_usage());
        assert_eq!(maintenance_ctx.interned_executable_ids_count(), 0);
    }

    {
        let execution_ctx = ctx.execution_context(0).unwrap();
        for _ in 0..100 {
            let name = Identifier::new("test").unwrap();
            execution_ctx.intern_address_name(&AccountAddress::ONE, &name);
        }
    }

    {
        let maintenance_ctx = ctx.maintenance_context().unwrap();
        assert_eq!(maintenance_ctx.interned_executable_ids_count(), 1);
    }
}

#[test]
fn test_arena_exhaustion() {
    // Create a context with only 2 arenas
    let ctx = Arc::new(GlobalContext::with_num_workers(
        2,
        GlobalContextConfig::default(),
    ));

    // Acquire arenas 0 and 1
    let _guard0 = ctx
        .execution_context(0)
        .expect("Arena 0 should be available");
    let _guard1 = ctx
        .execution_context(1)
        .expect("Arena 1 should be available");

    // Attempt to acquire arena 2 (out of bounds)
    assert!(ctx.execution_context(2).is_none());

    // Attempt to acquire arena 100 (way out of bounds)
    assert!(ctx.execution_context(100).is_none());
}

#[test]
fn test_double_acquisition_same_arena() {
    let ctx = Arc::new(GlobalContext::new());

    // Acquire arena 0
    let _guard1 = ctx
        .execution_context(0)
        .expect("First acquisition should succeed");

    // Attempt to acquire arena 0 again (should fail)
    assert!(ctx.execution_context(0).is_none());

    // Drop first guard
    drop(_guard1);

    // Now arena 0 should be available again
    let _guard2 = ctx
        .execution_context(0)
        .expect("Second acquisition should succeed after drop");
}

#[test]
fn test_per_worker_arena_metrics() {
    let ctx = Arc::new(GlobalContext::with_num_workers(
        4,
        GlobalContextConfig::default(),
    ));

    // Allocate different amounts in each worker's arena
    {
        let exec_ctx_0 = ctx.execution_context(0).unwrap();
        for i in 0..10 {
            let addr = AccountAddress::random();
            let name = Identifier::new(format!("module_0_{}", i)).unwrap();
            exec_ctx_0.intern_address_name(&addr, &name);
        }
    }

    {
        let exec_ctx_1 = ctx.execution_context(1).unwrap();
        for i in 0..20 {
            let addr = AccountAddress::random();
            let name = Identifier::new(format!("module_1_{}", i)).unwrap();
            exec_ctx_1.intern_address_name(&addr, &name);
        }
    }

    // Check per-worker arena metrics
    {
        let maintenance_ctx = ctx.maintenance_context().unwrap();

        let arena_0_bytes = maintenance_ctx.interner_arena_allocated_bytes_by_worker(0);
        let arena_1_bytes = maintenance_ctx.interner_arena_allocated_bytes_by_worker(1);
        let arena_2_bytes = maintenance_ctx.interner_arena_allocated_bytes_by_worker(2);
        let arena_3_bytes = maintenance_ctx.interner_arena_allocated_bytes_by_worker(3);

        // Arena 0 and 1 should have allocations
        assert!(arena_0_bytes > 0, "Arena 0 should have allocations");
        assert!(arena_1_bytes > 0, "Arena 1 should have allocations");

        // Arena 1 allocated 2x items, so should have at least as many bytes as arena 0
        // (Note: Due to interning deduplication and alignment, the exact relationship
        // may vary, so we just verify both have allocations)
        assert!(
            arena_1_bytes >= arena_0_bytes,
            "Arena 1 ({} bytes) should have at least as many bytes as Arena 0 ({} bytes)",
            arena_1_bytes,
            arena_0_bytes
        );

        // Arena 2 and 3 should be empty
        assert_eq!(arena_2_bytes, 0, "Arena 2 should be empty");
        assert_eq!(arena_3_bytes, 0, "Arena 3 should be empty");

        // Total should equal sum of all arenas
        let total_bytes = maintenance_ctx.interner_arena_allocated_bytes();
        assert_eq!(
            total_bytes,
            arena_0_bytes + arena_1_bytes + arena_2_bytes + arena_3_bytes
        );
    }
}

#[test]
fn test_concurrent_arena_allocation_no_contention() {
    let ctx = Arc::new(GlobalContext::with_num_workers(
        8,
        GlobalContextConfig::default(),
    ));

    let num_threads = 8;
    let barrier = Arc::new(Barrier::new(num_threads));

    let handles: Vec<_> = (0..num_threads)
        .map(|worker_id| {
            let ctx = Arc::clone(&ctx);
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                // Wait for all threads to be ready
                barrier.wait();

                // Each worker allocates in its own arena (lock-free)
                let exec_ctx = ctx
                    .execution_context(worker_id)
                    .expect("Arena should be available");

                for i in 0..100 {
                    let addr = AccountAddress::random();
                    let name = Identifier::new(format!("module_{}_{}", worker_id, i)).unwrap();
                    exec_ctx.intern_address_name(&addr, &name);
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    // Verify all allocations were successful
    let maintenance_ctx = ctx.maintenance_context().unwrap();
    assert_eq!(maintenance_ctx.interned_executable_ids_count(), 800); // 8 workers * 100 items
}

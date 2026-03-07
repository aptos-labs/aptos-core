// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for the global context.

use global_context::{configs::MaintenanceConfig, GlobalContext};
use move_binary_format::file_format::{
    empty_module, FunctionDefinition, FunctionHandle, FunctionHandleIndex, IdentifierIndex,
    ModuleHandleIndex, SignatureIndex, Visibility,
};
use move_core_types::{
    ability::AbilitySet, account_address::AccountAddress, identifier::Identifier,
    language_storage::TypeTag,
};
use std::{
    sync::{Arc, Barrier},
    thread,
    time::Duration,
};

/// Returns a compiled module that contains one generic function (one type
/// parameter) called `generic_fn`. Used by mono-cache tests.
fn module_with_generic_fn() -> move_binary_format::CompiledModule {
    let mut module = empty_module();
    module
        .identifiers
        .push(Identifier::new("generic_fn").unwrap());
    let fn_name_idx = IdentifierIndex((module.identifiers.len() - 1) as u16);
    module.function_handles.push(FunctionHandle {
        module: ModuleHandleIndex(0),
        name: fn_name_idx,
        parameters: SignatureIndex(0),
        return_: SignatureIndex(0),
        type_parameters: vec![AbilitySet::EMPTY],
        access_specifiers: None,
        attributes: vec![],
    });
    let fn_handle_idx = FunctionHandleIndex((module.function_handles.len() - 1) as u16);
    module.function_defs.push(FunctionDefinition {
        function: fn_handle_idx,
        visibility: Visibility::Public,
        is_entry: false,
        acquires_global_resources: vec![],
        code: None,
    });
    module
}

#[test]
fn test_different_contexts() {
    let ctx = GlobalContext::with_num_workers(1);

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
    let num_threads = 4;

    let ctx = Arc::new(GlobalContext::with_num_workers(num_threads));
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
    let num_threads = 2;

    let ctx = Arc::new(GlobalContext::with_num_workers(num_threads));
    let barrier = Arc::new(Barrier::new(num_threads));

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
    let num_threads = 2;

    let ctx = Arc::new(GlobalContext::with_num_workers(num_threads));
    let barrier = Arc::new(Barrier::new(num_threads));

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

    let ctx = Arc::new(GlobalContext::with_num_workers(num_threads));
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

    let ctx = Arc::new(GlobalContext::with_num_workers(num_threads));

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
    let ctx = Arc::new(GlobalContext::with_num_workers_and_config(
        1,
        MaintenanceConfig {
            max_global_arena_allocated_bytes: 1024,
            ..MaintenanceConfig::default()
        },
    ));

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
    let ctx = Arc::new(GlobalContext::with_num_workers(2));

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
    let ctx = Arc::new(GlobalContext::with_num_workers(1));

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
    let ctx = Arc::new(GlobalContext::with_num_workers(4));

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
    let num_threads = 8;

    let ctx = Arc::new(GlobalContext::with_num_workers(num_threads));
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

// ---------------------------------------------------------------------------
// Monomorphized function cache tests (Phase 3)
// ---------------------------------------------------------------------------

/// `MaintenanceConfig` default includes `mono_eviction_ttl_blocks = 1`.
#[test]
fn test_maintenance_config_default_ttl() {
    let config = MaintenanceConfig::default();
    assert_eq!(config.mono_eviction_ttl_blocks, 1);
    assert_eq!(config.max_monomorphized_functions, 1_000_000);
}

/// When mono count is below the threshold, `on_epoch_end` does not evict.
#[test]
fn test_mono_cache_no_eviction_below_threshold() {
    let module = module_with_generic_fn();
    let ctx = GlobalContext::with_num_workers_and_config(1, MaintenanceConfig {
        max_monomorphized_functions: 100,
        mono_eviction_ttl_blocks: 1,
        ..MaintenanceConfig::default()
    });

    {
        let exec_ctx = ctx.execution_context(0).unwrap();
        let exec = exec_ctx.intern_compiled_module(&module, 0);
        let fn_id = exec_ctx.intern_function_name(&Identifier::new("generic_fn").unwrap());
        let tl = exec_ctx.intern_type_tags(&[TypeTag::U64]);
        // Insert 1 mono entry — well below threshold of 100.
        exec_ctx.get_monomorphized_function(exec, fn_id, tl);
    }

    {
        let mut maint = ctx.maintenance_context().unwrap();
        // Promote cold → hot so the count is visible.
        maint.on_epoch_end();
        // Count should still be 1: no eviction fired.
        assert_eq!(maint.monomorphized_function_count(), 1);
    }
}

/// TTL eviction: entries not touched in `ttl_blocks` blocks are swept.
#[test]
fn test_mono_cache_ttl_eviction() {
    let module = module_with_generic_fn();
    // Use a tiny threshold so eviction fires after inserting 2 entries.
    let ctx = GlobalContext::with_num_workers_and_config(1, MaintenanceConfig {
        max_monomorphized_functions: 1,
        mono_eviction_ttl_blocks: 0, // cutoff == block_idx; evict everything <= block_idx
        ..MaintenanceConfig::default()
    });

    {
        let exec_ctx = ctx.execution_context(0).unwrap();
        let exec = exec_ctx.intern_compiled_module(&module, 0);
        let fn_id = exec_ctx.intern_function_name(&Identifier::new("generic_fn").unwrap());
        // Insert 2 distinct mono entries so we exceed threshold = 1.
        let tl_u64 = exec_ctx.intern_type_tags(&[TypeTag::U64]);
        let tl_bool = exec_ctx.intern_type_tags(&[TypeTag::Bool]);
        exec_ctx.get_monomorphized_function(exec, fn_id, tl_u64);
        exec_ctx.get_monomorphized_function(exec, fn_id, tl_bool);
    }

    {
        let mut maint = ctx.maintenance_context().unwrap();
        // block_idx is 0 at this point; cutoff = 0.saturating_sub(0) = 0.
        // Entries inserted at block 0 have last_used_block = 0 <= 0, so they
        // are evicted.
        maint.on_epoch_end();
        // After eviction (and block advance to 1), count should be 0.
        assert_eq!(maint.monomorphized_function_count(), 0);
    }
}

/// A full flush (`check_memory_usage`) resets the mono counter to 0.
#[test]
fn test_mono_cache_flush_resets_counter() {
    let module = module_with_generic_fn();
    let ctx = GlobalContext::with_num_workers_and_config(1, MaintenanceConfig {
        max_global_arena_allocated_bytes: 1, // force flush immediately
        ..MaintenanceConfig::default()
    });

    {
        let exec_ctx = ctx.execution_context(0).unwrap();
        let exec = exec_ctx.intern_compiled_module(&module, 0);
        let fn_id = exec_ctx.intern_function_name(&Identifier::new("generic_fn").unwrap());
        let tl = exec_ctx.intern_type_tags(&[TypeTag::U64]);
        exec_ctx.get_monomorphized_function(exec, fn_id, tl);
    }

    {
        let mut maint = ctx.maintenance_context().unwrap();
        // Force full flush — this frees all executables and resets mono_total.
        assert!(maint.check_memory_usage());
        assert_eq!(maint.monomorphized_function_count(), 0);
    }
}

/// Concurrent inserts for the same mono key: only one entry is committed;
/// the counter is incremented exactly once, not twice.
#[test]
fn test_mono_cache_concurrent_insert_no_double_count() {
    let module = module_with_generic_fn();
    let ctx = Arc::new(GlobalContext::with_num_workers(2));

    // Intern the module from worker 0 so it is in the cold cache.
    {
        let exec_ctx = ctx.execution_context(0).unwrap();
        exec_ctx.intern_compiled_module(&module, 0);
    }

    // Promote the cold entry to hot so both threads can look it up.
    {
        let mut maint = ctx.maintenance_context().unwrap();
        maint.on_epoch_end();
    }

    let barrier = Arc::new(Barrier::new(2));

    // Both threads call get_monomorphized_function for the same
    // (fn_id, type_list) simultaneously. Only one commit should succeed.
    let handles: Vec<_> = (0..2)
        .map(|worker_id| {
            let ctx = Arc::clone(&ctx);
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                let exec_ctx = ctx.execution_context(worker_id).unwrap();
                let fn_id = exec_ctx.intern_function_name(&Identifier::new("generic_fn").unwrap());
                let tl = exec_ctx.intern_type_tags(&[TypeTag::U64]);
                // The empty_module self-id is (AccountAddress::ZERO, "<SELF>").
                let module_id = exec_ctx.intern_address_name(
                    &AccountAddress::ZERO,
                    &Identifier::new("<SELF>").unwrap(),
                );
                let exec = exec_ctx
                    .get_executable(module_id)
                    .expect("executable must be in hot cache");
                barrier.wait();
                exec_ctx.get_monomorphized_function(exec, fn_id, tl);
            })
        })
        .collect();

    for h in handles {
        h.join().unwrap();
    }

    // Exactly one entry should have been committed (race loser frees its copy).
    let maint = ctx.maintenance_context().unwrap();
    assert_eq!(maint.monomorphized_function_count(), 1);
}

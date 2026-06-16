// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Integration tests for acquiring execution or maintenance guards from global
//! context.

use mono_move_global_context::GlobalContext;
use move_core_types::{account_address::AccountAddress, ident_str};
use std::{
    sync::{Arc, Barrier},
    thread,
    time::Duration,
};

#[test]
fn test_contexts() {
    let mut ctx = GlobalContext::with_num_execution_workers(4);

    {
        let _guard = ctx.try_execution_context(0).unwrap();
    }
    {
        let _guard = ctx.maintenance_context();
    }
    {
        let _guard1 = ctx.try_execution_context(0).unwrap();
        let _guard2 = ctx.try_execution_context(1).unwrap();
        let _guard3 = ctx.try_execution_context(2).unwrap();
        let _guard4 = ctx.try_execution_context(3).unwrap();

        // Arena shard at 0 is already locked.
        assert!(ctx.try_execution_context(0).is_none())
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
fn test_block_execution_simulation() {
    let num_threads = 4;
    let mut ctx = Arc::new(GlobalContext::with_num_execution_workers(num_threads));

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
        let ctx = Arc::get_mut(&mut ctx).unwrap();
        let _guard = ctx.maintenance_context();
        thread::sleep(Duration::from_millis(100));
    }
}

#[test]
fn test_global_arena_reset() {
    let mut ctx = GlobalContext::with_num_execution_workers(1);

    {
        let guard = ctx.try_execution_context(0).unwrap();
        guard.intern_identifier(ident_str!("foo"));
        guard.intern_address_name(&AccountAddress::ZERO, ident_str!("bar"));
    }

    let mut guard = ctx.maintenance_context();
    assert_eq!(guard.interned_identifiers_count(), 2);
    assert_eq!(guard.interned_module_ids_count(), 1);

    guard.reset_arena_pool();
    assert_eq!(guard.interned_identifiers_count(), 0);
    assert_eq!(guard.interned_module_ids_count(), 0);
}

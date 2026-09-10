// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Integration tests for acquiring execution or maintenance guards from global
//! context.

use mono_move_global_context::GlobalContext;
use move_core_types::{account_address::AccountAddress, ident_str};
use std::{
    sync::{Arc, Barrier},
    thread,
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
    let ready = Arc::new(Barrier::new(num_threads));
    let holding = Arc::new(Barrier::new(num_threads));

    let handles: Vec<_> = (0..num_threads)
        .map(|worker_id| {
            let ctx: Arc<GlobalContext> = Arc::clone(&ctx);
            let ready = Arc::clone(&ready);
            let holding = Arc::clone(&holding);
            thread::spawn(move || {
                // Wait for all threads to be ready.
                ready.wait();

                // All threads should be able to acquire execution context simultaneously.
                let guard = ctx.try_execution_context(worker_id);

                // Hold every guard until all threads have one. Every worker must
                // reach this barrier even when acquisition fails, or the others
                // block here forever and the failure becomes a hang.
                holding.wait();
                guard.is_some()
            })
        })
        .collect();

    for handle in handles {
        assert!(
            handle.join().unwrap(),
            "worker could not acquire its context"
        );
    }
}

#[test]
fn test_block_execution_simulation() {
    let num_threads = 4;
    let mut ctx = Arc::new(GlobalContext::with_num_execution_workers(num_threads));

    for _ in 0..5 {
        // Execution phase: every worker holds its guard until all of them do.
        let holding = Arc::new(Barrier::new(num_threads));
        let handles: Vec<_> = (0..num_threads)
            .map(|worker_id| {
                let ctx: Arc<GlobalContext> = Arc::clone(&ctx);
                let holding = Arc::clone(&holding);
                thread::spawn(move || {
                    let guard = ctx.try_execution_context(worker_id);
                    // Reached even on failure; see `test_concurrent_execution_contexts`.
                    holding.wait();
                    guard.is_some()
                })
            })
            .collect();

        for handle in handles {
            assert!(
                handle.join().unwrap(),
                "worker could not acquire its context"
            );
        }

        // Maintenance phase: single thread with exclusive access.
        let ctx = Arc::get_mut(&mut ctx).unwrap();
        let _guard = ctx.maintenance_context();
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

// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Tests for interning of struct IDs.

use global_context::GlobalContext;
use move_core_types::identifier::Identifier;
use parking_lot::Mutex;
use std::{
    collections::HashSet,
    sync::{Arc, Barrier},
    thread,
};

#[test]
fn test_same_struct_name_single_thread() {
    let ctx = GlobalContext::new();
    let execution_ctx = ctx.execution_context(0).unwrap();

    let name = Identifier::new("Account").unwrap();
    let ptr1 = execution_ctx.intern_struct_name(&name);
    let ptr2 = execution_ctx.intern_struct_name(&name);

    assert!(ptr1 == ptr2);
    assert_eq!(ptr1.name(), ptr2.name());
    assert_eq!(ptr1.as_usize(), ptr2.as_usize());
}

#[test]
fn test_different_struct_names_single_thread() {
    let ctx = GlobalContext::new();
    let execution_ctx = ctx.execution_context(0).unwrap();

    let name1 = Identifier::new("Account").unwrap();
    let name2 = Identifier::new("Coin").unwrap();

    let ptr1 = execution_ctx.intern_struct_name(&name1);
    let ptr2 = execution_ctx.intern_struct_name(&name2);

    assert!(ptr1 != ptr2);
    assert_ne!(ptr1.name(), ptr2.name());
    assert_ne!(ptr1.as_usize(), ptr2.as_usize());
}

#[test]
fn test_concurrent_same_struct_name() {
    let ctx = Arc::new(GlobalContext::new());

    let num_threads = 4;
    let barrier = Arc::new(Barrier::new(num_threads));
    let name = Arc::new(Identifier::new("Account").unwrap());

    let addresses = Arc::new(Mutex::new(HashSet::new()));
    let handles: Vec<_> = (0..num_threads)
        .map(|worker_id| {
            let ctx = Arc::clone(&ctx);
            let barrier = Arc::clone(&barrier);
            let name = Arc::clone(&name);
            let addresses = Arc::clone(&addresses);

            thread::spawn(move || {
                let execution_ctx = ctx.execution_context(worker_id).unwrap();

                barrier.wait();
                let ptr = execution_ctx.intern_struct_name(&name);
                addresses.lock().insert(ptr.as_usize());
                ptr.name().to_string()
            })
        })
        .collect();

    let results = handles
        .into_iter()
        .map(|h| h.join().unwrap())
        .collect::<Vec<_>>();
    for name in &results {
        assert_eq!(name, "Account");
    }
    let addresses = Arc::into_inner(addresses).unwrap().into_inner();
    assert_eq!(addresses.len(), 1);
}

#[test]
fn test_concurrent_different_struct_names() {
    let ctx = Arc::new(GlobalContext::new());

    let num_threads = 4;
    let barrier = Arc::new(Barrier::new(num_threads));

    let addresses = Arc::new(Mutex::new(HashSet::new()));
    let handles: Vec<_> = (0..num_threads)
        .map(|thread_id| {
            let ctx = Arc::clone(&ctx);
            let barrier = Arc::clone(&barrier);
            let addresses = Arc::clone(&addresses);

            thread::spawn(move || {
                let execution_ctx = ctx.execution_context(thread_id).unwrap();

                barrier.wait();

                let name = Identifier::new(format!("Struct_{}", thread_id)).unwrap();
                let ptr = execution_ctx.intern_struct_name(&name);
                addresses.lock().insert(ptr.as_usize());
                ptr.name().to_string()
            })
        })
        .collect();

    let results = handles
        .into_iter()
        .map(|h| h.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(results.len(), num_threads);
    for (idx, name) in results.into_iter().enumerate() {
        assert_eq!(name, format!("Struct_{}", idx));
    }
    let addresses = Arc::into_inner(addresses).unwrap().into_inner();
    assert_eq!(addresses.len(), num_threads);
}

#[test]
fn test_struct_and_function_name_collision() {
    let ctx = GlobalContext::new();
    let execution_ctx = ctx.execution_context(0).unwrap();

    let name = Identifier::new("a").unwrap();
    let func_ptr = execution_ctx.intern_function_name(&name);
    let struct_ptr = execution_ctx.intern_struct_name(&name);

    // Should point to same allocation.
    assert_eq!(func_ptr.as_usize(), struct_ptr.as_usize());
    assert_eq!(func_ptr.name(), struct_ptr.name());
}

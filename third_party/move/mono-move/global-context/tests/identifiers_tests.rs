// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests for interning of Move identifiers.

use mono_move_global_context::GlobalContext;
use move_core_types::{ident_str, identifier::Identifier};
use parking_lot::Mutex;
use std::{
    collections::HashSet,
    sync::{Arc, Barrier},
    thread,
};

#[test]
fn test_same_identifiers() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();

    let name1 = guard.intern_identifier(ident_str!("transfer"));
    let name2 = guard.intern_identifier(ident_str!("transfer"));

    assert!(name1 == name2);
    assert_eq!(name1.as_str(), name2.as_str());
}

#[test]
fn test_different_identifiers() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();

    let name1 = guard.intern_identifier(ident_str!("transfer"));
    let name2 = guard.intern_identifier(ident_str!("coin"));

    assert!(name1 != name2);
    assert_ne!(name1.as_str(), name2.as_str());
}

#[test]
fn test_concurrent_same_identifiers() {
    let num_threads = 4;

    let ctx = Arc::new(GlobalContext::with_num_execution_workers(num_threads));
    let barrier = Arc::new(Barrier::new(num_threads));
    let name = Arc::new(ident_str!("foo").to_owned());

    let addresses = Arc::new(Mutex::new(HashSet::new()));
    let handles = (0..num_threads)
        .map(|tid| {
            let ctx = Arc::clone(&ctx);
            let barrier = Arc::clone(&barrier);
            let name = Arc::clone(&name);
            let addresses = Arc::clone(&addresses);

            thread::spawn(move || {
                let execution_ctx = ctx.try_execution_context(tid).unwrap();

                barrier.wait();
                let str = execution_ctx.intern_identifier(&name);
                assert_eq!(str.as_str(), name.as_str());
                addresses.lock().insert(str.raw_address_for_testing());
            })
        })
        .collect::<Vec<_>>();

    for handle in handles {
        handle.join().unwrap();
    }
    let addresses = Arc::into_inner(addresses).unwrap().into_inner();
    assert_eq!(addresses.len(), 1);
}

#[test]
fn test_concurrent_different_identifiers() {
    let num_threads = 4;

    let ctx = Arc::new(GlobalContext::with_num_execution_workers(num_threads));
    let barrier = Arc::new(Barrier::new(num_threads));
    let addresses = Arc::new(Mutex::new(HashSet::new()));

    let handles: Vec<_> = (0..num_threads)
        .map(|tid| {
            let ctx = Arc::clone(&ctx);
            let barrier = Arc::clone(&barrier);
            let addresses = Arc::clone(&addresses);

            thread::spawn(move || {
                let execution_ctx = ctx.try_execution_context(tid).unwrap();

                barrier.wait();

                let name = Identifier::new(format!("name_{}", tid)).unwrap();
                let str = execution_ctx.intern_identifier(&name);
                assert_eq!(str.as_str(), name.as_str());
                addresses.lock().insert(str.raw_address_for_testing());
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }
    let addresses = Arc::into_inner(addresses).unwrap().into_inner();
    assert_eq!(addresses.len(), num_threads);
}

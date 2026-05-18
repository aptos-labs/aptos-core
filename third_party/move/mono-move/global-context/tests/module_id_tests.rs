// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests for interning of module IDs.

use mono_move_global_context::GlobalContext;
use move_core_types::{account_address::AccountAddress, ident_str, language_storage::ModuleId};
use parking_lot::Mutex;
use std::{
    collections::HashSet,
    str::FromStr,
    sync::{Arc, Barrier},
    thread,
};

#[test]
fn test_same_executable_id() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();

    let module_id = ModuleId::new(AccountAddress::ONE, ident_str!("foo").to_owned());

    let id1 = guard.intern_module_id(&module_id);
    let id2 = guard.intern_address_name(&module_id.address, &module_id.name);

    assert!(id1 == id2);
    assert_eq!(id1.address(), id2.address());
    assert_eq!(id1.name(), id2.name());
}

#[test]
fn test_different_executable_ids() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();

    let module_id1 = ModuleId::new(AccountAddress::ONE, ident_str!("foo").to_owned());
    let module_id2 = ModuleId::new(AccountAddress::ONE, ident_str!("bar").to_owned());

    let id1 = guard.intern_module_id(&module_id1);
    let id2 = guard.intern_module_id(&module_id2);

    assert!(id1 != id2);
    assert_ne!(id1.name(), id2.name());

    let id3 = guard.intern_address_name(&AccountAddress::ZERO, ident_str!("bar"));
    assert!(id1 != id3 && id2 != id3);
    assert_ne!(id1.name(), id3.name());
    assert_ne!(id2.address(), id3.address());
}

#[test]
fn test_concurrent_same_executable_ids() {
    let num_threads = 4;

    let ctx = Arc::new(GlobalContext::with_num_execution_workers(num_threads));
    let barrier = Arc::new(Barrier::new(num_threads));
    let module_id = Arc::new(ModuleId::new(
        AccountAddress::ONE,
        ident_str!("foo").to_owned(),
    ));

    let addresses = Arc::new(Mutex::new(HashSet::new()));
    let handles = (0..num_threads)
        .map(|tid| {
            let ctx = Arc::clone(&ctx);
            let barrier = Arc::clone(&barrier);
            let module_id = Arc::clone(&module_id);
            let addresses = Arc::clone(&addresses);

            thread::spawn(move || {
                let execution_ctx = ctx.try_execution_context(tid).unwrap();

                barrier.wait();
                let id = execution_ctx.intern_module_id(&module_id);
                assert_eq!(id.address(), &module_id.address);
                assert_eq!(id.name(), module_id.name.as_str());
                addresses.lock().insert(id.raw_address_for_testing());
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
fn test_concurrent_different_executable_ids() {
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

                let addr = AccountAddress::from_str(&format!("0x{}", tid)).unwrap();
                let id = execution_ctx.intern_address_name(&addr, ident_str!("foo"));
                assert_eq!(id.address(), &addr);
                assert_eq!(id.name(), "foo");
                addresses.lock().insert(id.raw_address_for_testing());
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }
    let addresses = Arc::into_inner(addresses).unwrap().into_inner();
    assert_eq!(addresses.len(), num_threads);
}

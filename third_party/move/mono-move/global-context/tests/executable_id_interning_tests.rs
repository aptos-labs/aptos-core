// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Tests for interning of executable IDs.

use global_context::GlobalContext;
use move_core_types::{
    account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
};
use parking_lot::Mutex;
use std::{
    collections::HashSet,
    sync::{Arc, Barrier},
    thread,
};

#[test]
fn test_same_module_id_single_thread() {
    let ctx = GlobalContext::with_num_workers(1);
    let execution_ctx = ctx.execution_context(0).unwrap();

    let addr = AccountAddress::ONE;
    let name = Identifier::new("test").unwrap();
    let module_id = ModuleId::new(addr, name.clone());

    let ptr1 = execution_ctx.intern_module_id(&module_id);
    let ptr2 = execution_ctx.intern_address_name(&addr, &name);

    assert!(ptr1 == ptr2);
    assert_eq!(ptr1.address(), ptr2.address());
    assert_eq!(ptr1.name(), ptr2.name());
}

#[test]
fn test_different_module_id_single_thread() {
    let ctx = GlobalContext::with_num_workers(1);
    let execution_ctx = ctx.execution_context(0).unwrap();

    let module_id = ModuleId::new(AccountAddress::ONE, Identifier::new("module1").unwrap());
    let ptr1 = execution_ctx.intern_module_id(&module_id);
    let ptr2 = execution_ctx
        .intern_address_name(&AccountAddress::ONE, &Identifier::new("module2").unwrap());

    assert!(ptr1 != ptr2);
    assert_eq!(ptr1.address(), ptr2.address());
    assert_ne!(ptr1.name(), ptr2.name());
}

#[test]
fn test_concurrent_same_module_id() {
    let num_threads = 4;

    let ctx = Arc::new(GlobalContext::with_num_workers(num_threads));
    let barrier = Arc::new(Barrier::new(num_threads));
    let module_id = Arc::new(ModuleId::new(
        AccountAddress::ONE,
        Identifier::new("a").unwrap(),
    ));

    let addresses = Arc::new(Mutex::new(HashSet::new()));
    let handles: Vec<_> = (0..num_threads)
        .map(|worker_id| {
            let ctx = Arc::clone(&ctx);
            let barrier = Arc::clone(&barrier);
            let module_id = Arc::clone(&module_id);
            let addresses = Arc::clone(&addresses);

            thread::spawn(move || {
                let execution_ctx = ctx.execution_context(worker_id).unwrap();

                barrier.wait();
                let ptr = execution_ctx.intern_module_id(&module_id);
                addresses.lock().insert(ptr.as_usize());
                (*ptr.address(), ptr.name().to_string())
            })
        })
        .collect();

    let results = handles
        .into_iter()
        .map(|h| h.join().unwrap())
        .collect::<Vec<_>>();
    for (addr, name) in &results {
        assert_eq!(addr, &AccountAddress::ONE);
        assert_eq!(name, "a");
    }
    let addresses = Arc::into_inner(addresses).unwrap().into_inner();
    assert_eq!(addresses.len(), 1);
}

#[test]
fn test_concurrent_different_module_ids() {
    let num_threads = 4;

    let ctx = Arc::new(GlobalContext::with_num_workers(num_threads));
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

                let module_id = ModuleId::new(
                    AccountAddress::ONE,
                    Identifier::new(format!("module_{}", thread_id)).unwrap(),
                );
                let ptr = execution_ctx.intern_module_id(&module_id);
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
        assert_eq!(name, format!("module_{}", idx));
    }
    let addresses = Arc::into_inner(addresses).unwrap().into_inner();
    assert_eq!(addresses.len(), num_threads);
}

// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Stress tests verifying correctness of type interning under concurrent load.
//!
//! Multiple worker threads race to intern the same or different types and the
//! invariants (pointer equality for same types, distinct pointers for distinct
//! types, stable counts) must all hold.
//!
//! All tests use a [`Barrier`] to synchronise workers so they start interning
//! simultaneously, maximising the chance of exposing races.

use mono_move_global_context::GlobalContext;
use move_core_types::{
    ability::AbilitySet,
    account_address::AccountAddress,
    identifier::Identifier,
    language_storage::{FunctionParamOrReturnTag, FunctionTag, StructTag, TypeTag},
};
use std::{
    sync::{Arc, Barrier, Mutex},
    thread,
};

/// All workers intern the same `Vector(U64)` → single deduplicated entry.
#[test]
fn test_concurrent_same_type_tag_deduped() {
    let num_workers = 4;
    let ctx = Arc::new(GlobalContext::with_num_execution_workers(num_workers));
    let barrier = Arc::new(Barrier::new(num_workers));
    let addrs: Arc<Mutex<Vec<usize>>> = Arc::new(Mutex::new(Vec::new()));

    let handles: Vec<_> = (0..num_workers)
        .map(|worker_id| {
            let ctx: Arc<GlobalContext> = Arc::clone(&ctx);
            let barrier = Arc::clone(&barrier);
            let addrs = Arc::clone(&addrs);
            thread::spawn(move || {
                let guard = ctx.execution_context(worker_id).unwrap();
                barrier.wait();
                let r = guard.intern_type_tag(&TypeTag::Vector(Box::new(TypeTag::U64)));
                addrs.lock().unwrap().push(r.as_raw_ptr() as usize);
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    let addrs = addrs.lock().unwrap();
    assert!(
        addrs.windows(2).all(|w| w[0] == w[1]),
        "Concurrent interning of the same type produced different pointers"
    );

    let maintenance = ctx.maintenance_context().unwrap();
    assert_eq!(maintenance.interned_types_count(), 1);
}

/// Worker _i_ interns a distinct `Vector(Uᵢ)` → all pointers distinct; count = N.
#[test]
fn test_concurrent_distinct_type_tags() {
    let num_workers = 8;
    let tags = [
        TypeTag::Vector(Box::new(TypeTag::Bool)),
        TypeTag::Vector(Box::new(TypeTag::U8)),
        TypeTag::Vector(Box::new(TypeTag::U16)),
        TypeTag::Vector(Box::new(TypeTag::U32)),
        TypeTag::Vector(Box::new(TypeTag::U64)),
        TypeTag::Vector(Box::new(TypeTag::U128)),
        TypeTag::Vector(Box::new(TypeTag::U256)),
        TypeTag::Vector(Box::new(TypeTag::Address)),
    ];

    let ctx = Arc::new(GlobalContext::with_num_execution_workers(num_workers));
    let barrier = Arc::new(Barrier::new(num_workers));
    let addrs: Arc<Mutex<Vec<usize>>> = Arc::new(Mutex::new(Vec::new()));

    let handles: Vec<_> = (0..num_workers)
        .map(|worker_id| {
            let ctx: Arc<GlobalContext> = Arc::clone(&ctx);
            let barrier = Arc::clone(&barrier);
            let addrs = Arc::clone(&addrs);
            let tag = tags[worker_id].clone();
            thread::spawn(move || {
                let guard = ctx.execution_context(worker_id).unwrap();
                barrier.wait();
                let r = guard.intern_type_tag(&tag);
                addrs.lock().unwrap().push(r.as_raw_ptr() as usize);
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    let addrs = addrs.lock().unwrap();
    for i in 0..num_workers {
        for j in (i + 1)..num_workers {
            assert_ne!(
                addrs[i], addrs[j],
                "Workers {i} and {j} produced the same pointer for distinct types"
            );
        }
    }

    let maintenance = ctx.maintenance_context().unwrap();
    assert_eq!(maintenance.interned_types_count(), num_workers);
}

/// All workers call `intern_type_tags([U64, Bool])` concurrently → single
/// deduplicated list entry.
#[test]
fn test_concurrent_same_type_list_deduped() {
    let num_workers = 4;
    let ctx = Arc::new(GlobalContext::with_num_execution_workers(num_workers));
    let barrier = Arc::new(Barrier::new(num_workers));

    let handles: Vec<_> = (0..num_workers)
        .map(|worker_id| {
            let ctx: Arc<GlobalContext> = Arc::clone(&ctx);
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                let guard = ctx.execution_context(worker_id).unwrap();
                barrier.wait();
                guard.intern_type_tags(&[TypeTag::U64, TypeTag::Bool]);
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    let maintenance = ctx.maintenance_context().unwrap();
    assert_eq!(
        maintenance.interned_type_lists_count(),
        1,
        "Concurrent interning of the same type list produced multiple entries"
    );
}

/// All workers intern the same `Function(Value(U64) → [])` → single deduplicated entry.
#[test]
fn test_concurrent_function_types_deduped() {
    let num_workers = 4;
    let function_tag = TypeTag::Function(Box::new(FunctionTag {
        args: vec![FunctionParamOrReturnTag::Value(TypeTag::U64)],
        results: vec![],
        abilities: AbilitySet::EMPTY,
    }));

    let ctx = Arc::new(GlobalContext::with_num_execution_workers(num_workers));
    let barrier = Arc::new(Barrier::new(num_workers));
    let addrs: Arc<Mutex<Vec<usize>>> = Arc::new(Mutex::new(Vec::new()));

    let handles: Vec<_> = (0..num_workers)
        .map(|worker_id| {
            let ctx: Arc<GlobalContext> = Arc::clone(&ctx);
            let barrier = Arc::clone(&barrier);
            let addrs = Arc::clone(&addrs);
            let tag = function_tag.clone();
            thread::spawn(move || {
                let guard = ctx.execution_context(worker_id).unwrap();
                barrier.wait();
                let r = guard.intern_type_tag(&tag);
                addrs.lock().unwrap().push(r.as_raw_ptr() as usize);
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    let addrs = addrs.lock().unwrap();
    assert!(
        addrs.windows(2).all(|w| w[0] == w[1]),
        "Concurrent interning of the same Function type produced different pointers"
    );

    let maintenance = ctx.maintenance_context().unwrap();
    assert_eq!(maintenance.interned_types_count(), 1);
}

/// Each worker interns a mix of primitives and composites; after joining,
/// the composite count equals the number of unique composite types interned.
#[test]
fn test_concurrent_mixed_type_interning() {
    // Each worker interns: Bool (primitive), Vector(U8) (composite), Vector(U16) (composite).
    // All workers intern the same composites, so the final count should be 2.
    let num_workers = 4;
    let ctx = Arc::new(GlobalContext::with_num_execution_workers(num_workers));
    let barrier = Arc::new(Barrier::new(num_workers));

    let handles: Vec<_> = (0..num_workers)
        .map(|worker_id| {
            let ctx: Arc<GlobalContext> = Arc::clone(&ctx);
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                let guard = ctx.execution_context(worker_id).unwrap();
                barrier.wait();
                guard.intern_type_tag(&TypeTag::Bool);
                guard.intern_type_tag(&TypeTag::Vector(Box::new(TypeTag::U8)));
                guard.intern_type_tag(&TypeTag::Vector(Box::new(TypeTag::U16)));
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    let maintenance = ctx.maintenance_context().unwrap();
    assert_eq!(maintenance.interned_types_count(), 2); // Vector(U8) + Vector(U16)
}

/// All workers intern the same `StructTag` → single deduplicated entry across
/// all four interners (types, executable_ids, identifiers, type_lists).
#[test]
fn test_concurrent_same_struct_deduped() {
    let num_workers = 4;
    let struct_tag = TypeTag::Struct(Box::new(StructTag {
        address: AccountAddress::ZERO,
        module: Identifier::new("mymod").unwrap(),
        name: Identifier::new("MyStruct").unwrap(),
        type_args: vec![],
    }));

    let ctx = Arc::new(GlobalContext::with_num_execution_workers(num_workers));
    let barrier = Arc::new(Barrier::new(num_workers));
    let addrs: Arc<Mutex<Vec<usize>>> = Arc::new(Mutex::new(Vec::new()));

    let handles: Vec<_> = (0..num_workers)
        .map(|worker_id| {
            let ctx: Arc<GlobalContext> = Arc::clone(&ctx);
            let barrier = Arc::clone(&barrier);
            let addrs = Arc::clone(&addrs);
            let tag = struct_tag.clone();
            thread::spawn(move || {
                let guard = ctx.execution_context(worker_id).unwrap();
                barrier.wait();
                let r = guard.intern_type_tag(&tag);
                addrs.lock().unwrap().push(r.as_raw_ptr() as usize);
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    let addrs = addrs.lock().unwrap();
    assert!(
        addrs.windows(2).all(|w| w[0] == w[1]),
        "Concurrent interning of the same StructTag produced different pointers"
    );

    let maintenance = ctx.maintenance_context().unwrap();
    assert_eq!(maintenance.interned_types_count(), 1);
    assert_eq!(maintenance.interned_executable_ids_count(), 1);
}

/// All workers intern `Struct<U64>` → the `[U64]` type_args list is interned once.
#[test]
fn test_concurrent_generic_struct_shared_args() {
    let num_workers = 4;
    let struct_tag = TypeTag::Struct(Box::new(StructTag {
        address: AccountAddress::ZERO,
        module: Identifier::new("mymod").unwrap(),
        name: Identifier::new("Generic").unwrap(),
        type_args: vec![TypeTag::U64],
    }));

    let ctx = Arc::new(GlobalContext::with_num_execution_workers(num_workers));
    let barrier = Arc::new(Barrier::new(num_workers));

    let handles: Vec<_> = (0..num_workers)
        .map(|worker_id| {
            let ctx: Arc<GlobalContext> = Arc::clone(&ctx);
            let barrier = Arc::clone(&barrier);
            let tag = struct_tag.clone();
            thread::spawn(move || {
                let guard = ctx.execution_context(worker_id).unwrap();
                barrier.wait();
                guard.intern_type_tag(&tag);
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    let maintenance = ctx.maintenance_context().unwrap();
    assert_eq!(maintenance.interned_type_lists_count(), 1);
    assert_eq!(maintenance.interned_types_count(), 1);
}

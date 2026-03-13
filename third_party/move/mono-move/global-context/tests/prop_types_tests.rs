// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Property-based tests for type interning invariants.
//!
//! Uses proptest to verify that interning invariants hold for arbitrary types.
//! Two complementary approaches:
//!
//! - **Sequential**: single worker, standard proptest closures.
//! - **Concurrent**: threads are spawned inside proptest closures to verify
//!   deduplication under race conditions on random inputs.
//!
//! `TypeTag`'s built-in `any::<TypeTag>()` only covers a subset of primitives,
//! so `primitive_type_tag_strategy` and `composite_type_tag_strategy` are
//! written here to cover all 15 primitives and all composite variants.

use mono_move_global_context::GlobalContext;
use move_core_types::{
    ability::AbilitySet,
    account_address::AccountAddress,
    identifier::Identifier,
    language_storage::{FunctionParamOrReturnTag, FunctionTag, StructTag, TypeTag},
};
use proptest::{collection::vec, prelude::*};
use std::{
    sync::{Arc, Barrier, Mutex},
    thread,
};

fn primitive_type_tag_strategy() -> impl Strategy<Value = TypeTag> {
    prop_oneof![
        Just(TypeTag::Bool),
        Just(TypeTag::U8),
        Just(TypeTag::U16),
        Just(TypeTag::U32),
        Just(TypeTag::U64),
        Just(TypeTag::U128),
        Just(TypeTag::U256),
        Just(TypeTag::I8),
        Just(TypeTag::I16),
        Just(TypeTag::I32),
        Just(TypeTag::I64),
        Just(TypeTag::I128),
        Just(TypeTag::I256),
        Just(TypeTag::Address),
        Just(TypeTag::Signer),
    ]
}

/// Simple, non-recursive `StructTag` strategy. Uses a fixed address/module so
/// struct identity is controlled solely by the proptest-generated name.
fn struct_tag_strategy() -> impl Strategy<Value = StructTag> {
    any::<Identifier>().prop_map(|name| StructTag {
        address: AccountAddress::ZERO,
        module: Identifier::new("testmod").unwrap(),
        name,
        type_args: vec![],
    })
}

fn function_tag_strategy() -> impl Strategy<Value = FunctionTag> {
    (
        primitive_type_tag_strategy(),
        any::<bool>(),
        any::<AbilitySet>(),
    )
        .prop_map(|(prim, has_arg, abilities)| FunctionTag {
            args: if has_arg {
                vec![FunctionParamOrReturnTag::Value(prim)]
            } else {
                vec![]
            },
            results: vec![],
            abilities,
        })
}

fn composite_type_tag_strategy() -> impl Strategy<Value = TypeTag> {
    prop_oneof![
        primitive_type_tag_strategy().prop_map(|p| TypeTag::Vector(Box::new(p))),
        function_tag_strategy().prop_map(|f| TypeTag::Function(Box::new(f))),
        struct_tag_strategy().prop_map(|s| TypeTag::Struct(Box::new(s))),
    ]
}

proptest! {
    /// Interning a primitive twice returns the same `Ref`.
    #[test]
    fn prop_primitive_intern_idempotent(prim in primitive_type_tag_strategy()) {
        let ctx = GlobalContext::with_num_execution_workers(1);
        let guard = ctx.execution_context(0).unwrap();
        let r1 = guard.intern_type_tag(&prim);
        let r2 = guard.intern_type_tag(&prim);
        prop_assert!(r1 == r2);
    }

    /// Primitives are backed by statics — they never touch the types interner.
    #[test]
    fn prop_primitive_bypasses_interner(prim in primitive_type_tag_strategy()) {
        let ctx = GlobalContext::with_num_execution_workers(1);
        let guard = ctx.execution_context(0).unwrap();
        guard.intern_type_tag(&prim);
        drop(guard);
        let m = ctx.maintenance_context().unwrap();
        prop_assert_eq!(m.interned_types_count(), 0);
    }

    /// Interning `Vector(P)` twice returns the same `Ref`.
    #[test]
    fn prop_vector_intern_idempotent(prim in primitive_type_tag_strategy()) {
        let ctx = GlobalContext::with_num_execution_workers(1);
        let guard = ctx.execution_context(0).unwrap();
        let tag = TypeTag::Vector(Box::new(prim));
        let r1 = guard.intern_type_tag(&tag);
        let r2 = guard.intern_type_tag(&tag);
        prop_assert!(r1 == r2);
    }

    /// First intern of a composite creates an entry; re-intern does not grow the count.
    #[test]
    fn prop_composite_enters_interner_once(tag in composite_type_tag_strategy()) {
        let ctx = GlobalContext::with_num_execution_workers(1);

        let guard = ctx.execution_context(0).unwrap();
        guard.intern_type_tag(&tag);
        drop(guard);

        let m = ctx.maintenance_context().unwrap();
        let count_after_first = m.interned_types_count();
        prop_assume!(count_after_first >= 1);
        drop(m);

        let guard = ctx.execution_context(0).unwrap();
        guard.intern_type_tag(&tag);
        drop(guard);

        let m = ctx.maintenance_context().unwrap();
        prop_assert_eq!(m.interned_types_count(), count_after_first);
    }

    /// `intern_type_tags` on the same primitive list is idempotent.
    #[test]
    fn prop_type_tag_list_idempotent(tags in vec(primitive_type_tag_strategy(), 0..8)) {
        let ctx = GlobalContext::with_num_execution_workers(1);
        let guard = ctx.execution_context(0).unwrap();
        let lr1 = guard.intern_type_tags(&tags);
        let lr2 = guard.intern_type_tags(&tags);
        prop_assert!(lr1 == lr2);
    }

    /// A `FunctionTag` interned twice returns the same `Ref`; the count is stable.
    #[test]
    fn prop_function_type_idempotent(args in vec(primitive_type_tag_strategy(), 0..4)) {
        let function_tag = TypeTag::Function(Box::new(FunctionTag {
            args: args
                .iter()
                .map(|t| FunctionParamOrReturnTag::Value(t.clone()))
                .collect(),
            results: vec![],
            abilities: AbilitySet::EMPTY,
        }));

        let ctx = GlobalContext::with_num_execution_workers(1);
        let guard = ctx.execution_context(0).unwrap();
        let r1 = guard.intern_type_tag(&function_tag);
        let r2 = guard.intern_type_tag(&function_tag);
        prop_assert!(r1 == r2);

        drop(guard);
        let m = ctx.maintenance_context().unwrap();
        let count = m.interned_types_count();
        prop_assert!(count >= 1);
        drop(m);

        let guard = ctx.execution_context(0).unwrap();
        guard.intern_type_tag(&function_tag);
        drop(guard);

        let m = ctx.maintenance_context().unwrap();
        prop_assert_eq!(m.interned_types_count(), count);
    }

    /// Two structurally distinct primitives produce distinct `Ref`s.
    #[test]
    fn prop_different_primitive_types_distinct(
        a in primitive_type_tag_strategy(),
        b in primitive_type_tag_strategy(),
    ) {
        prop_assume!(a != b);
        let ctx = GlobalContext::with_num_execution_workers(1);
        let guard = ctx.execution_context(0).unwrap();
        let ra = guard.intern_type_tag(&a);
        let rb = guard.intern_type_tag(&b);
        prop_assert!(ra != rb);
    }

    /// Any `StructTag` interned twice produces the same `Ref`.
    #[test]
    fn prop_struct_tag_idempotent(tag in struct_tag_strategy()) {
        let ctx = GlobalContext::with_num_execution_workers(1);
        let guard = ctx.execution_context(0).unwrap();
        let r1 = guard.intern_type_tag(&TypeTag::Struct(Box::new(tag.clone())));
        let r2 = guard.intern_type_tag(&TypeTag::Struct(Box::new(tag)));
        prop_assert!(r1 == r2);
        drop(guard);
        let m = ctx.maintenance_context().unwrap();
        prop_assert_eq!(m.interned_types_count(), 1);
    }

    /// Any `StructTag` intern touches all four interners.
    #[test]
    fn prop_struct_tag_enters_all_interners(tag in struct_tag_strategy()) {
        let ctx = GlobalContext::with_num_execution_workers(1);
        let guard = ctx.execution_context(0).unwrap();
        guard.intern_type_tag(&TypeTag::Struct(Box::new(tag)));
        drop(guard);
        let m = ctx.maintenance_context().unwrap();
        prop_assert!(m.interned_executable_ids_count() >= 1);
        prop_assert!(m.interned_identifiers_count() >= 1);
        prop_assert!(m.interned_types_count() >= 1);
        prop_assert!(m.interned_type_lists_count() >= 1);
    }

    /// Two `StructTag`s with distinct names produce distinct `Ref`s.
    #[test]
    fn prop_distinct_struct_tags_distinct_refs(
        name_a in any::<Identifier>(),
        name_b in any::<Identifier>(),
    ) {
        prop_assume!(name_a != name_b);
        let ctx = GlobalContext::with_num_execution_workers(1);
        let guard = ctx.execution_context(0).unwrap();

        let ra = guard.intern_type_tag(&TypeTag::Struct(Box::new(StructTag {
            address: AccountAddress::ZERO,
            module: Identifier::new("testmod").unwrap(),
            name: name_a,
            type_args: vec![],
        })));
        let rb = guard.intern_type_tag(&TypeTag::Struct(Box::new(StructTag {
            address: AccountAddress::ZERO,
            module: Identifier::new("testmod").unwrap(),
            name: name_b,
            type_args: vec![],
        })));

        prop_assert!(ra != rb);
    }

    /// N workers simultaneously intern the same composite type → all addresses
    /// equal; count is stable on re-intern.
    #[test]
    fn prop_concurrent_composite_deduped(tag in composite_type_tag_strategy()) {
        let num_workers = 4;
        let ctx = Arc::new(GlobalContext::with_num_execution_workers(num_workers));
        let barrier = Arc::new(Barrier::new(num_workers));
        let addrs: Arc<Mutex<Vec<usize>>> = Arc::new(Mutex::new(Vec::new()));

        let handles: Vec<_> = (0..num_workers)
            .map(|worker_id| {
                let ctx: Arc<GlobalContext> = Arc::clone(&ctx);
                let barrier = Arc::clone(&barrier);
                let addrs = Arc::clone(&addrs);
                let tag = tag.clone();
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
        prop_assert!(
            addrs.windows(2).all(|w| w[0] == w[1]),
            "Concurrent interning produced different pointers for the same type"
        );

        // Re-interning must not grow the count.
        let m = ctx.maintenance_context().unwrap();
        let count = m.interned_types_count();
        prop_assert!(count >= 1);
        drop(m);

        let guard = ctx.execution_context(0).unwrap();
        guard.intern_type_tag(&tag);
        drop(guard);

        let m = ctx.maintenance_context().unwrap();
        prop_assert_eq!(m.interned_types_count(), count);
    }

    /// N workers simultaneously intern the same `StructTag` → same address;
    /// `executable_ids` count = 1.
    #[test]
    fn prop_concurrent_struct_deduped(tag in struct_tag_strategy()) {
        let num_workers = 4;
        let ctx = Arc::new(GlobalContext::with_num_execution_workers(num_workers));
        let barrier = Arc::new(Barrier::new(num_workers));
        let addrs: Arc<Mutex<Vec<usize>>> = Arc::new(Mutex::new(Vec::new()));

        let handles: Vec<_> = (0..num_workers)
            .map(|worker_id| {
                let ctx: Arc<GlobalContext> = Arc::clone(&ctx);
                let barrier = Arc::clone(&barrier);
                let addrs = Arc::clone(&addrs);
                let tag = tag.clone();
                thread::spawn(move || {
                    let guard = ctx.execution_context(worker_id).unwrap();
                    barrier.wait();
                    let r = guard.intern_type_tag(&TypeTag::Struct(Box::new(tag)));
                    addrs.lock().unwrap().push(r.as_raw_ptr() as usize);
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        let addrs = addrs.lock().unwrap();
        prop_assert!(
            addrs.windows(2).all(|w| w[0] == w[1]),
            "Concurrent StructTag interning produced different pointers"
        );

        let m = ctx.maintenance_context().unwrap();
        prop_assert_eq!(m.interned_types_count(), 1);
        prop_assert_eq!(m.interned_executable_ids_count(), 1);
    }

    /// N workers simultaneously call `intern_type_tags` on the same list →
    /// single deduplicated list entry.
    #[test]
    fn prop_concurrent_type_list_deduped(
        tags in vec(primitive_type_tag_strategy(), 1..6)
    ) {
        let num_workers = 4;
        let ctx = Arc::new(GlobalContext::with_num_execution_workers(num_workers));
        let barrier = Arc::new(Barrier::new(num_workers));

        let handles: Vec<_> = (0..num_workers)
            .map(|worker_id| {
                let ctx: Arc<GlobalContext> = Arc::clone(&ctx);
                let barrier = Arc::clone(&barrier);
                let tags = tags.clone();
                thread::spawn(move || {
                    let guard = ctx.execution_context(worker_id).unwrap();
                    barrier.wait();
                    guard.intern_type_tags(&tags);
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        let m = ctx.maintenance_context().unwrap();
        prop_assert_eq!(
            m.interned_type_lists_count(),
            1,
            "Concurrent intern_type_tags produced multiple list entries for the same list"
        );
    }
}

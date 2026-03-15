// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Integration tests for interning of Move types.

use mono_move_global_context::{
    maintenance_config::{MaintenanceConfig, TypeTreeSizeLimits},
    GlobalContext, TypeError, TypeTreeSize,
};
use move_core_types::{
    ability::{Ability, AbilitySet},
    account_address::AccountAddress,
    ident_str,
    language_storage::{FunctionParamOrReturnTag, FunctionTag, StructTag, TypeTag},
};
use parking_lot::Mutex;
use proptest::prelude::*;
use std::{
    collections::HashSet,
    str::FromStr,
    sync::{Arc, Barrier},
    thread,
};

#[test]
fn test_primitive_types() {
    let all_primitives = [
        TypeTag::Bool,
        TypeTag::U8,
        TypeTag::U16,
        TypeTag::U32,
        TypeTag::U64,
        TypeTag::U128,
        TypeTag::U256,
        TypeTag::I8,
        TypeTag::I16,
        TypeTag::I32,
        TypeTag::I64,
        TypeTag::I128,
        TypeTag::I256,
        TypeTag::Address,
        TypeTag::Signer,
    ];

    let ctx = GlobalContext::with_num_execution_workers(1);

    // Intern from two separate guards; static-backed pointers must be identical.
    let addrs1: Vec<usize> = {
        let guard = ctx.try_execution_context(0).unwrap();
        all_primitives
            .iter()
            .map(|tag| {
                guard
                    .intern_type_tag(tag)
                    .unwrap()
                    .raw_address_for_testing()
            })
            .collect()
    };
    let addrs2: Vec<usize> = {
        let guard = ctx.try_execution_context(0).unwrap();
        all_primitives
            .iter()
            .map(|tag| {
                guard
                    .intern_type_tag(tag)
                    .unwrap()
                    .raw_address_for_testing()
            })
            .collect()
    };

    assert_eq!(
        addrs1, addrs2,
        "Primitive type pointers must be identical across guards (static-backed)"
    );

    // All 15 raw addresses must be pairwise distinct.
    let unique: HashSet<usize> = addrs1.iter().copied().collect();
    assert_eq!(
        unique.len(),
        15,
        "All 15 primitive types must have distinct addresses"
    );
}

// ── 2b. Empty type list is static ────────────────────────────────────────────

#[test]
fn test_empty_type_list_is_static() {
    let ctx = GlobalContext::with_num_execution_workers(1);

    let addr1 = {
        let guard = ctx.try_execution_context(0).unwrap();
        guard
            .intern_type_tags(&[])
            .unwrap()
            .raw_address_for_testing()
    };
    let addr2 = {
        let guard = ctx.try_execution_context(0).unwrap();
        guard
            .intern_type_tags(&[])
            .unwrap()
            .raw_address_for_testing()
    };
    assert_eq!(
        addr1, addr2,
        "intern_type_tags(&[]) must return the same static pointer"
    );

    let addr3 = {
        let guard = ctx.try_execution_context(0).unwrap();
        guard
            .intern_function_param_or_return_type_tags(&[])
            .unwrap()
            .raw_address_for_testing()
    };
    let addr4 = {
        let guard = ctx.try_execution_context(0).unwrap();
        guard
            .intern_function_param_or_return_type_tags(&[])
            .unwrap()
            .raw_address_for_testing()
    };
    assert_eq!(
        addr3, addr4,
        "intern_function_param_or_return_type_tags(&[]) must return the same static pointer"
    );

    // Both empty-list functions share the same backing static.
    assert_eq!(
        addr1, addr3,
        "Both empty type-list intern functions must return the same static pointer"
    );
}

// ── 2c. Composite type canonicalization is idempotent ────────────────────────

#[test]
fn test_composite_type_idempotent() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();

    // vector<u8> interned twice → same pointer.
    let vec_u8 = TypeTag::Vector(Box::new(TypeTag::U8));
    let r1 = guard
        .intern_type_tag(&vec_u8)
        .unwrap()
        .raw_address_for_testing();
    let r2 = guard
        .intern_type_tag(&vec_u8)
        .unwrap()
        .raw_address_for_testing();
    assert_eq!(
        r1, r2,
        "vector<u8> must intern to the same address each time"
    );

    // Struct interned twice → same pointer.
    let struct_tag = TypeTag::Struct(Box::new(StructTag {
        address: AccountAddress::ONE,
        module: ident_str!("foo").to_owned(),
        name: ident_str!("Bar").to_owned(),
        type_args: vec![],
    }));
    let r3 = guard
        .intern_type_tag(&struct_tag)
        .unwrap()
        .raw_address_for_testing();
    let r4 = guard
        .intern_type_tag(&struct_tag)
        .unwrap()
        .raw_address_for_testing();
    assert_eq!(
        r3, r4,
        "Struct type must intern to the same address each time"
    );

    // Non-empty type list interned twice → same pointer.
    let tags = [TypeTag::Bool, TypeTag::U8];
    let l1 = guard
        .intern_type_tags(&tags)
        .unwrap()
        .raw_address_for_testing();
    let l2 = guard
        .intern_type_tags(&tags)
        .unwrap()
        .raw_address_for_testing();
    assert_eq!(
        l1, l2,
        "Non-empty type list must intern to the same address each time"
    );
}

// ── 2d. Nested type sharing ───────────────────────────────────────────────────

#[test]
fn test_nested_type_sharing() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();

    let vec_u8 = TypeTag::Vector(Box::new(TypeTag::U8));
    let vec_vec_u8 = TypeTag::Vector(Box::new(TypeTag::Vector(Box::new(TypeTag::U8))));

    // Intern the inner type first, record its canonical address.
    let addr_a = guard
        .intern_type_tag(&vec_u8)
        .unwrap()
        .raw_address_for_testing();

    // Intern the outer type (which recursively re-interns the inner).
    let _ = guard.intern_type_tag(&vec_vec_u8).unwrap();

    // The inner type's canonical pointer must be unchanged.
    let addr_a_again = guard
        .intern_type_tag(&vec_u8)
        .unwrap()
        .raw_address_for_testing();
    assert_eq!(
        addr_a, addr_a_again,
        "vector<u8> canonical address must be stable after interning vector<vector<u8>>"
    );
}

// ── 2e. Different types → different pointers ─────────────────────────────────

#[test]
fn test_different_types_have_distinct_pointers() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();

    let vec_bool = guard
        .intern_type_tag(&TypeTag::Vector(Box::new(TypeTag::Bool)))
        .unwrap()
        .raw_address_for_testing();
    let vec_u8 = guard
        .intern_type_tag(&TypeTag::Vector(Box::new(TypeTag::U8)))
        .unwrap()
        .raw_address_for_testing();
    assert_ne!(
        vec_bool, vec_u8,
        "vector<bool> and vector<u8> must have distinct addresses"
    );

    // Struct distinction: vary address, module, and name independently.
    let s_base = guard
        .intern_type_tag(&TypeTag::Struct(Box::new(StructTag {
            address: AccountAddress::ONE,
            module: ident_str!("mod_a").to_owned(),
            name: ident_str!("T").to_owned(),
            type_args: vec![],
        })))
        .unwrap()
        .raw_address_for_testing();
    // Different address, same module + name.
    let s_diff_addr = guard
        .intern_type_tag(&TypeTag::Struct(Box::new(StructTag {
            address: AccountAddress::TWO,
            module: ident_str!("mod_a").to_owned(),
            name: ident_str!("T").to_owned(),
            type_args: vec![],
        })))
        .unwrap()
        .raw_address_for_testing();
    // Same address, different module, same name.
    let s_diff_mod = guard
        .intern_type_tag(&TypeTag::Struct(Box::new(StructTag {
            address: AccountAddress::ONE,
            module: ident_str!("mod_b").to_owned(),
            name: ident_str!("T").to_owned(),
            type_args: vec![],
        })))
        .unwrap()
        .raw_address_for_testing();
    // Same address + module, different struct name.
    let s_diff_name = guard
        .intern_type_tag(&TypeTag::Struct(Box::new(StructTag {
            address: AccountAddress::ONE,
            module: ident_str!("mod_a").to_owned(),
            name: ident_str!("U").to_owned(),
            type_args: vec![],
        })))
        .unwrap()
        .raw_address_for_testing();
    assert_ne!(
        s_base, s_diff_addr,
        "Different addresses must produce distinct struct pointers"
    );
    assert_ne!(
        s_base, s_diff_mod,
        "Different module names must produce distinct struct pointers"
    );
    assert_ne!(
        s_base, s_diff_name,
        "Different struct names must produce distinct struct pointers"
    );
    let s1 = s_base;
    let s2 = s_diff_addr;
    assert_ne!(s1, s2, "Distinct StructTags must have distinct addresses");

    let list1 = guard
        .intern_type_tags(&[TypeTag::Bool])
        .unwrap()
        .raw_address_for_testing();
    let list2 = guard
        .intern_type_tags(&[TypeTag::U8])
        .unwrap()
        .raw_address_for_testing();
    assert_ne!(
        list1, list2,
        "Distinct non-empty type lists must have distinct addresses"
    );
}

// ── 2f. FunctionParamOrReturnTag transparency and reference variants ──────────

#[test]
fn test_function_param_or_return_tag() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();

    // Value(Bool) is transparent: same pointer as intern_type_tag(&Bool).
    let bool_addr = guard
        .intern_type_tag(&TypeTag::Bool)
        .unwrap()
        .raw_address_for_testing();
    let value_bool = FunctionParamOrReturnTag::Value(TypeTag::Bool);
    let value_bool_addr = guard
        .intern_function_param_or_return_type_tag(&value_bool)
        .unwrap()
        .raw_address_for_testing();
    assert_eq!(
        bool_addr, value_bool_addr,
        "Value(Bool) must be transparent — same pointer as Bool"
    );

    // Reference(U8) → pointer distinct from U8.
    let u8_addr = guard
        .intern_type_tag(&TypeTag::U8)
        .unwrap()
        .raw_address_for_testing();
    let ref_u8 = FunctionParamOrReturnTag::Reference(TypeTag::U8);
    let ref_u8_addr = guard
        .intern_function_param_or_return_type_tag(&ref_u8)
        .unwrap()
        .raw_address_for_testing();
    assert_ne!(ref_u8_addr, u8_addr, "Reference(U8) must differ from U8");

    // MutableReference(U8) → distinct from both Reference(U8) and U8.
    let mut_ref_u8 = FunctionParamOrReturnTag::MutableReference(TypeTag::U8);
    let mut_ref_u8_addr = guard
        .intern_function_param_or_return_type_tag(&mut_ref_u8)
        .unwrap()
        .raw_address_for_testing();
    assert_ne!(
        mut_ref_u8_addr, ref_u8_addr,
        "MutableReference(U8) must differ from Reference(U8)"
    );
    assert_ne!(
        mut_ref_u8_addr, u8_addr,
        "MutableReference(U8) must differ from U8"
    );

    // Same FunctionParamOrReturnTag slice interned twice → same pointer.
    let tags = [FunctionParamOrReturnTag::Reference(TypeTag::Bool)];
    let l1 = guard
        .intern_function_param_or_return_type_tags(&tags)
        .unwrap()
        .raw_address_for_testing();
    let l2 = guard
        .intern_function_param_or_return_type_tags(&tags)
        .unwrap()
        .raw_address_for_testing();
    assert_eq!(
        l1, l2,
        "FunctionParamOrReturnTag slice must intern idempotently"
    );
}

// ── 2g. Arena reset clears type caches ───────────────────────────────────────

#[test]
fn test_arena_reset_clears_type_caches() {
    let ctx = GlobalContext::with_num_execution_workers(1);

    {
        let guard = ctx.try_execution_context(0).unwrap();
        guard
            .intern_type_tag(&TypeTag::Vector(Box::new(TypeTag::U8)))
            .unwrap();
        guard
            .intern_type_tag(&TypeTag::Struct(Box::new(StructTag {
                address: AccountAddress::ONE,
                module: ident_str!("m").to_owned(),
                name: ident_str!("T").to_owned(),
                type_args: vec![],
            })))
            .unwrap();
        guard
            .intern_type_tags(&[TypeTag::Bool, TypeTag::U8])
            .unwrap();
    }

    {
        let mut maintenance = ctx.try_maintenance_context().unwrap();
        // Exactly 2 composite types (Vector, Struct) and 1 explicit type list ([Bool, U8]).
        // Primitives are static — they do not appear in the map. The Struct's empty type_args
        // reuse the static EMPTY_TYPE_LIST and do not add an entry either.
        assert_eq!(
            maintenance.interned_types_count(),
            2,
            "Expected exactly 2 interned types (Vector(U8) and Struct) before reset"
        );
        assert_eq!(
            maintenance.interned_type_lists_count(),
            1,
            "Expected exactly 1 interned type list ([Bool, U8]) before reset"
        );

        maintenance.reset_arena_pool();

        assert_eq!(
            maintenance.interned_types_count(),
            0,
            "interned_types_count must be 0 after reset"
        );
        assert_eq!(
            maintenance.interned_type_lists_count(),
            0,
            "interned_type_lists_count must be 0 after reset"
        );
    }

    // After reset, re-interning must work without panicking, be idempotent,
    // and return non-null (non-zero) pointers.
    {
        let guard = ctx.try_execution_context(0).unwrap();

        let r1 = guard
            .intern_type_tag(&TypeTag::Vector(Box::new(TypeTag::U8)))
            .unwrap()
            .raw_address_for_testing();
        let r2 = guard
            .intern_type_tag(&TypeTag::Vector(Box::new(TypeTag::U8)))
            .unwrap()
            .raw_address_for_testing();
        assert!(
            r1 > 0,
            "Re-interned Vector(U8) after reset must be a non-null address"
        );
        assert_eq!(r1, r2, "Re-interning after reset must still be idempotent");

        let l1 = guard
            .intern_type_tags(&[TypeTag::Bool, TypeTag::U8])
            .unwrap()
            .raw_address_for_testing();
        let l2 = guard
            .intern_type_tags(&[TypeTag::Bool, TypeTag::U8])
            .unwrap()
            .raw_address_for_testing();
        assert!(
            l1 > 0,
            "Re-interned type list after reset must be a non-null address"
        );
        assert_eq!(
            l1, l2,
            "Re-interning type list after reset must still be idempotent"
        );

        // Exercise the full recursive Function re-interning path.
        let func = TypeTag::Function(Box::new(FunctionTag {
            args: vec![FunctionParamOrReturnTag::Value(TypeTag::U8)],
            results: vec![FunctionParamOrReturnTag::Value(TypeTag::Bool)],
            abilities: AbilitySet::EMPTY,
        }));
        let f1 = guard
            .intern_type_tag(&func)
            .unwrap()
            .raw_address_for_testing();
        let f2 = guard
            .intern_type_tag(&func)
            .unwrap()
            .raw_address_for_testing();
        assert!(
            f1 > 0,
            "Re-interned Function after reset must be a non-null address"
        );
        assert_eq!(
            f1, f2,
            "Re-interning Function after reset must be idempotent"
        );
    }
}

// ── 2h. Concurrent same-type interning converges to one canonical pointer ────

#[test]
fn test_concurrent_same_type_converges() {
    let num_threads = 4;
    let ctx = Arc::new(GlobalContext::with_num_execution_workers(num_threads));
    let barrier = Arc::new(Barrier::new(num_threads));
    let struct_tag = Arc::new(TypeTag::Struct(Box::new(StructTag {
        address: AccountAddress::ONE,
        module: ident_str!("foo").to_owned(),
        name: ident_str!("Bar").to_owned(),
        type_args: vec![],
    })));
    let addresses = Arc::new(Mutex::new(HashSet::new()));

    let handles: Vec<_> = (0..num_threads)
        .map(|tid| {
            let ctx = Arc::clone(&ctx);
            let barrier = Arc::clone(&barrier);
            let struct_tag = Arc::clone(&struct_tag);
            let addresses = Arc::clone(&addresses);
            thread::spawn(move || {
                let guard = ctx.try_execution_context(tid).unwrap();
                barrier.wait();
                let r = guard.intern_type_tag(&struct_tag).unwrap();
                addresses.lock().insert(r.raw_address_for_testing());
            })
        })
        .collect();

    for h in handles {
        h.join().unwrap();
    }
    let addrs = Arc::into_inner(addresses).unwrap().into_inner();
    assert_eq!(
        addrs.len(),
        1,
        "All threads must converge to a single canonical pointer"
    );
}

// ── 2i. Concurrent different-type interning produces distinct pointers ────────

#[test]
fn test_concurrent_different_types_produce_distinct_pointers() {
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
                let guard = ctx.try_execution_context(tid).unwrap();
                let addr = AccountAddress::from_str(&format!("0x{}", tid + 1)).unwrap();
                let tag = TypeTag::Struct(Box::new(StructTag {
                    address: addr,
                    module: ident_str!("foo").to_owned(),
                    name: ident_str!("T").to_owned(),
                    type_args: vec![],
                }));
                barrier.wait();
                let r = guard.intern_type_tag(&tag).unwrap();
                addresses.lock().insert(r.raw_address_for_testing());
            })
        })
        .collect();

    for h in handles {
        h.join().unwrap();
    }
    let addrs = Arc::into_inner(addresses).unwrap().into_inner();
    assert_eq!(
        addrs.len(),
        num_threads,
        "Each distinct struct type must have a distinct canonical pointer"
    );
}

// ── 2j. Proptest — idempotency for arbitrary TypeTag ─────────────────────────

proptest! {
    #[test]
    fn prop_intern_type_tag_idempotent(ty in any::<TypeTag>()) {
        let ctx = GlobalContext::with_num_execution_workers(1);
        let guard = ctx.try_execution_context(0).unwrap();
        let r1 = guard.intern_type_tag(&ty).unwrap();
        let r2 = guard.intern_type_tag(&ty).unwrap();
        prop_assert_eq!(r1.raw_address_for_testing(), r2.raw_address_for_testing());
    }

    // ── 2k. Proptest — two distinct TypeTags → different interned pointers ───

    #[test]
    fn prop_distinct_type_tags_produce_distinct_pointers(
        ty1 in any::<TypeTag>(),
        ty2 in any::<TypeTag>(),
    ) {
        prop_assume!(ty1 != ty2);
        let ctx = GlobalContext::with_num_execution_workers(1);
        let guard = ctx.try_execution_context(0).unwrap();
        prop_assert_ne!(
            guard.intern_type_tag(&ty1).unwrap().raw_address_for_testing(),
            guard.intern_type_tag(&ty2).unwrap().raw_address_for_testing(),
        );
    }

    // ── 2l. Proptest — intern_type_tags idempotency for arbitrary Vec<TypeTag> ─

    #[test]
    fn prop_intern_type_tags_idempotent(
        tys in proptest::collection::vec(any::<TypeTag>(), 0..8),
    ) {
        let ctx = GlobalContext::with_num_execution_workers(1);
        let guard = ctx.try_execution_context(0).unwrap();
        let l1 = guard.intern_type_tags(&tys).unwrap();
        let l2 = guard.intern_type_tags(&tys).unwrap();
        prop_assert_eq!(l1.raw_address_for_testing(), l2.raw_address_for_testing());
    }
}

// ── 2m. Proptest — concurrent same-type converges (arbitrary type) ───────────

proptest! {
    #[test]
    fn prop_concurrent_intern_same_type_converges(ty in any::<TypeTag>()) {
        let num_threads = 4;
        let ctx = Arc::new(GlobalContext::with_num_execution_workers(num_threads));
        let barrier = Arc::new(Barrier::new(num_threads));
        let ty = Arc::new(ty);
        let addresses = Arc::new(Mutex::new(HashSet::new()));

        let handles: Vec<_> = (0..num_threads)
            .map(|tid| {
                let ctx = Arc::clone(&ctx);
                let barrier = Arc::clone(&barrier);
                let ty = Arc::clone(&ty);
                let addresses = Arc::clone(&addresses);
                thread::spawn(move || {
                    let guard = ctx.try_execution_context(tid).unwrap();
                    barrier.wait();
                    let r = guard.intern_type_tag(&ty).unwrap();
                    addresses.lock().insert(r.raw_address_for_testing());
                })
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }
        let addrs = Arc::into_inner(addresses).unwrap().into_inner();
        prop_assert_eq!(addrs.len(), 1);
    }

    // ── 2n. Proptest — concurrent same type-list converges (arbitrary list) ──

    #[test]
    fn prop_concurrent_intern_same_type_list_converges(
        tys in proptest::collection::vec(any::<TypeTag>(), 0..8),
    ) {
        let num_threads = 4;
        let ctx = Arc::new(GlobalContext::with_num_execution_workers(num_threads));
        let barrier = Arc::new(Barrier::new(num_threads));
        let tys = Arc::new(tys);
        let addresses = Arc::new(Mutex::new(HashSet::new()));

        let handles: Vec<_> = (0..num_threads)
            .map(|tid| {
                let ctx = Arc::clone(&ctx);
                let barrier = Arc::clone(&barrier);
                let tys = Arc::clone(&tys);
                let addresses = Arc::clone(&addresses);
                thread::spawn(move || {
                    let guard = ctx.try_execution_context(tid).unwrap();
                    barrier.wait();
                    let r = guard.intern_type_tags(&tys).unwrap();
                    addresses.lock().insert(r.raw_address_for_testing());
                })
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }
        let addrs = Arc::into_inner(addresses).unwrap().into_inner();
        prop_assert_eq!(addrs.len(), 1);
    }
}

// ── Gap-filling deterministic tests ──────────────────────────────────────────

// ── Function type: idempotency, ability-set distinction, arg distinction ─────

#[test]
fn test_function_type_interning() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();

    let func = TypeTag::Function(Box::new(FunctionTag {
        args: vec![FunctionParamOrReturnTag::Value(TypeTag::U8)],
        results: vec![FunctionParamOrReturnTag::Value(TypeTag::Bool)],
        abilities: AbilitySet::EMPTY,
    }));

    // Idempotency.
    let r1 = guard
        .intern_type_tag(&func)
        .unwrap()
        .raw_address_for_testing();
    let r2 = guard
        .intern_type_tag(&func)
        .unwrap()
        .raw_address_for_testing();
    assert_eq!(r1, r2, "TypeTag::Function must intern idempotently");

    // Same signature, different AbilitySet → distinct pointer.
    let func_copy = TypeTag::Function(Box::new(FunctionTag {
        args: vec![FunctionParamOrReturnTag::Value(TypeTag::U8)],
        results: vec![FunctionParamOrReturnTag::Value(TypeTag::Bool)],
        abilities: AbilitySet::singleton(Ability::Copy),
    }));
    let r3 = guard
        .intern_type_tag(&func_copy)
        .unwrap()
        .raw_address_for_testing();
    assert_ne!(
        r1, r3,
        "Function types with different AbilitySets must have distinct addresses"
    );

    // Same AbilitySet, different arg type → distinct pointer.
    let func_u64_arg = TypeTag::Function(Box::new(FunctionTag {
        args: vec![FunctionParamOrReturnTag::Value(TypeTag::U64)],
        results: vec![FunctionParamOrReturnTag::Value(TypeTag::Bool)],
        abilities: AbilitySet::EMPTY,
    }));
    let r4 = guard
        .intern_type_tag(&func_u64_arg)
        .unwrap()
        .raw_address_for_testing();
    assert_ne!(
        r1, r4,
        "Function types with different args must have distinct addresses"
    );

    // Same AbilitySet, different result type → distinct pointer.
    let func_u8_result = TypeTag::Function(Box::new(FunctionTag {
        args: vec![FunctionParamOrReturnTag::Value(TypeTag::U8)],
        results: vec![FunctionParamOrReturnTag::Value(TypeTag::U8)],
        abilities: AbilitySet::EMPTY,
    }));
    let r5 = guard
        .intern_type_tag(&func_u8_result)
        .unwrap()
        .raw_address_for_testing();
    assert_ne!(
        r1, r5,
        "Function types with different results must have distinct addresses"
    );

    // Zero-arg, zero-result function (edge case for empty type lists).
    let func_unit = TypeTag::Function(Box::new(FunctionTag {
        args: vec![],
        results: vec![],
        abilities: AbilitySet::EMPTY,
    }));
    let r6 = guard
        .intern_type_tag(&func_unit)
        .unwrap()
        .raw_address_for_testing();
    let r7 = guard
        .intern_type_tag(&func_unit)
        .unwrap()
        .raw_address_for_testing();
    assert_eq!(
        r6, r7,
        "Zero-arg zero-result function must intern idempotently"
    );
    assert_ne!(r1, r6, "Non-empty-arg and zero-arg functions must differ");

    // Function type must differ from a structurally unrelated Vector type.
    let vec_u8 = guard
        .intern_type_tag(&TypeTag::Vector(Box::new(TypeTag::U8)))
        .unwrap()
        .raw_address_for_testing();
    assert_ne!(r1, vec_u8, "Function type must differ from Vector type");

    // Function type must differ from a Struct (different discriminant).
    let struct_addr = guard
        .intern_type_tag(&TypeTag::Struct(Box::new(StructTag {
            address: AccountAddress::ONE,
            module: ident_str!("foo").to_owned(),
            name: ident_str!("T").to_owned(),
            type_args: vec![],
        })))
        .unwrap()
        .raw_address_for_testing();
    assert_ne!(
        r1, struct_addr,
        "Function type must differ from Struct type"
    );

    // AbilitySet::ALL vs EMPTY — both with the same args/results.
    let func_all_abilities = TypeTag::Function(Box::new(FunctionTag {
        args: vec![FunctionParamOrReturnTag::Value(TypeTag::U8)],
        results: vec![FunctionParamOrReturnTag::Value(TypeTag::Bool)],
        abilities: AbilitySet::ALL,
    }));
    let r_all = guard
        .intern_type_tag(&func_all_abilities)
        .unwrap()
        .raw_address_for_testing();
    assert_eq!(
        r_all,
        guard
            .intern_type_tag(&func_all_abilities)
            .unwrap()
            .raw_address_for_testing(),
        "Function with ALL abilities must intern idempotently"
    );
    assert_ne!(
        r1, r_all,
        "Function with ALL abilities must differ from one with EMPTY abilities"
    );
}

// ── Signed integer types used as Vector element ───────────────────────────────

#[test]
fn test_signed_integer_types_in_vector() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();

    let vec_i32 = TypeTag::Vector(Box::new(TypeTag::I32));
    let vec_u32 = TypeTag::Vector(Box::new(TypeTag::U32));
    let vec_i64 = TypeTag::Vector(Box::new(TypeTag::I64));
    let vec_i128 = TypeTag::Vector(Box::new(TypeTag::I128));

    let r_i32 = guard
        .intern_type_tag(&vec_i32)
        .unwrap()
        .raw_address_for_testing();
    let r_u32 = guard
        .intern_type_tag(&vec_u32)
        .unwrap()
        .raw_address_for_testing();
    let r_i64 = guard
        .intern_type_tag(&vec_i64)
        .unwrap()
        .raw_address_for_testing();
    let r_i128 = guard
        .intern_type_tag(&vec_i128)
        .unwrap()
        .raw_address_for_testing();

    // Idempotency.
    assert_eq!(
        r_i32,
        guard
            .intern_type_tag(&vec_i32)
            .unwrap()
            .raw_address_for_testing()
    );

    // Signed vs unsigned with the same bit-width must differ.
    assert_ne!(r_i32, r_u32, "vector<i32> must differ from vector<u32>");

    // Different signed widths must differ.
    assert_ne!(r_i32, r_i64, "vector<i32> must differ from vector<i64>");
    assert_ne!(r_i64, r_i128, "vector<i64> must differ from vector<i128>");
}

// ── FunctionParamOrReturnTag: multi-element slice, mixed kinds, ordering ──────

#[test]
fn test_function_param_tags_multi_element_and_ordering() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();

    let mixed = [
        FunctionParamOrReturnTag::Reference(TypeTag::U8),
        FunctionParamOrReturnTag::Value(TypeTag::U64),
        FunctionParamOrReturnTag::MutableReference(TypeTag::Bool),
    ];

    // Idempotency for a multi-element, mixed-kind slice.
    let l1 = guard
        .intern_function_param_or_return_type_tags(&mixed)
        .unwrap()
        .raw_address_for_testing();
    let l2 = guard
        .intern_function_param_or_return_type_tags(&mixed)
        .unwrap()
        .raw_address_for_testing();
    assert_eq!(
        l1, l2,
        "Multi-element mixed-kind slice must intern idempotently"
    );

    // Changing the kind of one element produces a distinct list.
    let changed_kind = [
        FunctionParamOrReturnTag::Value(TypeTag::U8), // was Reference
        FunctionParamOrReturnTag::Value(TypeTag::U64),
        FunctionParamOrReturnTag::MutableReference(TypeTag::Bool),
    ];
    let l3 = guard
        .intern_function_param_or_return_type_tags(&changed_kind)
        .unwrap()
        .raw_address_for_testing();
    assert_ne!(l1, l3, "Changing tag kind must produce a distinct list");

    // Reversed order must produce a distinct list.
    let reversed = [
        FunctionParamOrReturnTag::MutableReference(TypeTag::Bool),
        FunctionParamOrReturnTag::Value(TypeTag::U64),
        FunctionParamOrReturnTag::Reference(TypeTag::U8),
    ];
    let l4 = guard
        .intern_function_param_or_return_type_tags(&reversed)
        .unwrap()
        .raw_address_for_testing();
    assert_ne!(l1, l4, "Reversed slice must produce a distinct list");

    // Length distinction: [Ref(U8)] must differ from [Ref(U8), Val(Bool)].
    let one_elem = [FunctionParamOrReturnTag::Reference(TypeTag::U8)];
    let two_elem = [
        FunctionParamOrReturnTag::Reference(TypeTag::U8),
        FunctionParamOrReturnTag::Value(TypeTag::Bool),
    ];
    let l5 = guard
        .intern_function_param_or_return_type_tags(&one_elem)
        .unwrap()
        .raw_address_for_testing();
    let l6 = guard
        .intern_function_param_or_return_type_tags(&two_elem)
        .unwrap()
        .raw_address_for_testing();
    assert_ne!(l5, l6, "[Ref(U8)] must differ from [Ref(U8), Val(Bool)]");
}

// ── Struct with non-empty type_args ──────────────────────────────────────────

#[test]
fn test_struct_with_non_empty_type_args() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();

    let mk_foo = |type_args: Vec<TypeTag>| {
        TypeTag::Struct(Box::new(StructTag {
            address: AccountAddress::ONE,
            module: ident_str!("foo").to_owned(),
            name: ident_str!("Foo").to_owned(),
            type_args,
        }))
    };

    let foo_no_args = mk_foo(vec![]);
    let foo_u8 = mk_foo(vec![TypeTag::U8]);
    let foo_u64 = mk_foo(vec![TypeTag::U64]);
    let foo_u8_u64 = mk_foo(vec![TypeTag::U8, TypeTag::U64]);

    let r0 = guard
        .intern_type_tag(&foo_no_args)
        .unwrap()
        .raw_address_for_testing();
    let r1 = guard
        .intern_type_tag(&foo_u8)
        .unwrap()
        .raw_address_for_testing();
    let r2 = guard
        .intern_type_tag(&foo_u64)
        .unwrap()
        .raw_address_for_testing();
    let r3 = guard
        .intern_type_tag(&foo_u8_u64)
        .unwrap()
        .raw_address_for_testing();

    // Idempotency.
    assert_eq!(
        r1,
        guard
            .intern_type_tag(&foo_u8)
            .unwrap()
            .raw_address_for_testing()
    );

    // Different type_args → different pointers.
    assert_ne!(r0, r1, "Foo<> and Foo<U8> must differ");
    assert_ne!(r1, r2, "Foo<U8> and Foo<U64> must differ");
    assert_ne!(r1, r3, "Foo<U8> and Foo<U8,U64> must differ");
    assert_ne!(r2, r3, "Foo<U64> and Foo<U8,U64> must differ");
}

// ── Type-list ordering invariant ──────────────────────────────────────────────

#[test]
fn test_type_list_ordering_matters() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();

    let bool_u8 = guard
        .intern_type_tags(&[TypeTag::Bool, TypeTag::U8])
        .unwrap()
        .raw_address_for_testing();
    let u8_bool = guard
        .intern_type_tags(&[TypeTag::U8, TypeTag::Bool])
        .unwrap()
        .raw_address_for_testing();
    assert_ne!(
        bool_u8, u8_bool,
        "[Bool, U8] and [U8, Bool] must have distinct addresses"
    );

    // Same for FunctionParamOrReturnTag lists.
    let ref_val = [
        FunctionParamOrReturnTag::Reference(TypeTag::U8),
        FunctionParamOrReturnTag::Value(TypeTag::Bool),
    ];
    let val_ref = [
        FunctionParamOrReturnTag::Value(TypeTag::Bool),
        FunctionParamOrReturnTag::Reference(TypeTag::U8),
    ];
    let l1 = guard
        .intern_function_param_or_return_type_tags(&ref_val)
        .unwrap()
        .raw_address_for_testing();
    let l2 = guard
        .intern_function_param_or_return_type_tags(&val_ref)
        .unwrap()
        .raw_address_for_testing();
    assert_ne!(
        l1, l2,
        "Reversed FunctionParamOrReturnTag lists must differ"
    );
}

// ── Type-list with duplicate elements ────────────────────────────────────────

#[test]
fn test_type_list_with_duplicate_elements() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();

    let bool_bool = guard
        .intern_type_tags(&[TypeTag::Bool, TypeTag::Bool])
        .unwrap()
        .raw_address_for_testing();
    let bool_u8 = guard
        .intern_type_tags(&[TypeTag::Bool, TypeTag::U8])
        .unwrap()
        .raw_address_for_testing();
    let bool_single = guard
        .intern_type_tags(&[TypeTag::Bool])
        .unwrap()
        .raw_address_for_testing();

    // Idempotency for duplicate-element list.
    assert_eq!(
        bool_bool,
        guard
            .intern_type_tags(&[TypeTag::Bool, TypeTag::Bool])
            .unwrap()
            .raw_address_for_testing()
    );

    // [Bool, Bool] ≠ [Bool, U8] and ≠ [Bool].
    assert_ne!(
        bool_bool, bool_u8,
        "[Bool, Bool] must differ from [Bool, U8]"
    );
    assert_ne!(
        bool_bool, bool_single,
        "[Bool, Bool] must differ from [Bool]"
    );
}

// ── Length-1 list is distinct from the empty list ────────────────────────────

#[test]
fn test_length_one_list_distinct_from_empty() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();

    let empty = guard
        .intern_type_tags(&[])
        .unwrap()
        .raw_address_for_testing();
    let one = guard
        .intern_type_tags(&[TypeTag::Bool])
        .unwrap()
        .raw_address_for_testing();

    assert_ne!(
        empty, one,
        "Empty list and length-1 list must have distinct addresses"
    );
}

// ── Reference and MutableReference of composite types ────────────────────────

#[test]
fn test_reference_of_composite_types() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();

    // Reference(Vector(U8)).
    let ref_vec_u8 = FunctionParamOrReturnTag::Reference(TypeTag::Vector(Box::new(TypeTag::U8)));
    let r1 = guard
        .intern_function_param_or_return_type_tag(&ref_vec_u8)
        .unwrap()
        .raw_address_for_testing();
    let r1_again = guard
        .intern_function_param_or_return_type_tag(&ref_vec_u8)
        .unwrap()
        .raw_address_for_testing();
    assert_eq!(
        r1, r1_again,
        "Reference(Vector(U8)) must intern idempotently"
    );

    // Reference(Vector(U8)) ≠ Reference(U8).
    let ref_u8 = FunctionParamOrReturnTag::Reference(TypeTag::U8);
    let r2 = guard
        .intern_function_param_or_return_type_tag(&ref_u8)
        .unwrap()
        .raw_address_for_testing();
    assert_ne!(
        r1, r2,
        "Reference(Vector(U8)) must differ from Reference(U8)"
    );

    // Reference(Vector(U8)) ≠ Vector(U8).
    let vec_u8_addr = guard
        .intern_type_tag(&TypeTag::Vector(Box::new(TypeTag::U8)))
        .unwrap()
        .raw_address_for_testing();
    assert_ne!(
        r1, vec_u8_addr,
        "Reference(Vector(U8)) must differ from Vector(U8)"
    );

    // MutableReference(Struct(...)).
    let struct_tag = StructTag {
        address: AccountAddress::ONE,
        module: ident_str!("foo").to_owned(),
        name: ident_str!("T").to_owned(),
        type_args: vec![],
    };
    let mut_ref_struct =
        FunctionParamOrReturnTag::MutableReference(TypeTag::Struct(Box::new(struct_tag.clone())));
    let ref_struct =
        FunctionParamOrReturnTag::Reference(TypeTag::Struct(Box::new(struct_tag.clone())));

    let r3 = guard
        .intern_function_param_or_return_type_tag(&mut_ref_struct)
        .unwrap()
        .raw_address_for_testing();
    let r4 = guard
        .intern_function_param_or_return_type_tag(&ref_struct)
        .unwrap()
        .raw_address_for_testing();

    assert_eq!(
        r3,
        guard
            .intern_function_param_or_return_type_tag(&mut_ref_struct)
            .unwrap()
            .raw_address_for_testing(),
        "MutableReference(Struct) must intern idempotently"
    );
    assert_ne!(
        r3, r4,
        "MutableReference(Struct) must differ from Reference(Struct)"
    );
}

// ── Deeply nested types ───────────────────────────────────────────────────────

#[test]
fn test_deeply_nested_types() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();

    let struct_tag = TypeTag::Struct(Box::new(StructTag {
        address: AccountAddress::ONE,
        module: ident_str!("foo").to_owned(),
        name: ident_str!("T").to_owned(),
        type_args: vec![],
    }));

    // Vector(Struct(...)).
    let vec_struct = TypeTag::Vector(Box::new(struct_tag.clone()));
    let r1 = guard
        .intern_type_tag(&vec_struct)
        .unwrap()
        .raw_address_for_testing();
    let r1_again = guard
        .intern_type_tag(&vec_struct)
        .unwrap()
        .raw_address_for_testing();
    assert_eq!(r1, r1_again, "Vector(Struct) must intern idempotently");

    // Vector(Struct) ≠ the inner Struct alone.
    let r_struct = guard
        .intern_type_tag(&struct_tag)
        .unwrap()
        .raw_address_for_testing();
    assert_ne!(r1, r_struct, "Vector(Struct) must differ from Struct");

    // Struct<Vector<U8>>: struct whose type_arg is a composite type.
    let struct_with_vec_arg = TypeTag::Struct(Box::new(StructTag {
        address: AccountAddress::ONE,
        module: ident_str!("foo").to_owned(),
        name: ident_str!("Wrapper").to_owned(),
        type_args: vec![TypeTag::Vector(Box::new(TypeTag::U8))],
    }));
    let r2 = guard
        .intern_type_tag(&struct_with_vec_arg)
        .unwrap()
        .raw_address_for_testing();
    let r2_again = guard
        .intern_type_tag(&struct_with_vec_arg)
        .unwrap()
        .raw_address_for_testing();
    assert_eq!(r2, r2_again, "Struct<Vector<U8>> must intern idempotently");

    // Struct<Vector<U8>> ≠ Struct<U8> (different type_args).
    let struct_with_u8_arg = TypeTag::Struct(Box::new(StructTag {
        address: AccountAddress::ONE,
        module: ident_str!("foo").to_owned(),
        name: ident_str!("Wrapper").to_owned(),
        type_args: vec![TypeTag::U8],
    }));
    let r3 = guard
        .intern_type_tag(&struct_with_u8_arg)
        .unwrap()
        .raw_address_for_testing();
    assert_ne!(r2, r3, "Struct<Vector<U8>> must differ from Struct<U8>");

    // The inner Vector(U8) canonical pointer must be stable after all the above.
    let vec_u8_addr_after = guard
        .intern_type_tag(&TypeTag::Vector(Box::new(TypeTag::U8)))
        .unwrap()
        .raw_address_for_testing();
    let vec_u8_addr_check = guard
        .intern_type_tag(&TypeTag::Vector(Box::new(TypeTag::U8)))
        .unwrap()
        .raw_address_for_testing();
    assert_eq!(
        vec_u8_addr_after, vec_u8_addr_check,
        "Vector(U8) canonical pointer must remain stable"
    );
}

// ── Concurrent convergence for TypeTag::Function ─────────────────────────────

#[test]
fn test_concurrent_function_type_converges() {
    let num_threads = 4;
    let ctx = Arc::new(GlobalContext::with_num_execution_workers(num_threads));
    let barrier = Arc::new(Barrier::new(num_threads));
    let func_tag = Arc::new(TypeTag::Function(Box::new(FunctionTag {
        args: vec![
            FunctionParamOrReturnTag::Reference(TypeTag::U8),
            FunctionParamOrReturnTag::Value(TypeTag::U64),
        ],
        results: vec![FunctionParamOrReturnTag::Value(TypeTag::Bool)],
        abilities: AbilitySet::singleton(Ability::Copy),
    })));
    let addresses = Arc::new(Mutex::new(HashSet::new()));

    let handles: Vec<_> = (0..num_threads)
        .map(|tid| {
            let ctx = Arc::clone(&ctx);
            let barrier = Arc::clone(&barrier);
            let func_tag = Arc::clone(&func_tag);
            let addresses = Arc::clone(&addresses);
            thread::spawn(move || {
                let guard = ctx.try_execution_context(tid).unwrap();
                barrier.wait();
                let r = guard.intern_type_tag(&func_tag).unwrap();
                addresses.lock().insert(r.raw_address_for_testing());
            })
        })
        .collect();

    for h in handles {
        h.join().unwrap();
    }
    let addrs = Arc::into_inner(addresses).unwrap().into_inner();
    assert_eq!(
        addrs.len(),
        1,
        "All threads must converge to a single canonical Function type pointer"
    );
}

// ── Proptest: FunctionParamOrReturnTag idempotency ────────────────────────────

proptest! {
    #[test]
    fn prop_intern_function_param_or_return_type_tag_idempotent(
        tag in any::<FunctionParamOrReturnTag>(),
    ) {
        let ctx = GlobalContext::with_num_execution_workers(1);
        let guard = ctx.try_execution_context(0).unwrap();
        let r1 = guard.intern_function_param_or_return_type_tag(&tag).unwrap();
        let r2 = guard.intern_function_param_or_return_type_tag(&tag).unwrap();
        prop_assert_eq!(r1.raw_address_for_testing(), r2.raw_address_for_testing());
    }
}

// ── Proptest: Function type idempotency (hand-rolled — TypeTag::Arbitrary ─────
// NOTE: The `Arbitrary` impl for `TypeTag` in `proptest_types.rs` does NOT
// generate `TypeTag::Function`, signed integers (`I8`–`I256`), or `Signer`.
// The proptests above therefore have a blind spot for those variants.
// The following test compensates with a hand-rolled strategy that generates
// `TypeTag::Function` values with varying arg kinds (Value / Reference /
// MutableReference), result types, and AbilitySets.
// The compensating proptest also uses only a fixed small set of primitive inner
// types for args/results; arbitrarily deep nesting inside Function args is not
// covered by this proptest but is exercised by the deterministic tests above.

proptest! {
    #[test]
    fn prop_intern_function_type_idempotent(
        arg_kind in 0u8..3u8,   // 0=Value, 1=Reference, 2=MutableReference
        arg_inner in prop_oneof![
            Just(TypeTag::Bool),
            Just(TypeTag::U8),
            Just(TypeTag::U64),
            Just(TypeTag::Address),
        ],
        result_tag in prop_oneof![
            Just(TypeTag::Bool),
            Just(TypeTag::U8),
            Just(TypeTag::U64),
        ],
        abilities in any::<AbilitySet>(),
    ) {
        let ctx = GlobalContext::with_num_execution_workers(1);
        let guard = ctx.try_execution_context(0).unwrap();

        let arg = match arg_kind {
            0 => FunctionParamOrReturnTag::Value(arg_inner),
            1 => FunctionParamOrReturnTag::Reference(arg_inner),
            _ => FunctionParamOrReturnTag::MutableReference(arg_inner),
        };
        let func = TypeTag::Function(Box::new(FunctionTag {
            args: vec![arg],
            results: vec![FunctionParamOrReturnTag::Value(result_tag)],
            abilities,
        }));
        let r1 = guard.intern_type_tag(&func).unwrap();
        let r2 = guard.intern_type_tag(&func).unwrap();
        prop_assert_eq!(r1.raw_address_for_testing(), r2.raw_address_for_testing());
    }
}

// ── TypeTreeSize correctness ──────────────────────────────────────────────────

fn unlimited_ctx() -> GlobalContext {
    GlobalContext::with_num_execution_workers_and_config(1, MaintenanceConfig {
        dummy: 0,
        type_tree_size_limits: TypeTreeSizeLimits {
            max_depth: u32::MAX,
            max_count: u32::MAX,
        },
    })
}

#[test]
fn test_type_tree_size_correctness() {
    let ctx = unlimited_ctx();
    let guard = ctx.try_execution_context(0).unwrap();

    // Primitives: count=1, depth=1.
    let bool_ref = guard.intern_type_tag(&TypeTag::Bool).unwrap();
    assert_eq!(bool_ref.size(), TypeTreeSize { count: 1, depth: 1 });

    // Vector(Bool): count=2, depth=2.
    let vec_bool = guard
        .intern_type_tag(&TypeTag::Vector(Box::new(TypeTag::Bool)))
        .unwrap();
    assert_eq!(vec_bool.size(), TypeTreeSize { count: 2, depth: 2 });

    // Vector(Vector(Bool)): count=3, depth=3.
    let vec_vec_bool = guard
        .intern_type_tag(&TypeTag::Vector(Box::new(TypeTag::Vector(Box::new(
            TypeTag::Bool,
        )))))
        .unwrap();
    assert_eq!(vec_vec_bool.size(), TypeTreeSize { count: 3, depth: 3 });

    // Ref(Bool) via FunctionParamOrReturnTag: count=2, depth=2.
    let ref_bool = guard
        .intern_function_param_or_return_type_tag(&FunctionParamOrReturnTag::Reference(
            TypeTag::Bool,
        ))
        .unwrap();
    assert_eq!(ref_bool.size(), TypeTreeSize { count: 2, depth: 2 });

    // Struct {} (no type args): count=1, depth=1.
    let struct_no_args = guard
        .intern_type_tag(&TypeTag::Struct(Box::new(StructTag {
            address: AccountAddress::ONE,
            module: ident_str!("foo").to_owned(),
            name: ident_str!("T").to_owned(),
            type_args: vec![],
        })))
        .unwrap();
    assert_eq!(struct_no_args.size(), TypeTreeSize { count: 1, depth: 1 });

    // Struct<Bool> (one type arg): count=2, depth=2.
    let struct_bool = guard
        .intern_type_tag(&TypeTag::Struct(Box::new(StructTag {
            address: AccountAddress::ONE,
            module: ident_str!("foo").to_owned(),
            name: ident_str!("T").to_owned(),
            type_args: vec![TypeTag::Bool],
        })))
        .unwrap();
    assert_eq!(struct_bool.size(), TypeTreeSize { count: 2, depth: 2 });

    // Struct<Bool, U8> (two type args): count=3, depth=2.
    let struct_bool_u8 = guard
        .intern_type_tag(&TypeTag::Struct(Box::new(StructTag {
            address: AccountAddress::ONE,
            module: ident_str!("foo").to_owned(),
            name: ident_str!("T").to_owned(),
            type_args: vec![TypeTag::Bool, TypeTag::U8],
        })))
        .unwrap();
    assert_eq!(struct_bool_u8.size(), TypeTreeSize { count: 3, depth: 2 });

    // Function([] -> []): count=1, depth=1.
    let func_unit = guard
        .intern_type_tag(&TypeTag::Function(Box::new(FunctionTag {
            args: vec![],
            results: vec![],
            abilities: AbilitySet::EMPTY,
        })))
        .unwrap();
    assert_eq!(func_unit.size(), TypeTreeSize { count: 1, depth: 1 });

    // Function([Bool] -> [U8]): count=3, depth=2.
    let func_bool_u8 = guard
        .intern_type_tag(&TypeTag::Function(Box::new(FunctionTag {
            args: vec![FunctionParamOrReturnTag::Value(TypeTag::Bool)],
            results: vec![FunctionParamOrReturnTag::Value(TypeTag::U8)],
            abilities: AbilitySet::EMPTY,
        })))
        .unwrap();
    assert_eq!(func_bool_u8.size(), TypeTreeSize { count: 3, depth: 2 });

    // Struct<Vector<Bool>>: count=3, depth=3.
    let struct_vec_bool = guard
        .intern_type_tag(&TypeTag::Struct(Box::new(StructTag {
            address: AccountAddress::ONE,
            module: ident_str!("foo").to_owned(),
            name: ident_str!("T").to_owned(),
            type_args: vec![TypeTag::Vector(Box::new(TypeTag::Bool))],
        })))
        .unwrap();
    assert_eq!(struct_vec_bool.size(), TypeTreeSize { count: 3, depth: 3 });
}

// ── Depth limit: ±1 boundary tests ───────────────────────────────────────────

#[test]
fn test_depth_limit_boundary() {
    let ctx = GlobalContext::with_num_execution_workers_and_config(1, MaintenanceConfig {
        dummy: 0,
        type_tree_size_limits: TypeTreeSizeLimits {
            max_depth: 3,
            max_count: u32::MAX,
        },
    });
    let guard = ctx.try_execution_context(0).unwrap();

    // Depth exactly 3 (Vector(Vector(Bool))): should succeed.
    let depth3 = TypeTag::Vector(Box::new(TypeTag::Vector(Box::new(TypeTag::Bool))));
    assert!(
        guard.intern_type_tag(&depth3).is_ok(),
        "depth-3 type must succeed with max_depth=3"
    );

    // Depth 4 (Vector(Vector(Vector(Bool)))): should fail.
    let depth4 = TypeTag::Vector(Box::new(TypeTag::Vector(Box::new(TypeTag::Vector(
        Box::new(TypeTag::Bool),
    )))));
    let result = guard.intern_type_tag(&depth4);
    assert!(
        matches!(result, Err(TypeError::TypeTooDeep { depth: 4, max: 3 })),
        "depth-4 type must fail with max_depth=3"
    );

    // Depth-4 must fail even though its depth-3 subtypes are already cached.
    // The depth-3 subtypes were interned in the successful call above, so the
    // cache has them. The new composite being built (depth-4) must still check
    // its own size and fail before allocating.
    let depth4_again = TypeTag::Vector(Box::new(TypeTag::Vector(Box::new(TypeTag::Vector(
        Box::new(TypeTag::Bool),
    )))));
    assert!(
        matches!(
            guard.intern_type_tag(&depth4_again),
            Err(TypeError::TypeTooDeep { .. })
        ),
        "depth-4 must fail even with depth-3 subtypes cached"
    );
}

// ── Count limit: ±1 boundary tests ───────────────────────────────────────────

#[test]
fn test_count_limit_boundary() {
    let ctx = GlobalContext::with_num_execution_workers_and_config(1, MaintenanceConfig {
        dummy: 0,
        type_tree_size_limits: TypeTreeSizeLimits {
            max_depth: u32::MAX,
            max_count: 3,
        },
    });
    let guard = ctx.try_execution_context(0).unwrap();

    // Struct<Bool, U8> has count=3: should succeed.
    let count3 = TypeTag::Struct(Box::new(StructTag {
        address: AccountAddress::ONE,
        module: ident_str!("m").to_owned(),
        name: ident_str!("T").to_owned(),
        type_args: vec![TypeTag::Bool, TypeTag::U8],
    }));
    assert!(
        guard.intern_type_tag(&count3).is_ok(),
        "count-3 type must succeed with max_count=3"
    );

    // Struct<Bool, U8, U16> has count=4: should fail.
    let count4 = TypeTag::Struct(Box::new(StructTag {
        address: AccountAddress::ONE,
        module: ident_str!("m").to_owned(),
        name: ident_str!("T").to_owned(),
        type_args: vec![TypeTag::Bool, TypeTag::U8, TypeTag::U16],
    }));
    let result = guard.intern_type_tag(&count4);
    assert!(
        matches!(result, Err(TypeError::TypeTooLarge { count: 4, max: 3 })),
        "count-4 type must fail with max_count=3"
    );
}

// ── Both limits exceeded: depth checked first ────────────────────────────────

#[test]
fn test_both_limits_depth_checked_first() {
    let ctx = GlobalContext::with_num_execution_workers_and_config(1, MaintenanceConfig {
        dummy: 0,
        type_tree_size_limits: TypeTreeSizeLimits {
            max_depth: 2,
            max_count: 2,
        },
    });
    let guard = ctx.try_execution_context(0).unwrap();

    // Vector(Bool): depth=2, count=2 — just within limits.
    assert!(
        guard
            .intern_type_tag(&TypeTag::Vector(Box::new(TypeTag::Bool)))
            .is_ok(),
        "depth=2,count=2 must succeed with max_depth=2,max_count=2"
    );

    // Vector(Vector(Bool)): depth=3, count=3 — exceeds both limits.
    // Depth is checked first, so the error must be TypeTooDeep.
    let result = guard.intern_type_tag(&TypeTag::Vector(Box::new(TypeTag::Vector(Box::new(
        TypeTag::Bool,
    )))));
    assert!(
        matches!(result, Err(TypeError::TypeTooDeep { .. })),
        "when both depth and count are exceeded, TypeTooDeep must be returned first"
    );
}

// ── Default limits are permissive ────────────────────────────────────────────

#[test]
fn test_default_limits_are_permissive() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();

    // Depth-64 chain: 63 Vectors around Bool gives depth = 1 + 63 = 64.
    let mut tag = TypeTag::Bool;
    for _ in 0..63 {
        tag = TypeTag::Vector(Box::new(tag));
    }
    assert!(
        guard.intern_type_tag(&tag).is_ok(),
        "depth-64 chain must succeed with default limits (max_depth=64)"
    );

    // Count-512: Struct with 511 Bool type args → count = 511 + 1 = 512.
    let type_args = vec![TypeTag::Bool; 511];
    let struct_512 = TypeTag::Struct(Box::new(StructTag {
        address: AccountAddress::ONE,
        module: ident_str!("m").to_owned(),
        name: ident_str!("T").to_owned(),
        type_args,
    }));
    assert!(
        guard.intern_type_tag(&struct_512).is_ok(),
        "count-512 type must succeed with default limits (max_count=512)"
    );
}

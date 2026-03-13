// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Integration tests for type interning and cache-hit deduplication.

use mono_move_global_context::GlobalContext;
use move_core_types::{
    ability::AbilitySet,
    account_address::AccountAddress,
    identifier::Identifier,
    language_storage::{FunctionParamOrReturnTag, FunctionTag, StructTag, TypeTag},
};

#[test]
fn test_primitive_type_tag_interning() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.execution_context(0).unwrap();

    let tags = [
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

    for i in 0..tags.len() {
        for j in i..tags.len() {
            let ty1 = guard.intern_type_tag(&tags[i]);
            let ty2 = guard.intern_type_tag(&tags[j]);
            if i == j {
                assert!(ty1 == ty2);
            } else {
                assert!(ty1 != ty2);
            }
        }
    }
}

#[test]
fn test_composite_type_tag_is_cached() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    {
        let guard = ctx.execution_context(0).unwrap();

        let ty1 = guard.intern_type_tag(&TypeTag::Vector(Box::new(TypeTag::U64)));
        let ty2 = guard.intern_type_tag(&TypeTag::Vector(Box::new(TypeTag::U64)));
        assert!(ty1 == ty2);
    }

    let maintenance = ctx.maintenance_context().unwrap();
    assert_eq!(maintenance.interned_types_count(), 1);
}

#[test]
fn test_type_tags_list_is_cached() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.execution_context(0).unwrap();

    let tags = vec![TypeTag::U64, TypeTag::Bool];

    // intern_type_tags([U64, Bool]) twice → same ListRef pointer.
    let ref1 = guard.intern_type_tags(&tags);
    let ref2 = guard.intern_type_tags(&tags);
    assert!(ref1 == ref2);

    drop(guard);
    let maintenance = ctx.maintenance_context().unwrap();
    // 1 list entry for [U64, Bool].
    assert_eq!(maintenance.interned_type_lists_count(), 1);
}

#[test]
fn test_function_type_tag_list_cache() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.execution_context(0).unwrap();

    let function_tag = TypeTag::Function(Box::new(FunctionTag {
        args: vec![FunctionParamOrReturnTag::Value(TypeTag::U64)],
        results: vec![],
        abilities: AbilitySet::EMPTY,
    }));

    // Same Function type interned twice → same Ref; counts do not grow.
    let ref1 = guard.intern_type_tag(&function_tag);
    let ref2 = guard.intern_type_tag(&function_tag);
    assert!(ref1 == ref2);

    drop(guard);
    let maintenance = ctx.maintenance_context().unwrap();
    assert_eq!(maintenance.interned_types_count(), 1);
}

#[test]
fn test_function_ref_arg_per_element_cache() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.execution_context(0).unwrap();

    // Two Reference(U64) args: first allocates Ref(U64), second hits cache.
    guard.intern_type_tag(&TypeTag::Function(Box::new(FunctionTag {
        args: vec![
            FunctionParamOrReturnTag::Reference(TypeTag::U64),
            FunctionParamOrReturnTag::Reference(TypeTag::U64),
        ],
        results: vec![],
        abilities: AbilitySet::EMPTY,
    })));

    drop(guard);
    let maintenance = ctx.maintenance_context().unwrap();
    // Only 1 Ref(U64) type in the interner despite 2 Reference(U64) args.
    assert_eq!(maintenance.interned_types_count(), 2); // Ref(U64) + Function(...)
}

#[test]
fn test_function_mut_ref_arg_per_element_cache() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.execution_context(0).unwrap();

    // Two MutableReference(U64) args: first allocates RefMut(U64), second hits cache.
    guard.intern_type_tag(&TypeTag::Function(Box::new(FunctionTag {
        args: vec![
            FunctionParamOrReturnTag::MutableReference(TypeTag::U64),
            FunctionParamOrReturnTag::MutableReference(TypeTag::U64),
        ],
        results: vec![],
        abilities: AbilitySet::EMPTY,
    })));

    drop(guard);
    let maintenance = ctx.maintenance_context().unwrap();
    // Only 1 RefMut(U64) type in the interner despite 2 MutableReference(U64) args.
    assert_eq!(maintenance.interned_types_count(), 2); // RefMut(U64) + Function(...)
}

#[test]
fn test_struct_type_tag_cached() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.execution_context(0).unwrap();

    let tag = TypeTag::Struct(Box::new(StructTag {
        address: AccountAddress::ZERO,
        module: Identifier::new("mymod").unwrap(),
        name: Identifier::new("MyStruct").unwrap(),
        type_args: vec![],
    }));

    let r1 = guard.intern_type_tag(&tag);
    let r2 = guard.intern_type_tag(&tag);
    assert!(r1 == r2);

    drop(guard);
    let maintenance = ctx.maintenance_context().unwrap();
    assert_eq!(maintenance.interned_types_count(), 1);
    assert_eq!(maintenance.interned_executable_ids_count(), 1);
    assert_eq!(maintenance.interned_type_lists_count(), 1);
    assert_eq!(maintenance.interned_identifiers_count(), 2);
}

#[test]
fn test_struct_different_names_distinct() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.execution_context(0).unwrap();

    let tag_a = TypeTag::Struct(Box::new(StructTag {
        address: AccountAddress::ZERO,
        module: Identifier::new("mymod").unwrap(),
        name: Identifier::new("A").unwrap(),
        type_args: vec![],
    }));
    let tag_b = TypeTag::Struct(Box::new(StructTag {
        address: AccountAddress::ZERO,
        module: Identifier::new("mymod").unwrap(),
        name: Identifier::new("B").unwrap(),
        type_args: vec![],
    }));

    let r1 = guard.intern_type_tag(&tag_a);
    let r2 = guard.intern_type_tag(&tag_b);
    assert!(r1 != r2);

    drop(guard);
    let maintenance = ctx.maintenance_context().unwrap();
    assert_eq!(maintenance.interned_types_count(), 2);
    assert_eq!(maintenance.interned_executable_ids_count(), 1);
}

#[test]
fn test_struct_different_addresses_distinct() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.execution_context(0).unwrap();

    let tag_a = TypeTag::Struct(Box::new(StructTag {
        address: AccountAddress::ZERO,
        module: Identifier::new("mymod").unwrap(),
        name: Identifier::new("MyStruct").unwrap(),
        type_args: vec![],
    }));
    let tag_b = TypeTag::Struct(Box::new(StructTag {
        address: AccountAddress::ONE,
        module: Identifier::new("mymod").unwrap(),
        name: Identifier::new("MyStruct").unwrap(),
        type_args: vec![],
    }));

    let r1 = guard.intern_type_tag(&tag_a);
    let r2 = guard.intern_type_tag(&tag_b);
    assert!(r1 != r2);

    drop(guard);
    let maintenance = ctx.maintenance_context().unwrap();
    assert_eq!(maintenance.interned_types_count(), 2);
    // Different addresses → different executable_ids.
    assert_eq!(maintenance.interned_executable_ids_count(), 2);
}

#[test]
fn test_struct_different_type_args_distinct() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.execution_context(0).unwrap();

    let tag_u64 = TypeTag::Struct(Box::new(StructTag {
        address: AccountAddress::ZERO,
        module: Identifier::new("mymod").unwrap(),
        name: Identifier::new("Generic").unwrap(),
        type_args: vec![TypeTag::U64],
    }));
    let tag_bool = TypeTag::Struct(Box::new(StructTag {
        address: AccountAddress::ZERO,
        module: Identifier::new("mymod").unwrap(),
        name: Identifier::new("Generic").unwrap(),
        type_args: vec![TypeTag::Bool],
    }));

    let r1 = guard.intern_type_tag(&tag_u64);
    let r2 = guard.intern_type_tag(&tag_bool);
    assert!(r1 != r2);

    drop(guard);
    let maintenance = ctx.maintenance_context().unwrap();
    assert_eq!(maintenance.interned_types_count(), 2);
    // Two distinct type-arg lists: [U64] and [Bool].
    assert_eq!(maintenance.interned_type_lists_count(), 2);
}

#[test]
fn test_generic_struct_shared_type_args() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.execution_context(0).unwrap();

    // Struct A<U64> and Struct B<U64> — both have type_args=[U64], should share the list entry.
    let tag_a = TypeTag::Struct(Box::new(StructTag {
        address: AccountAddress::ZERO,
        module: Identifier::new("mymod").unwrap(),
        name: Identifier::new("A").unwrap(),
        type_args: vec![TypeTag::U64],
    }));
    let tag_b = TypeTag::Struct(Box::new(StructTag {
        address: AccountAddress::ZERO,
        module: Identifier::new("mymod").unwrap(),
        name: Identifier::new("B").unwrap(),
        type_args: vec![TypeTag::U64],
    }));

    guard.intern_type_tag(&tag_a);
    guard.intern_type_tag(&tag_b);

    drop(guard);
    let maintenance = ctx.maintenance_context().unwrap();
    // [U64] type list is shared between A<U64> and B<U64>.
    assert_eq!(maintenance.interned_type_lists_count(), 1);
    assert_eq!(maintenance.interned_types_count(), 2); // A<U64> and B<U64> are distinct
}

#[test]
fn test_function_shared_arg_list_across_types() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.execution_context(0).unwrap();

    let shared_args = vec![FunctionParamOrReturnTag::Value(TypeTag::U64)];

    // First function: args=[Value(U64)], results=[]
    let ref1 = guard.intern_type_tag(&TypeTag::Function(Box::new(FunctionTag {
        args: shared_args.clone(),
        results: vec![],
        abilities: AbilitySet::EMPTY,
    })));

    // Second function: same args, different results=[Value(Bool)]
    let ref2 = guard.intern_type_tag(&TypeTag::Function(Box::new(FunctionTag {
        args: shared_args.clone(),
        results: vec![FunctionParamOrReturnTag::Value(TypeTag::Bool)],
        abilities: AbilitySet::EMPTY,
    })));

    // Different functions, different Refs.
    assert!(ref1 != ref2);

    drop(guard);
    let maintenance = ctx.maintenance_context().unwrap();
    // type_lists: shared args=[Value(U64)], results=[], results=[Value(Bool)]
    assert_eq!(maintenance.interned_type_lists_count(), 3);
}

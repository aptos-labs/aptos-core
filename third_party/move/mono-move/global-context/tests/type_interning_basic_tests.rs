// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use global_context::{GlobalContext, GlobalContextConfig};
use move_core_types::{
    account_address::AccountAddress,
    identifier::Identifier,
    language_storage::{StructTag, TypeTag},
};

#[test]
fn test_primitive_type_interning() {
    let context = GlobalContext::new();
    let exec_ctx = context.execution_context(0).unwrap();

    // Intern primitive types
    let bool_ptr1 = exec_ctx.intern_type_tag(&TypeTag::Bool);
    let bool_ptr2 = exec_ctx.intern_type_tag(&TypeTag::Bool);
    let u64_ptr1 = exec_ctx.intern_type_tag(&TypeTag::U64);
    let u64_ptr2 = exec_ctx.intern_type_tag(&TypeTag::U64);

    // Check pointer identity (same type should give same pointer)
    assert!(bool_ptr1 == bool_ptr2);
    assert!(u64_ptr1 == u64_ptr2);
    assert!(bool_ptr1 != u64_ptr1);

    // Test all primitive types
    let primitives = vec![
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

    for tag in &primitives {
        let ptr1 = exec_ctx.intern_type_tag(tag);
        let ptr2 = exec_ctx.intern_type_tag(tag);
        assert!(ptr1 == ptr2);
    }
}

#[test]
fn test_vector_type_interning() {
    let context = GlobalContext::new();
    let exec_ctx = context.execution_context(0).unwrap();

    // Simple vector types
    let vec_u64_1 = exec_ctx.intern_type_tag(&TypeTag::Vector(Box::new(TypeTag::U64)));
    let vec_u64_2 = exec_ctx.intern_type_tag(&TypeTag::Vector(Box::new(TypeTag::U64)));
    assert!(vec_u64_1 == vec_u64_2);

    // Nested vectors
    let vec_vec_u64_1 = exec_ctx.intern_type_tag(&TypeTag::Vector(Box::new(TypeTag::Vector(
        Box::new(TypeTag::U64),
    ))));
    let vec_vec_u64_2 = exec_ctx.intern_type_tag(&TypeTag::Vector(Box::new(TypeTag::Vector(
        Box::new(TypeTag::U64),
    ))));
    assert!(vec_vec_u64_1 == vec_vec_u64_2);

    // Different vector types should have different pointers
    let vec_u8 = exec_ctx.intern_type_tag(&TypeTag::Vector(Box::new(TypeTag::U8)));
    assert!(vec_u64_1 != vec_u8);
}

#[test]
fn test_struct_type_interning() {
    let context = GlobalContext::new();
    let exec_ctx = context.execution_context(0).unwrap();

    let address = AccountAddress::from_hex_literal("0x1").unwrap();
    let module = Identifier::new("TestModule").unwrap();
    let name = Identifier::new("TestStruct").unwrap();

    // Non-generic struct
    let struct_tag1 = TypeTag::Struct(Box::new(StructTag {
        address,
        module: module.clone(),
        name: name.clone(),
        type_args: vec![],
    }));
    let struct_tag2 = TypeTag::Struct(Box::new(StructTag {
        address,
        module: module.clone(),
        name: name.clone(),
        type_args: vec![],
    }));

    let struct_ptr1 = exec_ctx.intern_type_tag(&struct_tag1);
    let struct_ptr2 = exec_ctx.intern_type_tag(&struct_tag2);
    assert!(struct_ptr1 == struct_ptr2);

    // Generic struct with type arguments
    let generic_struct1 = TypeTag::Struct(Box::new(StructTag {
        address,
        module: module.clone(),
        name: name.clone(),
        type_args: vec![TypeTag::U64, TypeTag::Bool],
    }));
    let generic_struct2 = TypeTag::Struct(Box::new(StructTag {
        address,
        module: module.clone(),
        name: name.clone(),
        type_args: vec![TypeTag::U64, TypeTag::Bool],
    }));

    let generic_ptr1 = exec_ctx.intern_type_tag(&generic_struct1);
    let generic_ptr2 = exec_ctx.intern_type_tag(&generic_struct2);
    assert!(generic_ptr1 == generic_ptr2);

    // Different type arguments should give different pointers
    let generic_struct3 = TypeTag::Struct(Box::new(StructTag {
        address,
        module,
        name,
        type_args: vec![TypeTag::U8, TypeTag::Bool],
    }));
    let generic_ptr3 = exec_ctx.intern_type_tag(&generic_struct3);
    assert!(generic_ptr1 != generic_ptr3);
}

#[test]
fn test_type_list_interning() {
    let context = GlobalContext::new();
    let exec_ctx = context.execution_context(0).unwrap();

    // Intern type lists
    let list1 = vec![TypeTag::U64, TypeTag::Bool, TypeTag::Address];
    let list2 = vec![TypeTag::U64, TypeTag::Bool, TypeTag::Address];
    let list3 = vec![TypeTag::U64, TypeTag::Bool];

    let ptr1 = exec_ctx.intern_type_tags(&list1);
    let ptr2 = exec_ctx.intern_type_tags(&list2);
    let ptr3 = exec_ctx.intern_type_tags(&list3);

    // Same lists should have same pointer
    assert!(ptr1 == ptr2);
    assert!(ptr1 != ptr3);
}

#[test]
fn test_empty_type_list() {
    let context = GlobalContext::new();
    let exec_ctx = context.execution_context(0).unwrap();

    let empty1 = exec_ctx.intern_type_tags(&[]);
    let empty2 = exec_ctx.intern_type_tags(&[]);
    assert!(empty1 == empty2);
}

#[test]
fn test_maintenance_flush() {
    let context = GlobalContext::with_config(GlobalContextConfig {
        memory_threshold_bytes: 1, // Very small threshold to force flush
    });

    {
        let exec_ctx = context.execution_context(0).unwrap();

        // Intern some types
        exec_ctx.intern_type_tag(&TypeTag::U64);
        exec_ctx.intern_type_tag(&TypeTag::Bool);
        exec_ctx.intern_type_tags(&[TypeTag::U64, TypeTag::Bool]);
    }

    // Get maintenance context and check memory
    let mut maint_ctx = context.maintenance_context().unwrap();

    // Check counts before flush
    let type_count_before = maint_ctx.interned_type_count();
    let type_list_count_before = maint_ctx.interned_type_list_count();
    assert!(type_count_before > 0);
    assert!(type_list_count_before > 0);

    // Force flush by checking memory (threshold is very small)
    let flushed = maint_ctx.check_memory_usage();
    assert!(flushed);

    // Check counts after flush
    assert_eq!(maint_ctx.interned_type_count(), 0);
    assert_eq!(maint_ctx.interned_type_list_count(), 0);
}

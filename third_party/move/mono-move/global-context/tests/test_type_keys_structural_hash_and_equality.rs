// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Test that structural equality works correctly for type and type list keys.

use global_context::GlobalContext;
use move_core_types::language_storage::TypeTag;

#[test]
fn test_structural_equality_deduplication() {
    let context = GlobalContext::with_num_workers(1);
    let exec_ctx1 = context.execution_context(0).unwrap();
    let exec_ctx2 = context.execution_context(1).unwrap();

    // Intern the same type in two different execution contexts
    // They will be allocated at different arena locations
    let type1 = TypeTag::Vector(Box::new(TypeTag::U64));
    let type2 = TypeTag::Vector(Box::new(TypeTag::U64));

    let ptr1 = exec_ctx1.intern_type_tag(&type1);
    let ptr2 = exec_ctx2.intern_type_tag(&type2);

    // Even though they were interned in different contexts (and could be at
    // different arena locations initially), they should deduplicate to the
    // same pointer because the interner is shared
    assert!(ptr1 == ptr2);
}

#[test]
fn test_structural_equality_complex_types() {
    use move_core_types::{
        account_address::AccountAddress, identifier::Identifier, language_storage::StructTag,
    };

    let context = GlobalContext::with_num_workers(2);
    let exec_ctx1 = context.execution_context(0).unwrap();
    let exec_ctx2 = context.execution_context(1).unwrap();

    let address = AccountAddress::from_hex_literal("0x1").unwrap();
    let module = Identifier::new("TestModule").unwrap();
    let name = Identifier::new("TestStruct").unwrap();

    // Create structurally identical struct types
    let struct1 = TypeTag::Struct(Box::new(StructTag {
        address,
        module: module.clone(),
        name: name.clone(),
        type_args: vec![TypeTag::U64, TypeTag::Bool],
    }));

    let struct2 = TypeTag::Struct(Box::new(StructTag {
        address,
        module: module.clone(),
        name: name.clone(),
        type_args: vec![TypeTag::U64, TypeTag::Bool],
    }));

    // Intern in different contexts
    let ptr1 = exec_ctx1.intern_type_tag(&struct1);
    let ptr2 = exec_ctx2.intern_type_tag(&struct2);

    // Should be the same pointer due to structural equality
    assert!(ptr1 == ptr2,);
}

#[test]
fn test_type_list_structural_equality() {
    let context = GlobalContext::with_num_workers(2);
    let exec_ctx1 = context.execution_context(0).unwrap();
    let exec_ctx2 = context.execution_context(1).unwrap();

    // Create structurally identical type lists
    let list1 = vec![TypeTag::U64, TypeTag::Bool, TypeTag::Address];
    let list2 = vec![TypeTag::U64, TypeTag::Bool, TypeTag::Address];

    // Intern in different contexts
    let ptr1 = exec_ctx1.intern_type_tags(&list1);
    let ptr2 = exec_ctx2.intern_type_tags(&list2);

    // Should be the same pointer due to structural equality
    assert!(ptr1 == ptr2,);
}

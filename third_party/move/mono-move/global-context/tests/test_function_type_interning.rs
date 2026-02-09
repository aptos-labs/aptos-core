// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Tests for function type interning.

use global_context::GlobalContext;
use move_core_types::{
    ability::AbilitySet,
    language_storage::{FunctionParamOrReturnTag, FunctionTag, TypeTag},
};

#[test]
fn test_simple_function_type_interning() {
    let context = GlobalContext::new();
    let exec_ctx = context.execution_context(0).unwrap();

    // Create a simple function type: fn(U64) -> Bool
    let function_tag1 = TypeTag::Function(Box::new(FunctionTag {
        args: vec![FunctionParamOrReturnTag::Value(TypeTag::U64)],
        results: vec![FunctionParamOrReturnTag::Value(TypeTag::Bool)],
        abilities: AbilitySet::EMPTY,
    }));

    let function_tag2 = TypeTag::Function(Box::new(FunctionTag {
        args: vec![FunctionParamOrReturnTag::Value(TypeTag::U64)],
        results: vec![FunctionParamOrReturnTag::Value(TypeTag::Bool)],
        abilities: AbilitySet::EMPTY,
    }));

    let ptr1 = exec_ctx.intern_type_tag(&function_tag1);
    let ptr2 = exec_ctx.intern_type_tag(&function_tag2);

    assert!(ptr1 == ptr2);
}

#[test]
fn test_function_type_with_references() {
    let context = GlobalContext::new();
    let exec_ctx = context.execution_context(0).unwrap();

    // Create function type: fn(&U64, &mut Bool) -> U8
    let function_tag1 = TypeTag::Function(Box::new(FunctionTag {
        args: vec![
            FunctionParamOrReturnTag::Reference(TypeTag::U64),
            FunctionParamOrReturnTag::MutableReference(TypeTag::Bool),
        ],
        results: vec![FunctionParamOrReturnTag::Value(TypeTag::U8)],
        abilities: AbilitySet::EMPTY,
    }));

    let function_tag2 = TypeTag::Function(Box::new(FunctionTag {
        args: vec![
            FunctionParamOrReturnTag::Reference(TypeTag::U64),
            FunctionParamOrReturnTag::MutableReference(TypeTag::Bool),
        ],
        results: vec![FunctionParamOrReturnTag::Value(TypeTag::U8)],
        abilities: AbilitySet::EMPTY,
    }));

    let ptr1 = exec_ctx.intern_type_tag(&function_tag1);
    let ptr2 = exec_ctx.intern_type_tag(&function_tag2);

    assert!(ptr1 == ptr2,);
}

#[test]
fn test_different_function_types() {
    let context = GlobalContext::new();
    let exec_ctx = context.execution_context(0).unwrap();

    // Create two different function types
    let function1 = TypeTag::Function(Box::new(FunctionTag {
        args: vec![FunctionParamOrReturnTag::Value(TypeTag::U64)],
        results: vec![FunctionParamOrReturnTag::Value(TypeTag::Bool)],
        abilities: AbilitySet::EMPTY,
    }));

    let function2 = TypeTag::Function(Box::new(FunctionTag {
        args: vec![FunctionParamOrReturnTag::Value(TypeTag::Bool)],
        results: vec![FunctionParamOrReturnTag::Value(TypeTag::U64)],
        abilities: AbilitySet::EMPTY,
    }));

    let ptr1 = exec_ctx.intern_type_tag(&function1);
    let ptr2 = exec_ctx.intern_type_tag(&function2);

    // Different function types should have different pointers
    assert!(ptr1 != ptr2,);
}

#[test]
fn test_function_type_with_multiple_returns() {
    let context = GlobalContext::new();
    let exec_ctx = context.execution_context(0).unwrap();

    // Create function type: fn(U64) -> (Bool, U8)
    let function_tag1 = TypeTag::Function(Box::new(FunctionTag {
        args: vec![FunctionParamOrReturnTag::Value(TypeTag::U64)],
        results: vec![
            FunctionParamOrReturnTag::Value(TypeTag::Bool),
            FunctionParamOrReturnTag::Value(TypeTag::U8),
        ],
        abilities: AbilitySet::EMPTY,
    }));

    let function_tag2 = TypeTag::Function(Box::new(FunctionTag {
        args: vec![FunctionParamOrReturnTag::Value(TypeTag::U64)],
        results: vec![
            FunctionParamOrReturnTag::Value(TypeTag::Bool),
            FunctionParamOrReturnTag::Value(TypeTag::U8),
        ],
        abilities: AbilitySet::EMPTY,
    }));

    let ptr1 = exec_ctx.intern_type_tag(&function_tag1);
    let ptr2 = exec_ctx.intern_type_tag(&function_tag2);

    assert!(ptr1 == ptr2,);
}

#[test]
fn test_function_type_abilities() {
    let context = GlobalContext::new();
    let exec_ctx = context.execution_context(0).unwrap();

    // Create two function types with different abilities
    let function1 = TypeTag::Function(Box::new(FunctionTag {
        args: vec![FunctionParamOrReturnTag::Value(TypeTag::U64)],
        results: vec![FunctionParamOrReturnTag::Value(TypeTag::Bool)],
        abilities: AbilitySet::EMPTY,
    }));

    let function2 = TypeTag::Function(Box::new(FunctionTag {
        args: vec![FunctionParamOrReturnTag::Value(TypeTag::U64)],
        results: vec![FunctionParamOrReturnTag::Value(TypeTag::Bool)],
        abilities: AbilitySet::PRIMITIVES,
    }));

    let ptr1 = exec_ctx.intern_type_tag(&function1);
    let ptr2 = exec_ctx.intern_type_tag(&function2);

    // Different abilities should result in different types
    assert!(ptr1 != ptr2,);
}

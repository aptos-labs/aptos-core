// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::file_format::{
    empty_script_with_dependencies, FunctionAttribute, FunctionHandle, IdentifierIndex,
    ModuleHandleIndex, SignatureIndex,
};
use move_bytecode_verifier::verifier;
use move_core_types::vm_status::StatusCode;

#[test]
fn test_script_with_struct_api_attributes_fails() {
    // Create a script with a dependency module
    let mut script = empty_script_with_dependencies(vec!["TestModule"]);

    let func_handle = FunctionHandle {
        module: ModuleHandleIndex(0),
        name: IdentifierIndex(0),
        parameters: SignatureIndex(0),
        return_: SignatureIndex(0),
        type_parameters: vec![],
        access_specifiers: None,
        attributes: vec![FunctionAttribute::Pack],
    };
    script.function_handles.push(func_handle);

    // Verification should fail
    let result = verifier::verify_script(&script);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.major_status(), StatusCode::INVALID_OPERATION_IN_SCRIPT);
}

#[test]
fn test_script_with_borrow_field_attributes_fails() {
    // Test BorrowFieldImmutable
    let mut script = empty_script_with_dependencies(vec!["TestModule"]);
    let func_handle = FunctionHandle {
        module: ModuleHandleIndex(0),
        name: IdentifierIndex(0),
        parameters: SignatureIndex(0),
        return_: SignatureIndex(0),
        type_parameters: vec![],
        access_specifiers: None,
        attributes: vec![FunctionAttribute::BorrowFieldImmutable(0)],
    };
    script.function_handles.push(func_handle);

    let result = verifier::verify_script(&script);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.major_status(), StatusCode::INVALID_OPERATION_IN_SCRIPT);

    // Test BorrowFieldMutable
    let mut script = empty_script_with_dependencies(vec!["TestModule"]);
    let func_handle = FunctionHandle {
        module: ModuleHandleIndex(0),
        name: IdentifierIndex(0),
        parameters: SignatureIndex(0),
        return_: SignatureIndex(0),
        type_parameters: vec![],
        access_specifiers: None,
        attributes: vec![FunctionAttribute::BorrowFieldMutable(0)],
    };
    script.function_handles.push(func_handle);

    let result = verifier::verify_script(&script);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.major_status(), StatusCode::INVALID_OPERATION_IN_SCRIPT);
}

#[test]
fn test_script_without_function_attributes_succeeds() {
    // Create a script with a dependency module
    let mut script = empty_script_with_dependencies(vec!["TestModule"]);

    // Add a function handle without attributes
    let func_handle = FunctionHandle {
        module: ModuleHandleIndex(0),
        name: IdentifierIndex(0),
        parameters: SignatureIndex(0),
        return_: SignatureIndex(0),
        type_parameters: vec![],
        access_specifiers: None,
        attributes: vec![], // No attributes - this is OK
    };
    script.function_handles.push(func_handle);

    // Verification should succeed
    let result = verifier::verify_script(&script);
    if let Err(e) = &result {
        eprintln!("Unexpected error: {:?}", e);
    }
    assert!(
        result.is_ok(),
        "Expected script without attributes to pass verification"
    );
}

// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::{errors::VMResult, file_format::*, IndexKind};
use move_bytecode_verifier::DuplicationChecker;
use move_core_types::{ability::AbilitySet, identifier::Identifier, vm_status::StatusCode};
use proptest::prelude::*;

fn assert_duplicate_error(result: VMResult<()>, expected_index_kind: IndexKind) {
    let err = result.unwrap_err();
    assert_eq!(err.major_status(), StatusCode::DUPLICATE_ELEMENT);
    assert_eq!(err.indices().len(), 1);
    assert_eq!(err.indices()[0].0, expected_index_kind);
}

#[test]
fn duplicated_friend_decls() {
    let mut m = basic_test_module();
    let handle = ModuleHandle {
        address: AddressIdentifierIndex::new(0),
        name: IdentifierIndex::new(0),
    };
    m.friend_decls.push(handle.clone());
    m.friend_decls.push(handle);
    assert_duplicate_error(
        DuplicationChecker::verify_module(&m),
        IndexKind::ModuleHandle,
    );
}

#[test]
fn duplicated_function_names() {
    let mut m = basic_test_module();
    // basic_test_module already has one function with name at IdentifierIndex(1) = "foo"
    // Add another function handle with the same name identifier
    m.function_handles.push(FunctionHandle {
        module: ModuleHandleIndex(0),
        name: IdentifierIndex(1), // Same name as the first function ("foo")
        parameters: SignatureIndex(0),
        return_: SignatureIndex(0),
        type_parameters: vec![],
        access_specifiers: None,
        attributes: vec![],
    });
    m.function_defs.push(FunctionDefinition {
        function: FunctionHandleIndex(1),
        visibility: Visibility::Private,
        is_entry: false,
        acquires_global_resources: vec![],
        code: Some(CodeUnit {
            locals: SignatureIndex(0),
            code: vec![Bytecode::Ret],
        }),
    });
    // The checker first checks function handles before function definitions
    // Since we have duplicate handles, the error is reported at the FunctionHandle level
    assert_duplicate_error(
        DuplicationChecker::verify_module(&m),
        IndexKind::FunctionHandle,
    );
}

#[test]
fn duplicated_struct_names() {
    let mut m = basic_test_module();
    // basic_test_module already has one struct handle at index 0 (named "Bar")
    // Add an identifier for the struct name
    let struct_name_idx = IdentifierIndex(m.identifiers.len() as u16);
    m.identifiers
        .push(Identifier::new("MyStruct".to_string()).unwrap());

    // Create first struct handle (will be at index 1)
    m.struct_handles.push(StructHandle {
        module: ModuleHandleIndex(0),
        name: struct_name_idx,
        abilities: AbilitySet::EMPTY,
        type_parameters: vec![],
    });

    // Create first struct definition
    m.struct_defs.push(StructDefinition {
        struct_handle: StructHandleIndex(1),
        field_information: StructFieldInformation::Native,
    });

    // Create second struct handle with the same name (will be at index 2)
    m.struct_handles.push(StructHandle {
        module: ModuleHandleIndex(0),
        name: struct_name_idx, // Same name identifier
        abilities: AbilitySet::EMPTY,
        type_parameters: vec![],
    });

    // Create second struct definition
    m.struct_defs.push(StructDefinition {
        struct_handle: StructHandleIndex(2),
        field_information: StructFieldInformation::Native,
    });

    // The checker first checks struct handles before struct definitions
    // Since we have duplicate handles, the error is reported at the StructHandle level
    assert_duplicate_error(
        DuplicationChecker::verify_module(&m),
        IndexKind::StructHandle,
    );
}

proptest! {
    #[test]
    fn valid_duplication(module in CompiledModule::valid_strategy(20)) {
        prop_assert!(DuplicationChecker::verify_module(&module).is_ok());
    }
}

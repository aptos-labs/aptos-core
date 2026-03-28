// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests for struct API checker behavior that cannot be expressed as .masm transactional tests,
//! such as hand-crafted modules with malformed imported function handles.

use crate::verifier::{verify_module_with_config, VerifierConfig};
use move_binary_format::{
    file_format::{
        AddressIdentifierIndex, Bytecode, CodeUnit, CompiledModule, FunctionAttribute,
        FunctionDefinition, FunctionHandle, FunctionHandleIndex, IdentifierIndex, ModuleHandle,
        ModuleHandleIndex, Signature, SignatureIndex, Visibility,
    },
    file_format_common::VERSION_10,
};
use move_core_types::{
    account_address::AccountAddress, identifier::Identifier, vm_status::StatusCode,
};

fn base_module() -> CompiledModule {
    CompiledModule {
        version: VERSION_10,
        module_handles: vec![ModuleHandle {
            address: AddressIdentifierIndex(0),
            name: IdentifierIndex(0),
        }],
        self_module_handle_idx: ModuleHandleIndex(0),
        identifiers: vec![Identifier::new("M").unwrap()],
        address_identifiers: vec![AccountAddress::ZERO],
        function_handles: vec![],
        function_defs: vec![],
        signatures: vec![Signature(vec![])],
        struct_defs: vec![],
        struct_handles: vec![],
        constant_pool: vec![],
        metadata: vec![],
        field_handles: vec![],
        friend_decls: vec![],
        struct_def_instantiations: vec![],
        function_instantiations: vec![],
        field_instantiations: vec![],
        struct_variant_handles: vec![],
        struct_variant_instantiations: vec![],
        variant_field_handles: vec![],
        variant_field_instantiations: vec![],
    }
}

/// An imported function handle (not backed by any local FunctionDef) with duplicate
/// struct API attributes should be rejected. The .masm assembler cannot produce this
/// case because the assembler copies attributes faithfully from the exporting module;
/// only hand-crafted binaries can have this shape.
#[test]
fn imported_handle_with_duplicate_struct_api_attrs_rejected() {
    let mut module = base_module();

    // A second module handle represents the external module being imported from.
    module.module_handles.push(ModuleHandle {
        address: AddressIdentifierIndex(0),
        name: IdentifierIndex(1),
    });
    module.identifiers.push(Identifier::new("Other").unwrap());
    module
        .identifiers
        .push(Identifier::new("borrow_mut$S$0").unwrap());

    // Imported function handle (module index 1 = "Other", no local FunctionDef).
    // Two identical BorrowFieldMutable(0) attributes — only one is allowed.
    module.function_handles.push(FunctionHandle {
        module: ModuleHandleIndex(1),
        name: IdentifierIndex(2),
        parameters: SignatureIndex(0),
        return_: SignatureIndex(0),
        type_parameters: vec![],
        access_specifiers: None,
        attributes: vec![
            FunctionAttribute::BorrowFieldMutable(0),
            FunctionAttribute::BorrowFieldMutable(0),
        ],
    });

    let config = VerifierConfig::default();
    let err = verify_module_with_config(&config, &module)
        .expect_err("duplicate imported handle attributes should be rejected");
    assert_eq!(err.major_status(), StatusCode::INVALID_STRUCT_API_CODE);
}

/// A locally-defined function with duplicate struct API attributes should also be
/// rejected (this path goes through check_struct_api_impl, not StructApiContext::new).
#[test]
fn local_definition_with_duplicate_struct_api_attrs_rejected() {
    let mut module = base_module();
    module.identifiers.push(Identifier::new("pack$S").unwrap());

    module.function_handles.push(FunctionHandle {
        module: ModuleHandleIndex(0), // self
        name: IdentifierIndex(1),
        parameters: SignatureIndex(0),
        return_: SignatureIndex(0),
        type_parameters: vec![],
        access_specifiers: None,
        attributes: vec![FunctionAttribute::Pack, FunctionAttribute::Pack],
    });

    module.function_defs.push(FunctionDefinition {
        function: FunctionHandleIndex(0),
        visibility: Visibility::Public,
        is_entry: false,
        acquires_global_resources: vec![],
        code: Some(CodeUnit {
            locals: SignatureIndex(0),
            code: vec![Bytecode::Ret],
        }),
    });

    let config = VerifierConfig::default();
    let err = verify_module_with_config(&config, &module)
        .expect_err("duplicate local definition attributes should be rejected");
    assert_eq!(err.major_status(), StatusCode::INVALID_STRUCT_API_CODE);
}

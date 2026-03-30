// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests for struct API checker behavior that cannot be expressed as .masm transactional tests.
//! The .masm assembler produces function handles faithfully from the source annotations, so a
//! module with duplicate struct API attributes on an *imported* handle can only be constructed
//! as a hand-crafted binary.

use crate::verifier::{verify_module_with_config, VerifierConfig};
use move_binary_format::file_format::{
    empty_module, AddressIdentifierIndex, FunctionAttribute, FunctionHandle, IdentifierIndex,
    ModuleHandle, ModuleHandleIndex, Signature, SignatureIndex, SignatureToken, StructHandle,
    StructHandleIndex,
};
use move_core_types::{ability::AbilitySet, identifier::Identifier, vm_status::StatusCode};

/// An imported function handle (not backed by any local FunctionDef) with duplicate struct API
/// attributes should be rejected.
///
/// This cannot be a .masm transactional test: the assembler copies attributes faithfully from
/// source annotations, and a published module with duplicate attrs on its locally-defined
/// functions would already fail verification before any importer can reference it. Only a
/// hand-crafted binary can place duplicate attrs directly on an imported handle.
#[test]
fn imported_handle_with_duplicate_struct_api_attrs_rejected() {
    let mut module = empty_module();

    // A second module handle represents the external module being imported from.
    module.module_handles.push(ModuleHandle {
        address: AddressIdentifierIndex(0),
        name: IdentifierIndex(1),
    });
    module.identifiers.push(Identifier::new("Other").unwrap()); // index 1
    module.identifiers.push(Identifier::new("S").unwrap()); // index 2
    module.identifiers.push(Identifier::new("pack$S").unwrap()); // index 3

    // Struct handle for S (defined in "Other", zero fields).
    module.struct_handles.push(StructHandle {
        module: ModuleHandleIndex(1),
        name: IdentifierIndex(2),
        abilities: AbilitySet::EMPTY,
        type_parameters: vec![],
    });

    // Return signature: (S,). Parameters use the existing empty signature (index 0)
    // because S has zero fields so pack$S takes no arguments.
    module
        .signatures
        .push(Signature(vec![SignatureToken::Struct(StructHandleIndex(
            0,
        ))]));

    // Imported function handle (module index 1 = "Other", no local FunctionDef).
    // Two identical Pack attributes — only one is allowed.
    module.function_handles.push(FunctionHandle {
        module: ModuleHandleIndex(1),
        name: IdentifierIndex(3),
        parameters: SignatureIndex(0), // empty: S has zero fields
        return_: SignatureIndex(1),    // (S,)
        type_parameters: vec![],
        access_specifiers: None,
        attributes: vec![FunctionAttribute::Pack, FunctionAttribute::Pack],
    });

    let config = VerifierConfig::default();
    let err = verify_module_with_config(&config, &module)
        .expect_err("duplicate imported handle attributes should be rejected");
    assert_eq!(err.major_status(), StatusCode::INVALID_STRUCT_API_CODE);
}

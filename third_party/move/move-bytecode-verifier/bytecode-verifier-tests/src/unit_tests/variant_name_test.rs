// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::{
    file_format::{
        AbilitySet, AddressIdentifierIndex, FieldDefinition, IdentifierIndex, ModuleHandle,
        ModuleHandleIndex, Signature, SignatureToken, StructDefinition, StructFieldInformation,
        StructHandle, StructHandleIndex, StructTypeParameter, TypeSignature, VariantDefinition,
    },
    file_format_common::VERSION_7,
    CompiledModule,
};
use move_bytecode_verifier::{
    verifier::verify_module_with_config_for_test_with_version, VerifierConfig,
};
use move_core_types::{identifier::Identifier, vm_status::StatusCode};

/// Tests whether the name of a variant is in bounds. (That is, the IdentifierIndex
/// is in bounds of the identifier table.)
#[test]
fn test_variant_name() {
    // This is a POC produced during auditing
    let ty = SignatureToken::Bool;

    let cm = CompiledModule {
        version: 7,
        self_module_handle_idx: ModuleHandleIndex(0),
        module_handles: vec![ModuleHandle {
            address: AddressIdentifierIndex(0),
            name: IdentifierIndex(0),
        }],
        struct_handles: vec![StructHandle {
            module: ModuleHandleIndex(0),
            name: IdentifierIndex(0),
            abilities: AbilitySet::ALL,
            type_parameters: vec![StructTypeParameter {
                constraints: AbilitySet::EMPTY,
                is_phantom: true,
            }],
        }],
        function_handles: vec![],
        field_handles: vec![],
        friend_decls: vec![],
        struct_def_instantiations: vec![],
        function_instantiations: vec![],
        field_instantiations: vec![],
        signatures: vec![Signature(vec![]), Signature(vec![ty])],
        identifiers: vec![Identifier::new("M").unwrap()],
        address_identifiers: vec![],
        constant_pool: vec![],
        metadata: vec![],
        struct_defs: vec![StructDefinition {
            struct_handle: StructHandleIndex(0),
            field_information: StructFieldInformation::DeclaredVariants(vec![VariantDefinition {
                fields: vec![FieldDefinition {
                    name: IdentifierIndex(0),
                    signature: TypeSignature(SignatureToken::Bool),
                }],
                // <---- out of bound
                name: IdentifierIndex(1),
            }]),
        }],
        function_defs: vec![],
        struct_variant_handles: vec![],
        struct_variant_instantiations: vec![],
        variant_field_handles: vec![],
        variant_field_instantiations: vec![],
    };

    let result = verify_module_with_config_for_test_with_version(
        "test_variant_name",
        &VerifierConfig::production(),
        &cm,
        Some(VERSION_7),
    );

    assert_eq!(
        result.unwrap_err().major_status(),
        StatusCode::INDEX_OUT_OF_BOUNDS,
    );
}

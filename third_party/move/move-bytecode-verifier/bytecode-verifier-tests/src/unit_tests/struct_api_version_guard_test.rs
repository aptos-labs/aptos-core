// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Tests that the struct API checker is gated on bytecode version 10.
//!
//! The struct API checker in `code_unit_verifier` is guarded:
//!   ```ignore
//!   if module.version() >= VERSION_10 { ... run checker ... }
//!   ```
//! For modules at version 9 the checker is skipped entirely, so a function
//! with `FunctionAttribute::Pack` and an invalid bytecode body passes
//! verification at version 9 but fails at version 10.
//!
//! Note: `FunctionAttribute::Pack` (and other struct-API attributes) are also
//! gated at the *serializer*: trying to serialize such an attribute at version 9
//! returns an error.  The tests below call `verify_module_with_config` directly
//! for the version-9 case, exercising the verifier guard in isolation.

use move_binary_format::{
    file_format::{
        AddressIdentifierIndex, Bytecode, CodeUnit, FieldDefinition, FunctionAttribute,
        FunctionDefinition, FunctionHandle, FunctionHandleIndex, IdentifierIndex, ModuleHandle,
        ModuleHandleIndex, Signature, SignatureIndex, SignatureToken, StructDefinition,
        StructFieldInformation, StructHandle, StructHandleIndex, TypeSignature, Visibility,
    },
    file_format_common::{VERSION_10, VERSION_9},
    CompiledModule,
};
use move_bytecode_verifier::{verifier::verify_module_with_config, VerifierConfig};
use move_core_types::{ability::AbilitySet, identifier::Identifier, vm_status::StatusCode};

/// Build a minimal module containing:
///   struct S { f: u64 }
///   #[pack] public fun pack$S(u64): S { LdU64(42); Pack(S); Ret }
///
/// The function body is intentionally wrong: the pack pattern requires
/// `MoveLoc(0); Pack(S); Ret` but we use `LdU64(42); Pack(S); Ret`.
/// With a correct signature but an invalid bytecode pattern, the struct API
/// checker should reject this when module.version >= VERSION_10.
fn make_module_with_bad_pack_bytecode(version: u32) -> CompiledModule {
    // identifiers: [SELF, S, f, pack$S]
    let ident_self = Identifier::new("SELF").unwrap();
    let ident_s = Identifier::new("S").unwrap();
    let ident_f = Identifier::new("f").unwrap();
    let ident_pack_s = Identifier::new("pack$S").unwrap();

    // address_identifiers: [0x0]
    let addr = move_core_types::account_address::AccountAddress::ZERO;

    // signatures:
    //   [0] () - empty
    //   [1] (u64) - pack$S parameters
    //   [2] (S) - pack$S return type (StructHandleIndex(0) = S)
    let sig_empty = Signature(vec![]);
    let sig_params = Signature(vec![SignatureToken::U64]);
    let sig_return = Signature(vec![SignatureToken::Struct(StructHandleIndex(0))]);

    CompiledModule {
        version,
        self_module_handle_idx: ModuleHandleIndex(0),
        module_handles: vec![ModuleHandle {
            address: AddressIdentifierIndex(0),
            name: IdentifierIndex(0), // "SELF"
        }],
        struct_handles: vec![StructHandle {
            module: ModuleHandleIndex(0),
            name: IdentifierIndex(1), // "S"
            abilities: AbilitySet::EMPTY,
            type_parameters: vec![],
        }],
        function_handles: vec![FunctionHandle {
            module: ModuleHandleIndex(0),
            name: IdentifierIndex(3), // "pack$S"
            parameters: SignatureIndex(1),
            return_: SignatureIndex(2),
            type_parameters: vec![],
            access_specifiers: None,
            attributes: vec![FunctionAttribute::Pack],
        }],
        field_handles: vec![],
        friend_decls: vec![],
        struct_def_instantiations: vec![],
        function_instantiations: vec![],
        field_instantiations: vec![],
        signatures: vec![sig_empty, sig_params, sig_return],
        identifiers: vec![ident_self, ident_s, ident_f, ident_pack_s],
        address_identifiers: vec![addr],
        constant_pool: vec![],
        metadata: vec![],
        struct_defs: vec![StructDefinition {
            struct_handle: StructHandleIndex(0),
            field_information: StructFieldInformation::Declared(vec![FieldDefinition {
                name: IdentifierIndex(2), // "f"
                signature: TypeSignature(SignatureToken::U64),
            }]),
        }],
        function_defs: vec![FunctionDefinition {
            function: FunctionHandleIndex(0),
            visibility: Visibility::Public,
            is_entry: false,
            acquires_global_resources: vec![],
            code: Some(CodeUnit {
                locals: SignatureIndex(0),
                // Wrong: should be MoveLoc(0) but uses LdU64(42).
                // This violates the required pack bytecode pattern.
                code: vec![
                    Bytecode::LdU64(42),
                    Bytecode::Pack(move_binary_format::file_format::StructDefinitionIndex(0)),
                    Bytecode::Ret,
                ],
            }),
        }],
        struct_variant_handles: vec![],
        struct_variant_instantiations: vec![],
        variant_field_handles: vec![],
        variant_field_instantiations: vec![],
    }
}

/// A module with bytecode version 9 that has FunctionAttribute::Pack and invalid
/// struct API bytecode must pass the *verifier*: the struct API checker is skipped
/// for module.version() < VERSION_10.
///
/// We call `verify_module_with_config` directly rather than the test helper
/// because the *serializer* also gates struct-API attributes at VERSION_10;
/// this test exercises the verifier guard in isolation.
#[test]
fn test_struct_api_version_guard_v9_passes() {
    let module = make_module_with_bad_pack_bytecode(VERSION_9);
    let result = verify_module_with_config(&VerifierConfig::production(), &module);
    assert!(
        result.is_ok(),
        "expected v9 module to pass (struct API checker skipped), got: {:?}",
        result
    );
}

/// The same module with bytecode version 10 must be rejected with
/// INVALID_STRUCT_API_CODE: the struct API checker enforces the pack pattern.
#[test]
fn test_struct_api_version_guard_v10_fails() {
    let module = make_module_with_bad_pack_bytecode(VERSION_10);
    let result = verify_module_with_config(&VerifierConfig::production(), &module);
    assert_eq!(
        result.unwrap_err().major_status(),
        StatusCode::INVALID_STRUCT_API_CODE,
        "expected INVALID_STRUCT_API_CODE for v10 module with bad pack bytecode"
    );
}

/// The struct-API attributes (Pack, Unpack, etc.) are also gated in the
/// *serializer*: attempting to serialize a module with FunctionAttribute::Pack
/// at bytecode version 9 returns an error.  This ensures the attributes cannot
/// appear in persisted version-9 bytecode.
#[test]
fn test_struct_api_attribute_not_serializable_at_v9() {
    let module = make_module_with_bad_pack_bytecode(VERSION_9);
    let mut bytes = vec![];
    let result = module.serialize_for_version(Some(VERSION_9), &mut bytes);
    assert!(
        result.is_err(),
        "expected serialization to fail for FunctionAttribute::Pack at version 9"
    );
}

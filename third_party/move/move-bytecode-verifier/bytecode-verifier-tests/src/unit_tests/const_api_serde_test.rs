// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Tests that `FunctionAttribute::ConstantAccessor` is gated at the serializer/deserializer
//! boundary for bytecode version 11.
//!
//! The attribute is introduced at VERSION_11.  The serializer rejects it for earlier
//! versions, and a deserializer capped at VERSION_10 rejects any module that was
//! serialized at VERSION_11.  Together these two gates ensure that a module carrying
//! `ConstantAccessor` cannot appear in a pre-V11 environment.

use move_binary_format::{
    deserializer::DeserializerConfig,
    file_format::{
        AddressIdentifierIndex, Bytecode, CodeUnit, Constant, FunctionAttribute,
        FunctionDefinition, FunctionHandle, FunctionHandleIndex, IdentifierIndex, ModuleHandle,
        ModuleHandleIndex, Signature, SignatureIndex, SignatureToken, Visibility,
    },
    file_format_common::{VERSION_10, VERSION_11},
    CompiledModule,
};
use move_core_types::{
    account_address::AccountAddress, identifier::Identifier, vm_status::StatusCode,
};

/// Build a minimal module containing a function with `FunctionAttribute::ConstantAccessor`
/// at the requested bytecode `version`.
fn make_module_with_const_accessor(version: u32) -> CompiledModule {
    let ident_self = Identifier::new("SELF").unwrap();
    let ident_const_max = Identifier::new("const$MAX").unwrap();

    let addr = AccountAddress::ZERO;

    let sig_empty = Signature(vec![]);
    let sig_return = Signature(vec![SignatureToken::U64]);

    CompiledModule {
        version,
        self_module_handle_idx: ModuleHandleIndex(0),
        module_handles: vec![ModuleHandle {
            address: AddressIdentifierIndex(0),
            name: IdentifierIndex(0),
        }],
        struct_handles: vec![],
        function_handles: vec![FunctionHandle {
            module: ModuleHandleIndex(0),
            name: IdentifierIndex(1),
            parameters: SignatureIndex(0),
            return_: SignatureIndex(1),
            type_parameters: vec![],
            access_specifiers: None,
            attributes: vec![FunctionAttribute::ConstantAccessor],
        }],
        field_handles: vec![],
        friend_decls: vec![],
        struct_def_instantiations: vec![],
        function_instantiations: vec![],
        field_instantiations: vec![],
        signatures: vec![sig_empty, sig_return],
        identifiers: vec![ident_self, ident_const_max],
        address_identifiers: vec![addr],
        constant_pool: vec![Constant {
            type_: SignatureToken::U64,
            data: 100u64.to_le_bytes().to_vec(),
        }],
        metadata: vec![],
        struct_defs: vec![],
        function_defs: vec![FunctionDefinition {
            function: FunctionHandleIndex(0),
            visibility: Visibility::Public,
            is_entry: false,
            acquires_global_resources: vec![],
            code: Some(CodeUnit {
                locals: SignatureIndex(0),
                code: vec![
                    Bytecode::LdConst(move_binary_format::file_format::ConstantPoolIndex(0)),
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

/// Serializing a module with `FunctionAttribute::ConstantAccessor` at version 10 must fail
/// because the attribute was introduced at VERSION_11.
#[test]
fn test_const_api_attribute_not_serializable_at_v10() {
    let module = make_module_with_const_accessor(VERSION_10);
    let mut bytes = vec![];
    let result = module.serialize_for_version(Some(VERSION_10), &mut bytes);
    assert!(
        result.is_err(),
        "expected serialization to fail for FunctionAttribute::ConstantAccessor at version 10"
    );
}

/// A module carrying `FunctionAttribute::ConstantAccessor` serialized as V11 must be
/// rejected by a deserializer capped at V10, ensuring it cannot be loaded in a
/// pre-V11 environment.
#[test]
fn test_const_api_attribute_not_deserializable_at_v10() {
    let module = make_module_with_const_accessor(VERSION_11);
    let mut bytes = vec![];
    module
        .serialize_for_version(Some(VERSION_11), &mut bytes)
        .expect("V11 serialization should succeed");

    let config = DeserializerConfig::new(VERSION_10, u64::MAX);
    let result = CompiledModule::deserialize_with_config(&bytes, &config);
    assert_eq!(
        result.unwrap_err().major_status(),
        StatusCode::UNKNOWN_VERSION,
        "expected UNKNOWN_VERSION: V11 module with ConstantAccessor cannot be loaded under V10 cap"
    );
}

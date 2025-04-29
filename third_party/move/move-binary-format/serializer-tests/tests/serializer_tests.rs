// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::{
    access::ModuleAccess,
    deserializer::DeserializerConfig,
    file_format::{
        empty_module, empty_script, AccessKind, AccessSpecifier, AddressIdentifierIndex,
        AddressSpecifier, CompiledModule, CompiledScript, FunctionHandle, IdentifierIndex,
        ResourceSpecifier, Signature, SignatureIndex, SignatureToken, TableIndex,
    },
    file_format_common::{IDENTIFIER_SIZE_MAX, VERSION_7, VERSION_MAX},
};
use move_core_types::{
    ability::AbilitySet, account_address::AccountAddress, identifier::Identifier,
};
use proptest::prelude::*;

proptest! {
    #[test]
    fn serializer_roundtrip(module in CompiledModule::valid_strategy(20)) {
        let mut serialized = Vec::with_capacity(2048);
        module.serialize_for_version(Some(VERSION_MAX), &mut serialized).expect("serialization should work");


        let deserialized_module = CompiledModule::deserialize_with_config(
                &serialized,
                &DeserializerConfig::new(VERSION_MAX, IDENTIFIER_SIZE_MAX)
        ).expect("deserialization should work");

        prop_assert_eq!(module, deserialized_module);
    }
}

proptest! {
    // Generating arbitrary compiled modules is really slow, possibly because of
    // https://github.com/AltSysrq/proptest/issues/143.
    #![proptest_config(ProptestConfig::with_cases(16))]

    /// Make sure that garbage inputs don't crash the serializer and deserializer.
    #[test]
    fn garbage_inputs(module in any_with::<CompiledModule>(16)) {
        let mut serialized = Vec::with_capacity(65536);
        module.serialize_for_version(Some(VERSION_MAX), &mut serialized).expect("serialization should work");

        let deserialized_module = CompiledModule::deserialize_no_check_bounds(&serialized)
            .expect("deserialization should work");
        prop_assert_eq!(module, deserialized_module);
    }
}

#[test]
fn simple_generic_module_round_trip() {
    let mut m = empty_module();

    // signature unit
    let sig_unit_idx = SignatureIndex::new(m.signatures.len() as u16);
    m.signatures.push(Signature(vec![]));

    // signature T1
    let sig_t1_idx = SignatureIndex::new(m.signatures.len() as u16);
    m.signatures
        .push(Signature(vec![SignatureToken::TypeParameter(0)]));

    // identifier f
    let ident_f_idx = IdentifierIndex::new(m.identifiers.len() as u16);
    m.identifiers.push(Identifier::new("f").unwrap());

    // function handle f
    m.function_handles.push(FunctionHandle {
        module: m.self_handle_idx(),
        name: ident_f_idx,
        parameters: sig_t1_idx,
        return_: sig_unit_idx,
        type_parameters: vec![AbilitySet::EMPTY],
        access_specifiers: None,
        attributes: vec![],
    });

    let mut serialized = Vec::with_capacity(2048);
    m.serialize_for_version(Some(VERSION_MAX), &mut serialized)
        .expect("serialization should work");

    let deserialized_m = CompiledModule::deserialize_with_config(
        &serialized,
        &DeserializerConfig::new(VERSION_MAX, IDENTIFIER_SIZE_MAX),
    )
    .expect("deserialization should work");

    assert_eq!(m, deserialized_m);
}

#[test]
fn simple_script_round_trip() {
    let s = simple_script_with_access_specifiers();
    let mut serialized = Vec::with_capacity(2048);
    s.serialize_for_version(Some(VERSION_MAX), &mut serialized)
        .expect("serialization should work");

    let deserialized_s = CompiledScript::deserialize_with_config(
        &serialized,
        &DeserializerConfig::new(VERSION_MAX, IDENTIFIER_SIZE_MAX),
    )
    .expect("deserialization should work");

    assert_eq!(s, deserialized_s);
}

fn simple_script_with_access_specifiers() -> CompiledScript {
    let mut s = empty_script();
    let addr = AddressIdentifierIndex::new(s.address_identifiers.len() as TableIndex);
    s.address_identifiers.push(AccountAddress::ONE);
    s.access_specifiers = Some(vec![AccessSpecifier {
        kind: AccessKind::Reads,
        negated: false,
        resource: ResourceSpecifier::DeclaredAtAddress(addr),
        address: AddressSpecifier::Any,
    }]);
    s
}

#[test]
fn simple_script_round_trip_version_failure() {
    let s = simple_script_with_access_specifiers();
    let mut serialized = Vec::with_capacity(2048);
    let err = s
        .serialize_for_version(Some(VERSION_7), &mut serialized)
        .expect_err("serialization should not work");
    assert!(err
        .to_string()
        .contains("Access specifiers on scripts not supported"));
}

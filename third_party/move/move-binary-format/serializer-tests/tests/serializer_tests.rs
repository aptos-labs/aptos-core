// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::{
    access::ModuleAccess,
    file_format::{
        empty_module, AbilitySet, CompiledModule, FunctionHandle, IdentifierIndex, Signature,
        SignatureIndex, SignatureToken,
    },
};
use move_core_types::identifier::Identifier;
use proptest::prelude::*;

proptest! {
    #[test]
    fn serializer_roundtrip(module in CompiledModule::valid_strategy(20)) {
        let mut serialized = Vec::with_capacity(2048);
        module.serialize(&mut serialized).expect("serialization should work");

        let deserialized_module = CompiledModule::deserialize(&serialized)
            .expect("deserialization should work");

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
        module.serialize(&mut serialized).expect("serialization should work");

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
    });

    let mut serialized = Vec::with_capacity(2048);
    m.serialize(&mut serialized)
        .expect("serialization should work");

    let deserialized_m =
        CompiledModule::deserialize(&serialized).expect("deserialization should work");

    assert_eq!(m, deserialized_m);
}

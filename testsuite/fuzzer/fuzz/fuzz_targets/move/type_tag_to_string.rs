// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![no_main]
use arbitrary::Arbitrary;
use libfuzzer_sys::{fuzz_target, Corpus};
use move_core_types::{ability::AbilitySet, identifier::Identifier, language_storage::TypeTag};

mod utils;

#[derive(Arbitrary, Debug)]
struct FuzzData {
    a: TypeTag,
    b: TypeTag,
}

/// Returns whether every identifier is valid and every function ability set contains only defined
/// bits.
fn is_valid_type_tag(type_tag: &TypeTag) -> bool {
    match type_tag {
        TypeTag::Struct(struct_tag) => {
            Identifier::is_valid(struct_tag.module.to_string())
                && Identifier::is_valid(struct_tag.name.to_string())
                && struct_tag.type_args.iter().all(is_valid_type_tag)
        },
        TypeTag::Vector(inner_type_tag) => is_valid_type_tag(inner_type_tag),
        TypeTag::Function(function_tag) => {
            // Undefined ability bits survive `TypeTag` decoding but are omitted from canonical
            // strings, so exclude them from the injectivity property below.
            function_tag.abilities.is_subset(AbilitySet::ALL)
                && function_tag
                    .args
                    .iter()
                    .all(|t| is_valid_type_tag(t.inner_tag()))
                && function_tag
                    .results
                    .iter()
                    .all(|t| is_valid_type_tag(t.inner_tag()))
        },
        _ => true, // Primitive types are always valid
    }
}

/// Helper function to serialize and deserialize a TypeTag
fn roundtrip_type_tag(type_tag: &TypeTag) -> Option<TypeTag> {
    let serialized = bcs::to_bytes(type_tag).ok()?;
    bcs::from_bytes::<TypeTag>(&serialized).ok()
}

fuzz_target!(|data: FuzzData| -> Corpus {
    // Validate input data
    if !is_valid_type_tag(&data.a) || !is_valid_type_tag(&data.b) {
        return Corpus::Reject;
    }

    // Roundtrip type tags through serialization
    match roundtrip_type_tag(&data.a) {
        Some(tag) => assert_eq!(tag, data.a),
        None => return Corpus::Reject,
    };

    match roundtrip_type_tag(&data.b) {
        Some(tag) => assert_eq!(tag, data.b),
        None => return Corpus::Reject,
    };

    // If type tags are different, verify their string representations are also different

    if data.a != data.b {
        tdbg!(
            "a_type:{:?}\na_string:{}\nserialized:{:?}",
            data.a.clone(),
            data.a.to_canonical_string(),
            bcs::to_bytes(&data.a).unwrap()
        );
        tdbg!(
            "b_type:{:?}\nb_string:{}\nserialized:{:?}",
            data.b.clone(),
            data.b.to_canonical_string(),
            bcs::to_bytes(&data.b).unwrap()
        );
        assert!(data.a.to_canonical_string() != data.b.to_canonical_string());
    }

    Corpus::Keep
});

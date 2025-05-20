// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![no_main]
use arbitrary::Arbitrary;
use libfuzzer_sys::{fuzz_target, Corpus};

use move_core_types::language_storage::TypeTag;
use move_core_types::ability::AbilitySet;
use move_core_types::identifier::Identifier;
use bcs;
mod utils;

#[derive(Arbitrary, Debug)]
struct FuzzData {
    a: TypeTag,
    b: TypeTag,
}

/// Validates that all identifiers in a TypeTag are valid Move identifiers
fn contains_valid_identifiers(type_tag: &TypeTag) -> bool {
    match type_tag {
        TypeTag::Struct(struct_tag) => {
            Identifier::is_valid(&struct_tag.module.to_string()) &&
            Identifier::is_valid(&struct_tag.name.to_string()) &&
            struct_tag.type_args.iter().all(contains_valid_identifiers)
        },
        TypeTag::Vector(inner_type_tag) => contains_valid_identifiers(inner_type_tag),
        TypeTag::Function(function_tag) => {
            function_tag.args.iter().all(contains_valid_identifiers) &&
            function_tag.results.iter().all(contains_valid_identifiers)
        },
        _ => true, // Primitive types are always valid
    }
}

/// Validates ability sets within the TypeTag
fn validate_ability_set(type_tag: &TypeTag) -> bool {
    match type_tag {
        TypeTag::Struct(struct_tag) => {
            struct_tag.type_args.iter()
                .all(validate_ability_set)
        },
        TypeTag::Vector(inner_type_tag) => validate_ability_set(inner_type_tag),
        TypeTag::Function(function_tag) => {
            function_tag.abilities.into_u8() <= AbilitySet::ALL.into_u8() &&
            function_tag.args.iter().all(validate_ability_set) &&
            function_tag.results.iter().all(validate_ability_set)
        },
        _ => true,
    }
}

/// Helper function to serialize and deserialize a TypeTag
fn roundtrip_type_tag(type_tag: &TypeTag) -> Option<TypeTag> {
    let serialized = bcs::to_bytes(type_tag).ok()?;
    bcs::from_bytes::<TypeTag>(&serialized).ok()
}

fuzz_target!(|data: FuzzData| -> Corpus {
    // Validate input data
    if !contains_valid_identifiers(&data.a) ||
       !contains_valid_identifiers(&data.b) ||
       !validate_ability_set(&data.a) ||
       !validate_ability_set(&data.b) {
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
            data.a.to_string(),
            bcs::to_bytes(&data.a).unwrap()
        );
        tdbg!(
            "b_type:{:?}\nb_string:{}\nserialized:{:?}",
            data.b.clone(),
            data.b.to_string(),
            bcs::to_bytes(&data.b).unwrap()
        );
        assert!(data.a.to_string() != data.b.to_string());
    }

    Corpus::Keep
});

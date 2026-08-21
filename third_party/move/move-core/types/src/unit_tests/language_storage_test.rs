// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    account_address::AccountAddress,
    ident_str,
    identifier::Identifier,
    language_storage::{ModuleId, StructTag, TypeTag},
};
use bcs::test_helpers::assert_canonical_encode_decode;
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_module_id_canonical_roundtrip(module_id in any::<ModuleId>()) {
        assert_canonical_encode_decode(module_id);
    }
}

#[test]
fn test_type_tag_deserialize_rejects_invalid_identifiers() {
    // BCS mirror of `StructTag`, but with unvalidated strings in place of
    // `Identifier` fields.
    #[derive(serde::Serialize)]
    struct RawStructTag {
        address: AccountAddress,
        module: String,
        name: String,
        type_args: Vec<TypeTag>,
    }

    let raw_struct_tag_bytes = |module: &str, name: &str| {
        // `TypeTag::Struct` is enum variant 7 on the wire.
        let mut bytes = vec![7u8];
        bytes.extend(
            bcs::to_bytes(&RawStructTag {
                address: AccountAddress::ONE,
                module: module.to_string(),
                name: name.to_string(),
                type_args: vec![],
            })
            .unwrap(),
        );
        bytes
    };

    // Sanity-check the layout: valid identifiers deserialize successfully.
    let tag: TypeTag = bcs::from_bytes(&raw_struct_tag_bytes("m", "S")).unwrap();
    assert_eq!(
        tag,
        TypeTag::Struct(Box::new(StructTag {
            address: AccountAddress::ONE,
            module: Identifier::from(ident_str!("m")),
            name: Identifier::from(ident_str!("S")),
            type_args: vec![],
        }))
    );

    // Invalid identifiers must be rejected instead of producing a `TypeTag`
    // whose Display output cannot be parsed back.
    for (module, name) in [
        ("\n\n\ng\n", "S"),
        ("m", "\n\n\ng\n"),
        ("has space", "S"),
        ("m", ""),
        ("0starts_with_digit", "S"),
    ] {
        bcs::from_bytes::<TypeTag>(&raw_struct_tag_bytes(module, name)).expect_err(&format!(
            "deserializing struct tag with invalid identifier ({:?}, {:?}) should fail",
            module, name
        ));
    }
}

#[test]
fn test_type_tag_deserialize_case_insensitive() {
    let org_struct_tag = StructTag {
        address: AccountAddress::ONE,
        module: Identifier::from(ident_str!("TestModule")),
        name: Identifier::from(ident_str!("TestStruct")),
        type_args: vec![
            TypeTag::U8,
            TypeTag::U16,
            TypeTag::U32,
            TypeTag::U64,
            TypeTag::U128,
            TypeTag::U256,
            TypeTag::Bool,
            TypeTag::Address,
            TypeTag::Signer,
        ],
    };

    let current_json = serde_json::to_string(&org_struct_tag).unwrap();

    let upper_case_json = format!(
        r##"{{"address":"{}","module":"TestModule","name":"TestStruct","type_params":["U8","U16","U32","U64","U128","U256","Bool","Address","Signer"]}}"##,
        AccountAddress::ONE.to_hex()
    );
    let upper_case_decoded = serde_json::from_str(upper_case_json.as_str()).unwrap();
    assert_eq!(org_struct_tag, upper_case_decoded);

    let lower_case_json = format!(
        r##"{{"address":"{}","module":"TestModule","name":"TestStruct","type_args":["u8","u16","u32","u64","u128","u256","bool","address","signer"]}}"##,
        AccountAddress::ONE.to_hex()
    );
    let lower_case_decoded = serde_json::from_str(lower_case_json.as_str()).unwrap();
    assert_eq!(org_struct_tag, lower_case_decoded);

    assert_eq!(current_json, lower_case_json);
}

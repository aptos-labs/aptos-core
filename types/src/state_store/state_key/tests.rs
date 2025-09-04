// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    account_config::{AccountResource, ObjectGroupResource},
    state_store::state_key::StateKey,
};
use velor_crypto::hash::CryptoHash;
use move_core_types::{account_address::AccountAddress, ident_str, move_resource::MoveStructType};
use proptest::prelude::*;

fn assert_crypto_hash(key: &StateKey, expected_hash: &str) {
    let expected_hash = expected_hash.parse().unwrap();
    assert_eq!(CryptoHash::hash(key), expected_hash);
}

#[test]
fn test_resource_hash() {
    assert_crypto_hash(
        &StateKey::resource_typed::<AccountResource>(&AccountAddress::TWO).unwrap(),
        "8f9ab5d5e3c9f5b885fcceea388fecd16bdb490da08aac9d4f026ddc66733def",
    );
}

#[test]
fn test_resource_group_hash() {
    assert_crypto_hash(
        &StateKey::resource_group(&AccountAddress::TWO, &ObjectGroupResource::struct_tag()),
        "87973d52189ac6a25ea543214305c4c8fb3bc2ceea8c34600361b03527578133",
    );
}

#[test]
fn test_module_hash() {
    assert_crypto_hash(
        &StateKey::module(&AccountAddress::TWO, ident_str!("mymodule")),
        "83d33b345c5e4b25d8f4dfe2b98b492024313b3b6e4febea6bfa844dbd850200",
    );
}

#[test]
fn test_table_item_hash() {
    assert_crypto_hash(
        &StateKey::table_item(&"0x1002".parse().unwrap(), &[7, 2, 3]),
        "6f5550015f7a6036f88b2458f98a7e4800aba09e83f8f294dbf70bff77f224e6",
    );
}

#[test]
fn test_raw_hash() {
    assert_crypto_hash(
        &StateKey::raw(&[1, 2, 3]),
        "655ab5766bc87318e18d9287f32d318e15535d3db9d21a6e5a2b41a51b535aff",
    )
}

#[test]
fn test_debug() {
    // code
    let key = StateKey::module(&AccountAddress::ONE, ident_str!("account"));
    assert_eq!(
        &format!("{:?}", key),
        "StateKey::AccessPath { address: 0x1, path: \"Code(0000000000000000000000000000000000000000000000000000000000000001::account)\" }",
    );

    // resource
    let key = StateKey::resource_typed::<AccountResource>(&AccountAddress::FOUR).unwrap();
    assert_eq!(
        &format!("{:?}", key),
        "StateKey::AccessPath { address: 0x4, path: \"Resource(0x1::account::Account)\" }",
    );

    // resource group
    let key = StateKey::resource_group(&AccountAddress::THREE, &ObjectGroupResource::struct_tag());
    assert_eq!(
        &format!("{:?}", key),
        "StateKey::AccessPath { address: 0x3, path: \"ResourceGroup(0x1::object::ObjectGroup)\" }",
    );

    // table item
    let key = StateKey::table_item(&"0x123".parse().unwrap(), &[1]);
    assert_eq!(
        &format!("{:?}", key),
        "StateKey::TableItem { handle: 0000000000000000000000000000000000000000000000000000000000000123, key: 01 }"
    );

    // raw
    let key = StateKey::raw(&[1, 2, 3]);
    assert_eq!(&format!("{:?}", key), "StateKey::Raw(010203)",);
}

proptest! {
    #[test]
    fn test_shard_order(
        key1 in any::<StateKey>(),
        key2 in any::<StateKey>(),
    ) {
        let shard1 = key1.get_shard_id();
        let shard2 = key2.get_shard_id();

        assert_eq!(shard1, usize::from(key1.crypto_hash_ref().nibble(0)));
        assert_eq!(shard2, usize::from(key2.crypto_hash_ref().nibble(0)));

        if shard1 != shard2 {
            assert_eq!(
                shard1.cmp(&shard2),
                key1.crypto_hash_ref().cmp(key2.crypto_hash_ref()),
            )
        }
    }
}

// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    account_config::{AccountResource, ObjectGroupResource},
    state_store::state_key::{
        inner::{
            CollateralScope, StateKeyDecodeErr, StateKeyInner, TradingNativeKey,
            TradingNativeKeyTag,
        },
        StateKey,
    },
};
use aptos_crypto::hash::CryptoHash;
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

fn addr(byte: u8) -> AccountAddress {
    let mut bytes = [0u8; AccountAddress::LENGTH];
    bytes[AccountAddress::LENGTH - 1] = byte;
    AccountAddress::new(bytes)
}

/// Sub-tag ordinals are part of the on-disk key format.
#[test]
fn test_trading_native_sub_tags_pinned() {
    assert_eq!(TradingNativeKeyTag::Position as u8, 0);
    assert_eq!(TradingNativeKeyTag::Order as u8, 1);
    assert_eq!(TradingNativeKeyTag::BulkOrder as u8, 2);
    assert_eq!(TradingNativeKeyTag::Collateral as u8, 3);
}

#[test]
fn test_collateral_cross_key_encoding() {
    let key = StateKey::collateral(addr(5), addr(2), CollateralScope::Cross);

    let mut expected = vec![2u8, TradingNativeKeyTag::Collateral as u8];
    expected.extend_from_slice(addr(5).as_ref());
    expected.extend_from_slice(addr(2).as_ref());
    expected.push(0);
    assert_eq!(key.encoded().as_ref(), expected.as_slice());
    assert_eq!(key.size(), 1 + AccountAddress::LENGTH * 2 + 1);
    assert_eq!(StateKey::decode(key.encoded()).unwrap(), key);
}

#[test]
fn test_collateral_isolated_key_encoding() {
    let key = StateKey::collateral(addr(5), addr(2), CollateralScope::Isolated {
        market: addr(0x34),
    });

    let mut expected = vec![2u8, TradingNativeKeyTag::Collateral as u8];
    expected.extend_from_slice(addr(5).as_ref());
    expected.extend_from_slice(addr(2).as_ref());
    expected.push(1);
    expected.extend_from_slice(addr(0x34).as_ref());
    assert_eq!(key.encoded().as_ref(), expected.as_slice());
    assert_eq!(key.size(), 1 + AccountAddress::LENGTH * 3 + 1);
    assert_eq!(StateKey::decode(key.encoded()).unwrap(), key);
}

#[test]
fn test_collateral_scopes_are_distinct() {
    let cross = StateKey::collateral(addr(5), addr(2), CollateralScope::Cross);
    let isolated = StateKey::collateral(addr(5), addr(2), CollateralScope::Isolated {
        market: addr(0),
    });
    assert_ne!(cross, isolated);
    assert_ne!(cross.crypto_hash_ref(), isolated.crypto_hash_ref());
}

#[test]
fn test_collateral_decode_rejects_bad_scope() {
    let cross = StateKey::collateral(addr(5), addr(2), CollateralScope::Cross);
    let mut bad_tag = cross.encoded().to_vec();
    *bad_tag.last_mut().unwrap() = 7;
    assert!(matches!(
        StateKey::decode(&bad_tag),
        Err(StateKeyDecodeErr::NotEnoughBytes { .. })
    ));
    let mut cross_with_market = cross.encoded().to_vec();
    cross_with_market.extend_from_slice(addr(0x34).as_ref());
    assert!(matches!(
        StateKey::decode(&cross_with_market),
        Err(StateKeyDecodeErr::NotEnoughBytes { .. })
    ));
    let isolated = StateKey::collateral(addr(5), addr(2), CollateralScope::Isolated {
        market: addr(0x34),
    });
    let truncated = &isolated.encoded()[..isolated.encoded().len() - 1];
    assert!(matches!(
        StateKey::decode(truncated),
        Err(StateKeyDecodeErr::NotEnoughBytes { .. })
    ));
}

#[test]
fn test_order_key_encoding() {
    let order_id = 0x0102_0304_0506_0708_090A_0B0C_0D0E_0F10_u128;
    let key = StateKey::order(addr(5), addr(2), order_id);

    let mut expected = vec![2u8, TradingNativeKeyTag::Order as u8];
    expected.extend_from_slice(addr(5).as_ref());
    expected.extend_from_slice(addr(2).as_ref());
    expected.extend_from_slice(&order_id.to_be_bytes());
    assert_eq!(key.encoded().as_ref(), expected.as_slice());
    assert_eq!(key.size(), 1 + AccountAddress::LENGTH * 2 + 16);
    assert_eq!(StateKey::decode(key.encoded()).unwrap(), key);
}

#[test]
fn test_bulk_order_key_encoding() {
    let key = StateKey::bulk_order(addr(5), addr(2), addr(0x34));

    let mut expected = vec![2u8, TradingNativeKeyTag::BulkOrder as u8];
    expected.extend_from_slice(addr(5).as_ref());
    expected.extend_from_slice(addr(2).as_ref());
    expected.extend_from_slice(addr(0x34).as_ref());
    assert_eq!(key.encoded().as_ref(), expected.as_slice());
    assert_eq!(key.size(), 1 + AccountAddress::LENGTH * 3);
    assert_eq!(StateKey::decode(key.encoded()).unwrap(), key);
}

#[test]
fn test_trading_native_keys_are_distinct() {
    // Same addresses under different sub-tags never collide.
    let position = StateKey::position(addr(5), addr(2), addr(0x34));
    let bulk_order = StateKey::bulk_order(addr(5), addr(2), addr(0x34));
    assert_ne!(position, bulk_order);
    assert_ne!(position.crypto_hash_ref(), bulk_order.crypto_hash_ref());
}

#[test]
fn test_trading_native_decode_rejects_bad_input() {
    let order = StateKey::order(addr(5), addr(2), 7);
    let bytes = order.encoded();
    assert!(matches!(
        StateKey::decode(&bytes[..bytes.len() - 1]),
        Err(StateKeyDecodeErr::NotEnoughBytes { .. })
    ));
    let mut long = bytes.to_vec();
    long.push(0);
    assert!(matches!(
        StateKey::decode(&long),
        Err(StateKeyDecodeErr::NotEnoughBytes { .. })
    ));
    let mut unknown = bytes.to_vec();
    unknown[1] = 0xFF;
    assert!(matches!(
        StateKey::decode(&unknown),
        Err(StateKeyDecodeErr::UnknownTradingNativeSubTag {
            unknown_sub_tag: 0xFF
        })
    ));
}

#[test]
fn test_trading_native_constructor_matches_typed() {
    let key = TradingNativeKey::Order {
        exchange: addr(5),
        market: addr(2),
        order_id: 7,
    };
    assert_eq!(
        StateKey::trading_native(key.clone()),
        StateKey::order(addr(5), addr(2), 7)
    );
    assert_eq!(
        StateKey::trading_native(key.clone()).inner(),
        &StateKeyInner::TradingNative(key)
    );
}

#[test]
fn test_trading_native_debug() {
    assert_eq!(
        format!("{:?}", StateKey::order(addr(5), addr(2), 7)),
        "StateKey::TradingNative::Order { exchange: 0x5, market: 0x2, order_id: 7 }",
    );
    assert_eq!(
        format!("{:?}", StateKey::bulk_order(addr(5), addr(2), addr(3))),
        "StateKey::TradingNative::BulkOrder { exchange: 0x5, market: 0x2, account: 0x3 }",
    );
    assert_eq!(
        format!(
            "{:?}",
            StateKey::collateral(addr(5), addr(2), CollateralScope::Cross)
        ),
        "StateKey::TradingNative::Collateral { exchange: 0x5, account: 0x2, scope: Cross }",
    );
}

proptest! {
    #[test]
    fn test_encode_decode_round_trip(key in any::<StateKey>()) {
        prop_assert_eq!(StateKey::decode(key.encoded()).unwrap(), key);
    }

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

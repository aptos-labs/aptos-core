// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::MoveHarness;
use aptos_types::{
    on_chain_config::{CurrentTimeMicroseconds, FeatureFlag},
    state_store::state_value::StateValueMetadata,
};
use move_core_types::{account_address::AccountAddress, parser::parse_struct_tag};

#[test]
fn test_metadata_tracking() {
    let mut harness = MoveHarness::new();
    harness.new_epoch(); // so that timestamp is not 0 (rather, 7200000001)
    let timestamp = CurrentTimeMicroseconds {
        microseconds: 7200000001,
    };

    let coin_store = parse_struct_tag("0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>").unwrap();

    let address1 = AccountAddress::from_hex_literal("0x100").unwrap();
    let address2 = AccountAddress::from_hex_literal("0x200").unwrap();
    let address3 = AccountAddress::from_hex_literal("0x300").unwrap();

    // create and fund account1
    let account1 = harness.new_account_at(address1);

    // Disable storage slot metadata tracking
    harness.enable_features(vec![], vec![
        FeatureFlag::STORAGE_SLOT_METADATA,
        FeatureFlag::REFUNDABLE_BYTES,
        FeatureFlag::NEW_ACCOUNTS_DEFAULT_TO_FA_STORE,
    ]);
    // Create and fund account2
    harness.run_transaction_payload(
        &account1,
        aptos_cached_packages::aptos_stdlib::aptos_account_transfer(address2, 100),
    );
    // Observe that metadata is not tracked for address2 resources
    assert_eq!(
        harness
            .read_resource_metadata(&address2, coin_store.clone())
            .unwrap(),
        StateValueMetadata::none()
    );

    // Enable storage slot metadata tracking
    harness.enable_features(
        vec![
            FeatureFlag::STORAGE_SLOT_METADATA,
            FeatureFlag::REFUNDABLE_BYTES,
        ],
        vec![],
    );
    // Create and fund account3
    harness.run_transaction_payload(
        &account1,
        aptos_cached_packages::aptos_stdlib::aptos_account_transfer(address3, 100),
    );

    // Observe that metadata is tracked for address3 resources
    let meta3a = harness
        .read_resource_metadata(&address3, coin_store.clone())
        .unwrap();
    assert!(meta3a.slot_deposit() > 0);
    assert!(meta3a.bytes_deposit() > 0);
    assert_eq!(meta3a.creation_time_usecs(), timestamp.microseconds);

    // Bump the timestamp and modify the resource, observe that metadata doesn't change.
    harness.new_epoch();
    harness.run_transaction_payload(
        &account1,
        aptos_cached_packages::aptos_stdlib::aptos_account_transfer(address3, 100),
    );
    let meta3b = harness
        .read_resource_metadata(&address3, coin_store.clone())
        .unwrap();
    assert_eq!(meta3a, meta3b);

    // However, enabling refundable_bytes does make StateValueMetadata::None automatically upgrade
    harness.run_transaction_payload(
        &account1,
        aptos_cached_packages::aptos_stdlib::aptos_account_transfer(address2, 100),
    );
    assert_eq!(
        harness
            .read_resource_metadata(&address2, coin_store.clone())
            .unwrap(),
        StateValueMetadata::new(0, 0, &CurrentTimeMicroseconds { microseconds: 0 })
    );
}

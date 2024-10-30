// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Note[Orderless]: Done
use crate::{assert_success, assert_vm_status, MoveHarness};
use aptos_package_builder::PackageBuilder;
use aptos_types::{account_address::AccountAddress, on_chain_config::FeatureFlag};
use move_core_types::vm_status::StatusCode;
use rstest::rstest;

#[rstest(stateless_account, use_txn_payload_v2_format, use_orderless_transactions, 
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true),
)]
fn lazy_natives(stateless_account: bool, use_txn_payload_v2_format: bool, use_orderless_transactions: bool) {
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap(), if stateless_account { None } else { Some(0) });
    // Set flag to publish the package.
    h.enable_features(vec![], vec![FeatureFlag::DISALLOW_USER_NATIVES]);
    let mut builder = PackageBuilder::new("LazyNatives");
    builder.add_source(
        "test",
        "
module 0xcafe::test {
    native fun undefined();

    public entry fun nothing() {}
    public entry fun something() { undefined() }
}
    ",
    );
    let dir = builder.write_to_temp().unwrap();

    // Should be able to publish with unbound native.
    assert_success!(h.publish_package(&acc, dir.path()));

    h.enable_features(vec![], vec![FeatureFlag::DISALLOW_USER_NATIVES]);
    // Should be able to call nothing entry
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::test::nothing").unwrap(),
        vec![],
        vec![]
    ));

    // Should not be able to call something entry
    let status = h.run_entry_function(
        &acc,
        str::parse("0xcafe::test::something").unwrap(),
        vec![],
        vec![],
    );
    assert_vm_status!(status, StatusCode::MISSING_DEPENDENCY)
}

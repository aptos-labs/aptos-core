// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, assert_vm_status, MoveHarness};
use aptos_package_builder::PackageBuilder;
use aptos_types::{account_address::AccountAddress, on_chain_config::FeatureFlag};
use move_core_types::vm_status::StatusCode;

#[test]
fn lazy_natives_with_stateful_sender() {
    let mut h = MoveHarness::new();
    let stateless_acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap(), Some(0));
    lazy_natives(&mut h, acc);
}

#[test]
fn lazy_natives_with_stateless_sender() {
    let mut h = MoveHarness::new();
    let stateless_acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap(), None);
    lazy_natives(&mut h, acc);
}

#[test]
fn lazy_natives(h: &mut MoveHarness, acc: Account) {
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

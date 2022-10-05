// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::tests::common;
use crate::{assert_abort, assert_success, MoveHarness};
use aptos_types::account_address::AccountAddress;
use aptos_types::on_chain_config::FeatureFlag;
use package_builder::PackageBuilder;
use rstest::rstest;

#[rstest]
#[case(vec![])]
#[case(vec![FeatureFlag::NO_LEGACY_VECTOR])]
fn legacy_vector(#[case] features: Vec<FeatureFlag>) {
    let mut h = MoveHarness::new_with_features(features.clone());

    let fx_acc = h.aptos_framework_account();

    let move_stdlib = common::framework_dir_path("move-stdlib");
    assert_success!(h.publish_package(&fx_acc, &move_stdlib));

    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    let mut builder = PackageBuilder::new("LegacyVector");
    builder.add_source(
        "test",
        "
module 0xcafe::test {
    public entry fun some() { let v = vector[]; 0x1::vector::push_back(&mut v, 1); assert!(v == vector[1], 2) }
}
    ",
    );
    builder.add_local_dep("MoveStdlib", &move_stdlib.display().to_string());
    let dir = builder.write_to_temp().unwrap();

    // Should be able to publish.
    assert_success!(h.publish_package(&acc, dir.path()));

    // Should be able to call nothing entry
    let res = h.run_entry_function(
        &acc,
        str::parse("0xcafe::test::some").unwrap(),
        vec![],
        vec![],
    );

    if features.contains(&FeatureFlag::NO_LEGACY_VECTOR) {
        assert_abort!(res, 0x5_0001)
    } else {
        assert_success!(res)
    }
}

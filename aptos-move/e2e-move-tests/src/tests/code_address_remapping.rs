// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, tests::common, MoveHarness};
use aptos_framework::natives::code::PackageRegistry;
use aptos_types::on_chain_config::FeatureFlag;
use move_core_types::{account_address::AccountAddress, parser::parse_struct_tag};
use serde::{Deserialize, Serialize};

/// Mimics `0xface::test::State`
#[derive(Serialize, Deserialize)]
struct State {
    value: u64,
}

#[test]
fn remapped_code_publishing_basic() {
    let mut h = MoveHarness::new_with_features(vec![FeatureFlag::MODULE_ADDRESS_REMAPPING], vec![]);

    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xface").unwrap());
    assert_success!(h.publish_remapped_package(
        &acc,
        &common::test_dir_path("code_publishing.data/pack_initial"),
        vec![AccountAddress::from_hex_literal("0xcafe").unwrap()],
        vec![AccountAddress::from_hex_literal("0xface").unwrap()],
    ));

    // Validate metadata as expected.
    let registry = h
        .read_resource::<PackageRegistry>(
            acc.address(),
            parse_struct_tag("0x1::code::PackageRegistry").unwrap(),
        )
        .unwrap();
    assert_eq!(registry.packages.len(), 1);
    assert_eq!(registry.packages[0].name, "test_package");
    assert_eq!(registry.packages[0].modules.len(), 1);
    assert_eq!(registry.packages[0].modules[0].name, "test");

    // Validate code loaded as expected.
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xface::test::hello").unwrap(),
        vec![],
        vec![bcs::to_bytes::<u64>(&42).unwrap()]
    ));
    let state = h
        .read_resource::<State>(
            acc.address(),
            parse_struct_tag("0xface::test::State").unwrap(),
        )
        .unwrap();
    assert_eq!(state.value, 42)
}

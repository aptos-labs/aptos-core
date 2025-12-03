// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{assert_success, tests::common, MoveHarness};
use aptos_framework::BuildOptions;
use aptos_types::account_address::AccountAddress;

#[test]
fn test_self() {
    let mut h = MoveHarness::new();
    // Load the code
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("smart_data_structures_self.data"),
        BuildOptions::move_2()
    ));
}

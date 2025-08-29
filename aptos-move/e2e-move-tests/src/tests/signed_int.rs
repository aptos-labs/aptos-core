// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Transactional tests for signed integers,
//! introduced in Move language version 2.2.3 and onwards.

use crate::{assert_success, tests::common, MoveHarness};
use aptos_framework::BuildOptions;
use aptos_types::account_address::AccountAddress;

#[test]
fn function_signed_int() {
    let mut h = MoveHarness::new();
    // Load the code
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x99").unwrap());
    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("signed_int.data/pack"),
        BuildOptions::move_2()
            .set_latest_language()
            .with_experiment("signed-int-rewrite")
    ));

    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0x99::signed_int::test_entry").unwrap(),
        vec![],
        vec![],
    ));
}

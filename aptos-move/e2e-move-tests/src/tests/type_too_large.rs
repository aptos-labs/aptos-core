// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_abort, assert_success, tests::common, MoveHarness};
use aptos_types::account_address::AccountAddress;

#[test]
fn type_too_large() {
    let mut h = MoveHarness::new();

    // Load the code
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xbeef").unwrap());
    assert_success!(h.publish_package(
        &acc,
        &common::test_dir_path("type_too_large.data/type_too_large"),
    ));

    let result = h.run_entry_function(
        &acc,
        str::parse("0xbeef::test::run").unwrap(),
        vec![],
        vec![],
    );

    // The abort code is NFE_BCS_SERIALIZATION_FAILURE = 0x1c5, since the actual VM error
    // for TOO_MANY_TYPE_NODES is hidden by the bcs serializer and turned into this generic error.
    assert_abort!(result, 0x1C5);
}

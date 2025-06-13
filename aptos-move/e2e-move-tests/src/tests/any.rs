// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_abort, assert_success, tests::common, MoveHarness};
use aptos_framework::BuildOptions;
use move_core_types::account_address::AccountAddress;

#[test]
fn test_any_with_function_values() {
    let mut h = MoveHarness::new();

    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x123").unwrap());
    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("any.data/pack"),
        BuildOptions::move_2().set_latest_language(),
    ));

    for idx in [1, 2, 3, 4] {
        let result = h.run_entry_function(
            &acc,
            str::parse(&format!(
                "0x123::any_with_function_values::roundtrip_fails_{idx}"
            ))
            .unwrap(),
            vec![],
            vec![],
        );
        assert_abort!(result, 65537);
    }

    for idx in [1, 2] {
        let result = h.run_entry_function(
            &acc,
            str::parse(&format!(
                "0x123::any_with_function_values::roundtrip_success_{idx}"
            ))
            .unwrap(),
            vec![],
            vec![],
        );
        assert_success!(result);
    }
}

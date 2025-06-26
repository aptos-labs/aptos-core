// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, MoveHarness};
use aptos_package_builder::PackageBuilder;
use aptos_types::account_address::AccountAddress;
use move_core_types::vm_status::StatusCode;

#[test]
fn lazy_natives() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

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

    // Should be able to call nothing entry
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::test::nothing").unwrap(),
        vec![],
        vec![]
    ));

    // Should not be able to call something entry
    let result = h.try_run_entry_function(
        &acc,
        str::parse("0xcafe::test::something").unwrap(),
        vec![],
        vec![],
    );

    assert!(result.is_err());
    let status = result.unwrap_err();

    assert_eq!(status.status_code(), StatusCode::MISSING_NATIVE_FUNCTION);
    assert!(status.message().unwrap().contains("`undefined`"));
}

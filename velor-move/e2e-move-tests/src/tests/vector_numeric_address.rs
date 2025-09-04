// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, tests::common, MoveHarness};
use velor_package_builder::PackageBuilder;
use velor_types::account_address::AccountAddress;

/// Test whether `0x1::vector` (and not just `std::vector`) works as expected.
#[test]
fn vector_numeric_address() {
    let mut h = MoveHarness::new();

    let fx_acc = h.velor_framework_account();

    let move_stdlib = common::framework_dir_path("move-stdlib");
    assert_success!(h.publish_package(&fx_acc, &move_stdlib));

    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    let mut builder = PackageBuilder::new("Vector");
    builder.add_source(
        "test",
        "
module 0xcafe::test {
    public entry fun some() { let v = vector[]; 0x1::vector::push_back(&mut v, 1); assert!(v == vector[1], 2) }
}
    ",
    );
    builder.add_alias("std", "0x1");
    builder.add_local_dep("MoveStdlib", &move_stdlib.display().to_string());
    let dir = builder.write_to_temp().unwrap();
    assert_success!(h.publish_package(&acc, dir.path()));

    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::test::some").unwrap(),
        vec![],
        vec![],
    ));
}

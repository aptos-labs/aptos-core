// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, MoveHarness};
use velor_package_builder::PackageBuilder;
use velor_types::account_address::AccountAddress;
use move_core_types::{u256::U256, value::MoveValue};

#[test]
fn use_new_integer_types() {
    let mut h = MoveHarness::new();

    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    let mut builder = PackageBuilder::new("test");
    builder.add_source(
        "test",
        "
module 0xcafe::test {
    public entry fun run() {
        let x: u16 = 0x8000;
        _ = x + 0x7fff;

        let x: u32 = 0x80000000;
        _ = x + 0x7fffffff;

        let x: u256 = 0x8000000000000000000000000000000000000000000000000000000000000000;
        _ = x + 0x7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff;
    }
}
    ",
    );
    let dir = builder.write_to_temp().unwrap();
    assert_success!(h.publish_package(&acc, dir.path()));

    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::test::run").unwrap(),
        vec![],
        vec![],
    ));
}

#[test]
fn new_integer_types_as_txn_arguments() {
    let mut h = MoveHarness::new();

    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    let mut builder = PackageBuilder::new("test");
    builder.add_source(
        "test",
        "
module 0xcafe::test {
    public entry fun run(a: u16, b: u32, c: u256) {
        assert!(a == 0x8000, 100);
        assert!(b == 0x80000000, 100);
        assert!(c == 0x8000000000000000000000000000000000000000000000000000000000000000, 100);
    }
}
    ",
    );
    let dir = builder.write_to_temp().unwrap();
    assert_success!(h.publish_package(&acc, dir.path()));

    let args = [
        MoveValue::U16(0x8000),
        MoveValue::U32(0x80000000),
        MoveValue::U256(
            U256::from_str_radix(
                "8000000000000000000000000000000000000000000000000000000000000000",
                16,
            )
            .unwrap(),
        ),
    ]
    .into_iter()
    .map(|val| val.simple_serialize().unwrap())
    .collect();

    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::test::run").unwrap(),
        vec![],
        args,
    ));
}

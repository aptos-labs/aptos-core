// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Note[Orderless]: Done
use crate::{assert_success, MoveHarness};
use aptos_package_builder::PackageBuilder;
use move_core_types::{u256::U256, value::MoveValue};
use rstest::rstest;

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn use_new_integer_types(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);

    let acc = h.new_account_with_key_pair(if stateless_account { None } else { Some(0) });
    let mut builder = PackageBuilder::new("test");
    builder.add_source(
        "test",
        format!(
            r#"
module {}::test {{
    public entry fun run() {{
        let x: u16 = 0x8000;
        _ = x + 0x7fff;

        let x: u32 = 0x80000000;
        _ = x + 0x7fffffff;

        let x: u256 = 0x8000000000000000000000000000000000000000000000000000000000000000;
        _ = x + 0x7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff;
    }}
}}
    "#,
            acc.address()
        )
        .as_str(),
    );
    let dir = builder.write_to_temp().unwrap();
    assert_success!(h.publish_package(&acc, dir.path()));

    assert_success!(h.run_entry_function(
        &acc,
        str::parse(format!("{}::test::run", acc.address()).as_str()).unwrap(),
        vec![],
        vec![],
    ));
}

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn new_integer_types_as_txn_arguments(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);

    let acc = h.new_account_with_key_pair(if stateless_account { None } else { Some(0) });
    let mut builder = PackageBuilder::new("test");
    builder.add_source(
        "test",
        format!(
            r#"
module {}::test {{
    public entry fun run(a: u16, b: u32, c: u256) {{
        assert!(a == 0x8000, 100);
        assert!(b == 0x80000000, 100);
        assert!(c == 0x8000000000000000000000000000000000000000000000000000000000000000, 100);
    }}
}}
    "#,
            acc.address()
        )
        .as_str(),
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
        str::parse(format!("{}::test::run", acc.address()).as_str()).unwrap(),
        vec![],
        args,
    ));
}

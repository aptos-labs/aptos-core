// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Note[Orderless]: Done
use crate::{assert_success, tests::common, MoveHarness};
use aptos_package_builder::PackageBuilder;
use rstest::rstest;

/// Test whether `0x1::vector` (and not just `std::vector`) works as expected.
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
fn vector_numeric_address(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);

    let fx_acc = h.aptos_framework_account();

    let move_stdlib = common::framework_dir_path("move-stdlib");
    assert_success!(h.publish_package(&fx_acc, &move_stdlib));

    let acc = h.new_account_with_key_pair(if stateless_account { None } else { Some(0) });
    let mut builder = PackageBuilder::new("Vector");
    builder.add_source(
        "test",
        format!(r#"
module {}::test {{
    public entry fun some() {{ let v = vector[]; 0x1::vector::push_back(&mut v, 1); assert!(v == vector[1], 2) }}
    }}
    "#, acc.address()).as_str(),
    );
    builder.add_alias("std", "0x1");
    builder.add_local_dep("MoveStdlib", &move_stdlib.display().to_string());
    let dir = builder.write_to_temp().unwrap();
    assert_success!(h.publish_package(&acc, dir.path()));

    assert_success!(h.run_entry_function(
        &acc,
        str::parse(format!("{}::test::some", acc.address()).as_str()).unwrap(),
        vec![],
        vec![],
    ));
}

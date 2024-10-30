// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Note[Orderless]: Done
use crate::{assert_success, assert_vm_status, MoveHarness};
use aptos_package_builder::PackageBuilder;
use move_core_types::vm_status::StatusCode;
use rstest::rstest;

#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
fn lazy_natives(use_txn_payload_v2_format: bool, use_orderless_transactions: bool) {
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    // Users cannot publish native functions. So, only testing with framework account.
    let acc = h.aptos_framework_account();
    let mut builder = PackageBuilder::new("LazyNatives");
    builder.add_source(
        "test",
        "
            module 0x1::test {
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
        str::parse(format!("{}::test::nothing", acc.address()).as_str()).unwrap(),
        vec![],
        vec![]
    ));

    // Should not be able to call something entry
    let status = h.run_entry_function(
        &acc,
        str::parse(format!("{}::test::something", acc.address()).as_str()).unwrap(),
        vec![],
        vec![],
    );
    assert_vm_status!(status, StatusCode::MISSING_DEPENDENCY)
}

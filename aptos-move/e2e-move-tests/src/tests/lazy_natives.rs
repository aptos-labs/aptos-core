// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, assert_vm_status, MoveHarness};
use aptos_package_builder::PackageBuilder;
use move_core_types::vm_status::StatusCode;

#[test]
fn lazy_natives() {
    let mut h = MoveHarness::new();
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
        str::parse("0x1::test::nothing").unwrap(),
        vec![],
        vec![]
    ));

    // Should not be able to call something entry
    let status = h.run_entry_function(
        &acc,
        str::parse("0x1::test::something").unwrap(),
        vec![],
        vec![],
    );
    assert_vm_status!(status, StatusCode::MISSING_DEPENDENCY)
}

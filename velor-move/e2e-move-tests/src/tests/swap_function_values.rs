// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! Test swapping of function values via vector::replace.

use crate::{assert_success, tests::common, MoveHarness};
use velor_framework::BuildOptions;
use velor_package_builder::PackageBuilder;
use velor_types::account_address::AccountAddress;

#[test]
fn swap_function_values() {
    let mut builder = PackageBuilder::new("swap_function_values");
    let source = r#"
        module 0xc0ffee::m {

            struct NoCopy;

            entry fun test() {
                let nc = NoCopy;

                let f1 = || {
                    let NoCopy = nc;
                    42
                };
                let f2 = || 44;
                let v = vector[f2];
                let f3 = v.replace(0, f1);
                assert!(f3() == 44, 0);
                let f4 = v.pop_back();
                assert!(f4() == 42, 1);
                v.destroy_empty();
            }
        }
    "#;
    builder.add_source("swap_function_values.move", source);
    builder.add_local_dep(
        "VelorStdlib",
        &common::framework_dir_path("velor-stdlib").to_string_lossy(),
    );
    let path = builder.write_to_temp().unwrap();

    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xc0ffee").unwrap());
    assert_success!(h.publish_package_with_options(
        &acc,
        path.path(),
        BuildOptions::move_2().set_latest_language()
    ));

    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xc0ffee::m::test").unwrap(),
        vec![],
        vec![],
    ));
}

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Tests for deeply-nested function values. The Move VM must ensure that it is not possible to
//! construct values that are too deep, as this can cause stack overflow.

use crate::{assert_success, tests::common, MoveHarness};
use aptos_framework::BuildOptions;
use aptos_package_builder::PackageBuilder;
use aptos_transaction_simulation::Account;
use aptos_types::transaction::TransactionStatus;
use move_core_types::account_address::AccountAddress;

#[test]
fn test_vm_value_too_deep_with_function_values() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x99").unwrap());

    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x99::m {
            public fun dummy1(_v: vector<vector<vector<vector<vector<vector<vector<vector<|| has drop+copy>>>>>>>>) {}

            public entry fun run() {
                let f: || has copy+drop = || {};

                let i = 0;
                while (i < 10000) {
                  let x = vector[vector[vector[vector[vector[vector[vector[vector[f]]]]]]]];
                  f = || dummy1(x);
                  i = i + 1;
                };
            }
        }
        "#,
    );
    assert_success!(result);
    println!("Published");
    h.run_entry_function(&acc, str::parse("0x99::m::run").unwrap(), vec![], vec![]);
}

fn publish(h: &mut MoveHarness, account: &Account, source: &str) -> TransactionStatus {
    let mut builder = PackageBuilder::new("Package");
    builder.add_source("m.move", source);
    builder.add_local_dep(
        "MoveStdlib",
        &common::framework_dir_path("move-stdlib").to_string_lossy(),
    );
    let path = builder.write_to_temp().unwrap();
    h.publish_package_with_options(
        account,
        path.path(),
        BuildOptions::move_2().set_latest_language(),
    )
}

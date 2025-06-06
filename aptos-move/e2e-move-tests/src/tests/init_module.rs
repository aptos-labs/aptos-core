// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_abort, assert_success, tests::common, MoveHarness};
use aptos_framework::{BuildOptions, BuiltPackage};
use aptos_package_builder::PackageBuilder;
use aptos_types::{
    account_address::AccountAddress,
    transaction::{ExecutionStatus, TransactionStatus},
};
use claims::assert_ok;
use move_core_types::{parser::parse_struct_tag, vm_status::StatusCode};
use serde::{Deserialize, Serialize};
use test_case::test_case;

/// Mimics `0xcafe::test::ModuleData`
#[derive(Serialize, Deserialize)]
struct ModuleData {
    global_counter: u64,
}

#[test]
fn init_module() {
    let mut h = MoveHarness::new();

    // Load the code
    let acc = h.aptos_framework_account();
    assert_success!(
        h.publish_package_cache_building(&acc, &common::test_dir_path("init_module.data/pack"))
    );

    // Verify that init_module was called.
    let module_data = parse_struct_tag("0x1::test::ModuleData").unwrap();
    assert_eq!(
        h.read_resource::<ModuleData>(acc.address(), module_data.clone())
            .unwrap()
            .global_counter,
        42
    );

    // Republish to show that init_module is not called again. If init_module would be called again,
    // we would get an abort here because the first time, it used move_to for initialization.
    assert_success!(
        h.publish_package_cache_building(&acc, &common::test_dir_path("init_module.data/pack"))
    );
    assert_eq!(
        h.read_resource::<ModuleData>(acc.address(), module_data)
            .unwrap()
            .global_counter,
        42
    );
}

#[test]
fn init_module_when_republishing_package() {
    let mut h = MoveHarness::new();

    // Deploy a package that initially does not have the module that has the init_module function.
    let acc = h.aptos_framework_account();
    assert_success!(h.publish_package_cache_building(
        &acc,
        &common::test_dir_path("init_module.data/pack_initial")
    ));

    // Now republish the package with the new module that has init_module.
    assert_success!(
        h.publish_package_cache_building(&acc, &common::test_dir_path("init_module.data/pack"))
    );

    // Verify that init_module was called.
    let module_data = parse_struct_tag("0x1::test::ModuleData").unwrap();
    assert_eq!(
        h.read_resource::<ModuleData>(acc.address(), module_data)
            .unwrap()
            .global_counter,
        42
    );
}

#[test]
fn init_module_with_abort_and_republish() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x12").unwrap());

    let mut p1 = PackageBuilder::new("Pack");
    p1.add_source(
        "m.move",
        "module 0x12::M { fun init_module(_s: &signer) { abort 1 } }",
    );
    let path1 = p1.write_to_temp().unwrap();

    let mut p2 = PackageBuilder::new("Pack");
    p2.add_source(
        "m.move",
        "module 0x12::M { fun init_module(_s: &signer) {} }",
    );
    let path2 = p2.write_to_temp().unwrap();

    let txn1 = h.create_publish_package(&acc, path1.path(), None, |_| {});
    let txn2 = h.create_publish_package(&acc, path2.path(), None, |_| {});
    let res = h.run_block(vec![txn1, txn2]);

    // First publish aborts, package should not count as published.
    assert_abort!(res[0], 1);

    // 2nd publish succeeds, not the old but the new init_module is called.
    assert_success!(res[1]);
}

#[test_case(true)]
#[test_case(false)]
fn invalid_init_module(allow_extended_checks_to_fail: bool) {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x42").unwrap());

    let package_paths = [
        "init_module.data/many_arguments",
        "init_module.data/no_arguments",
        "init_module.data/non_private",
        "init_module.data/returns_values",
        "init_module.data/type_arguments",
        "init_module.data/wrong_arguments",
    ];
    for package_path in package_paths {
        let path = common::test_dir_path(package_path).to_owned();
        let mut options = BuildOptions::default();
        let package = if allow_extended_checks_to_fail {
            options
                .experiments
                .push("skip-bailout-on-extended-checks".to_string());
            assert_ok!(BuiltPackage::build(path, options))
        } else {
            assert!(BuiltPackage::build(path, options).is_err());
            continue;
        };

        let txn = h.create_publish_built_package(&acc, &package, |_| {});
        assert!(matches!(
            h.run(txn),
            TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some(
                StatusCode::INVALID_INIT_MODULE
            )))
        ));
    }
}

// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_types::account_address::AccountAddress;
use e2e_move_tests::{assert_abort, assert_success, assert_vm_status, enable_golden, MoveHarness};
use framework::natives::code::{PackageRegistry, UpgradePolicy};
use move_deps::move_core_types::parser::parse_struct_tag;
use move_deps::move_core_types::vm_status::StatusCode;
use serde::{Deserialize, Serialize};

mod common;

/// Mimics `0xcafe::test::State`
#[derive(Serialize, Deserialize)]
struct State {
    value: u64,
}

#[test]
fn code_publishing_basic() {
    let mut h = MoveHarness::new();
    enable_golden!(h);
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    assert_success!(h.publish_package(
        &acc,
        &common::package_path("code_publishing.data/pack_initial"),
        UpgradePolicy::compat(),
    ));

    // Validate metadata as expected.
    let registry = h
        .read_resource::<PackageRegistry>(
            acc.address(),
            parse_struct_tag("0x1::code::PackageRegistry").unwrap(),
        )
        .unwrap();
    assert_eq!(registry.packages.len(), 1);
    assert_eq!(registry.packages[0].name, "test_package");
    assert_eq!(registry.packages[0].modules.len(), 1);
    assert_eq!(registry.packages[0].modules[0].name, "test");

    // Validate code loaded as expected.
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::test::hello").unwrap(),
        vec![],
        vec![bcs::to_bytes::<u64>(&42).unwrap()]
    ));
    let state = h
        .read_resource::<State>(
            acc.address(),
            parse_struct_tag("0xcafe::test::State").unwrap(),
        )
        .unwrap();
    assert_eq!(state.value, 42)
}

#[test]
fn code_publishing_upgrade_success_no_compat() {
    let mut h = MoveHarness::new();
    enable_golden!(h);
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    // Install the initial version with no compat requirements
    assert_success!(h.publish_package(
        &acc,
        &common::package_path("code_publishing.data/pack_initial"),
        UpgradePolicy::no_compat(),
    ));

    // We should be able to upgrade it with the incompatible version
    assert_success!(h.publish_package(
        &acc,
        &common::package_path("code_publishing.data/pack_upgrade_incompat"),
        UpgradePolicy::no_compat(),
    ));
}

#[test]
fn code_publishing_upgrade_success_compat() {
    let mut h = MoveHarness::new();
    enable_golden!(h);
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    // Install the initial version with compat requirements
    assert_success!(h.publish_package(
        &acc,
        &common::package_path("code_publishing.data/pack_initial"),
        UpgradePolicy::compat(),
    ));

    // We should be able to upgrade it with the compatible version
    assert_success!(h.publish_package(
        &acc,
        &common::package_path("code_publishing.data/pack_upgrade_compat"),
        UpgradePolicy::compat(),
    ));
}

#[test]
fn code_publishing_upgrade_fail_compat() {
    let mut h = MoveHarness::new();
    enable_golden!(h);
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    // Install the initial version with compat requirements
    assert_success!(h.publish_package(
        &acc,
        &common::package_path("code_publishing.data/pack_initial"),
        UpgradePolicy::compat(),
    ));

    // We should not be able to upgrade it with the incompatible version
    let status = h.publish_package(
        &acc,
        &common::package_path("code_publishing.data/pack_upgrade_incompat"),
        UpgradePolicy::compat(),
    );
    assert_vm_status!(status, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE)
}

#[test]
fn code_publishing_upgrade_fail_immutable() {
    let mut h = MoveHarness::new();
    enable_golden!(h);
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    // Install the initial version with immutable requirements
    assert_success!(h.publish_package(
        &acc,
        &common::package_path("code_publishing.data/pack_initial"),
        UpgradePolicy::immutable(),
    ));

    // We should not be able to upgrade it with the compatible version
    let status = h.publish_package(
        &acc,
        &common::package_path("code_publishing.data/pack_upgrade_compat"),
        UpgradePolicy::immutable(),
    );
    assert_abort!(status, _);
}

#[test]
fn code_publishing_upgrade_fail_overlapping_module() {
    let mut h = MoveHarness::new();
    enable_golden!(h);
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    // Install the initial version
    assert_success!(h.publish_package(
        &acc,
        &common::package_path("code_publishing.data/pack_initial"),
        UpgradePolicy::compat(),
    ));

    // Install a different package with the same module.
    let status = h.publish_package(
        &acc,
        &common::package_path("code_publishing.data/pack_other_name"),
        UpgradePolicy::compat(),
    );
    assert_abort!(status, _);
}

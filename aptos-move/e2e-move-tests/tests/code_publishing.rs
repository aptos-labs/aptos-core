// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_types::account_address::AccountAddress;
use e2e_move_tests::{assert_abort, assert_success, assert_vm_status, MoveHarness};
use framework::natives::code::PackageRegistry;
use move_deps::move_core_types::parser::parse_struct_tag;
use move_deps::move_core_types::vm_status::StatusCode;
use serde::{Deserialize, Serialize};

mod common;

/// Mimics `0xcafe::test::State`
#[derive(Serialize, Deserialize)]
struct State {
    value: u64,
}

// TODO: figure reason for flaky access to Move.toml, yielding on CI sometimes to
// thread 'code_publishing_framework_upgrade' panicked at 'building package must succeed:
// Unable to find package manifest in '/runner/_work/aptos-core/aptos-core/aptos-move/e2e-move-tests/tests/code_publishing.data/pack_stdlib'
// or in its parents', aptos-move/e2e-move-tests/src/harness.rs:181:14

#[test]
fn code_publishing_basic() {
    // Parallel execution and code publishing don't work well yet, hence all test harness created
    // here have this off
    let mut h = MoveHarness::new_no_parallel();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    assert_success!(h.publish_package(
        &acc,
        &common::test_dir_path("code_publishing.data/pack_initial"),
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

// Ignored because we've disabled incompatible upgrade policy.
#[ignore]
#[test]
fn code_publishing_upgrade_success_no_compat() {
    let mut h = MoveHarness::new_no_parallel();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    // Install the initial version with no compat requirements
    assert_success!(h.publish_package(
        &acc,
        &common::test_dir_path("code_publishing.data/pack_initial_arbitrary"),
    ));

    // We should be able to upgrade it with the incompatible version
    assert_success!(h.publish_package(
        &acc,
        &common::test_dir_path("code_publishing.data/pack_upgrade_incompat_arbitrary"),
    ));
}

#[test]
fn code_publishing_upgrade_success_compat() {
    let mut h = MoveHarness::new_no_parallel();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    // Install the initial version with compat requirements
    assert_success!(h.publish_package(
        &acc,
        &common::test_dir_path("code_publishing.data/pack_initial"),
    ));

    // We should be able to upgrade it with the compatible version
    assert_success!(h.publish_package(
        &acc,
        &common::test_dir_path("code_publishing.data/pack_upgrade_compat"),
    ));
}

#[test]
fn code_publishing_upgrade_fail_compat() {
    let mut h = MoveHarness::new_no_parallel();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    // Install the initial version with compat requirements
    assert_success!(h.publish_package(
        &acc,
        &common::test_dir_path("code_publishing.data/pack_initial"),
    ));

    // We should not be able to upgrade it with the incompatible version
    let status = h.publish_package(
        &acc,
        &common::test_dir_path("code_publishing.data/pack_upgrade_incompat"),
    );
    assert_vm_status!(status, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE)
}

#[test]
fn code_publishing_upgrade_fail_immutable() {
    let mut h = MoveHarness::new_no_parallel();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    // Install the initial version with immutable requirements
    assert_success!(h.publish_package(
        &acc,
        &common::test_dir_path("code_publishing.data/pack_initial_immutable"),
    ));

    // We should not be able to upgrade it with the compatible version
    let status = h.publish_package(
        &acc,
        &common::test_dir_path("code_publishing.data/pack_upgrade_compat"),
    );
    assert_abort!(status, _);
}

#[test]
fn code_publishing_upgrade_fail_overlapping_module() {
    let mut h = MoveHarness::new_no_parallel();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    // Install the initial version
    assert_success!(h.publish_package(
        &acc,
        &common::test_dir_path("code_publishing.data/pack_initial"),
    ));

    // Install a different package with the same module.
    let status = h.publish_package(
        &acc,
        &common::test_dir_path("code_publishing.data/pack_other_name"),
    );
    assert_abort!(status, _);
}

/// This test verifies that the cache incoherence bug on module upgrade is fixed. This bug
/// exposes itself by that after module upgrade the old version of the module stays
/// active until the MoveVM terminates. In order to workaround this until there is a better
/// fix, we flush the cache in `MoveVmExt::new_session`. One can verify the fix by commenting
/// the flush operation out, then this test fails.
///
/// TODO: for some reason this test did not capture a serious bug in `code::check_coexistence`.
#[test]
fn code_publishing_upgrade_loader_cache_consistency() {
    let mut h = MoveHarness::new_no_parallel();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    // Create a sequence of package upgrades
    let txns = vec![
        h.create_publish_package(
            &acc,
            &common::test_dir_path("code_publishing.data/pack_initial"),
            None,
        ),
        // Compatible with above package
        h.create_publish_package(
            &acc,
            &common::test_dir_path("code_publishing.data/pack_upgrade_compat"),
            None,
        ),
        // Not compatible with above package, but with first one.
        // Correct behavior: should create backward_incompatible error
        // Bug behavior: succeeds because is compared with the first module
        h.create_publish_package(
            &acc,
            &common::test_dir_path("code_publishing.data/pack_compat_first_not_second"),
            None,
        ),
    ];
    let result = h.run_block(txns);
    assert_success!(result[0]);
    assert_success!(result[1]);
    assert_vm_status!(result[2], StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE)
}

#[test]
fn code_publishing_framework_upgrade() {
    let mut h = MoveHarness::new_no_parallel();
    let acc = h.aptos_framework_account();

    // We should be able to upgrade move-stdlib, as our local package has only
    // compatible changes. (We added a new function to string.move.)
    assert_success!(h.publish_package(
        &acc,
        &common::test_dir_path("code_publishing.data/pack_stdlib"),
    ));
}

#[test]
fn code_publishing_framework_upgrade_fail() {
    let mut h = MoveHarness::new_no_parallel();
    let acc = h.aptos_framework_account();

    // We should not be able to upgrade move-stdlib because we removed a function
    // from the string module.
    let result = h.publish_package(
        &acc,
        &common::test_dir_path("code_publishing.data/pack_stdlib_incompat"),
    );
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE)
}

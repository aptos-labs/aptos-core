// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_abort, assert_success, assert_vm_status, tests::common, MoveHarness};
use aptos_types::account_address::AccountAddress;
use framework::{BuildOptions, BuiltPackage};
use move_core_types::vm_status::StatusCode;

/// Run with `cargo test <test_name> -- --nocapture` to see output.

#[test]
fn publish_init_ok1() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xbeef").unwrap());

    // publish State(v0)
    assert_success!(h.publish_package(
        &acc,
        &common::test_dir_path("publish_init_cases/generic/state_v0"),
    ));

    // publish transaction test publish_and_use
    assert_success!(h.publish_package(
        &acc,
        &common::test_dir_path("publish_init_cases/publish_and_use"),
    ));

    // publish State(v1) and UseState
    assert_success!(h.publish_package(
        &acc,
        &common::test_dir_path("publish_init_cases/generic/use_state_ok"),
    ));
}

#[test]
fn publish_init_ok2() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xbeef").unwrap());

    // publish State(v0)
    assert_success!(h.publish_package(
        &acc,
        &common::test_dir_path("publish_init_cases/generic/state_v0"),
    ));

    // publish transaction test publish_and_use
    assert_success!(h.publish_package(
        &acc,
        &common::test_dir_path("publish_init_cases/publish_and_use"),
    ));

    // build State(v1) and UseState
    let package = BuiltPackage::build(
        common::test_dir_path("publish_init_cases/generic/use_state_ok"),
        BuildOptions::default(),
    )
    .expect("building package must succeed");
    let metadata = package
        .extract_metadata()
        .expect("extracting package metadata must succeed");
    let metadata = bcs::to_bytes(&metadata).expect("PackageMetadata has BCS");
    let code = package.extract_code();
    let args = vec![
        bcs::to_bytes(&metadata).unwrap(),
        bcs::to_bytes(&code).unwrap(),
    ];

    // run transaction that publishes State(v1) and UseState
    let result = h.run_entry_function(&acc, str::parse("0xbeef::test::run").unwrap(), vec![], args);
    assert_success!(result);
}

#[test]
fn publish_init_assert1() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xbeef").unwrap());

    // publish State(v0)
    assert_success!(h.publish_package(
        &acc,
        &common::test_dir_path("publish_init_cases/generic/state_v0"),
    ));

    // publish transaction test publish_and_use
    assert_success!(h.publish_package(
        &acc,
        &common::test_dir_path("publish_init_cases/publish_and_use"),
    ));

    // publish State(v1) and UseState
    // NOTICE: if the behavior of running the init changes to run in the
    // context of the update, this test will succeed and has to be changes.
    // It is here exactly to make us notice the difference
    assert_abort!(
        h.publish_package(
            &acc,
            &common::test_dir_path("publish_init_cases/generic/use_state_assert")
        ),
        300
    );
}

#[test]
fn publish_init_assert2() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xbeef").unwrap());

    // publish State(v0)
    assert_success!(h.publish_package(
        &acc,
        &common::test_dir_path("publish_init_cases/generic/state_v0"),
    ));

    // publish transaction test publish_and_use
    assert_success!(h.publish_package(
        &acc,
        &common::test_dir_path("publish_init_cases/publish_and_use"),
    ));

    // build State(v1) and UseState
    let package = BuiltPackage::build(
        common::test_dir_path("publish_init_cases/generic/use_state_assert"),
        BuildOptions::default(),
    )
    .expect("building package must succeed");
    let metadata = package
        .extract_metadata()
        .expect("extracting package metadata must succeed");
    let metadata = bcs::to_bytes(&metadata).expect("PackageMetadata has BCS");
    let code = package.extract_code();
    let args = vec![
        bcs::to_bytes(&metadata).unwrap(),
        bcs::to_bytes(&code).unwrap(),
    ];

    // run transaction that publishes State(v1) and UseState
    let result = h.run_entry_function(&acc, str::parse("0xbeef::test::run").unwrap(), vec![], args);
    // NOTICE: if the behavior of running the init changes to run in the
    // context of the update, this test will succeed and has to be changes.
    // It is here exactly to make us notice the difference
    assert_abort!(result, 300);
}

#[test]
fn publish_init_error1() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xbeef").unwrap());

    // publish State(v0)
    assert_success!(h.publish_package(
        &acc,
        &common::test_dir_path("publish_init_cases/generic/state_v0"),
    ));

    // publish transaction test publish_and_use
    assert_success!(h.publish_package(
        &acc,
        &common::test_dir_path("publish_init_cases/publish_and_use"),
    ));

    // publish State(v1) and UseState
    let result = h.publish_package(
        &acc,
        &common::test_dir_path("publish_init_cases/generic/use_state_error"),
    );
    // NOTICE: if the behavior of running the init changes to run in the
    // context of the update, this test will succeed and has to be changes.
    // It is here exactly to make us notice the difference
    assert_vm_status!(result, StatusCode::CONSTRAINT_NOT_SATISFIED);
}

#[test]
fn publish_init_error2() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xbeef").unwrap());

    // publish State(v0)
    assert_success!(h.publish_package(
        &acc,
        &common::test_dir_path("publish_init_cases/generic/state_v0"),
    ));

    // publish transaction test publish_and_use
    assert_success!(h.publish_package(
        &acc,
        &common::test_dir_path("publish_init_cases/publish_and_use"),
    ));

    // build State(v1) and UseState
    let package = BuiltPackage::build(
        common::test_dir_path("publish_init_cases/generic/use_state_error"),
        BuildOptions::default(),
    )
    .expect("building package must succeed");
    let metadata = package
        .extract_metadata()
        .expect("extracting package metadata must succeed");
    let metadata = bcs::to_bytes(&metadata).expect("PackageMetadata has BCS");
    let code = package.extract_code();
    let args = vec![
        bcs::to_bytes(&metadata).unwrap(),
        bcs::to_bytes(&code).unwrap(),
    ];

    // run transaction that publishes State(v1) and UseState
    let result = h.run_entry_function(&acc, str::parse("0xbeef::test::run").unwrap(), vec![], args);
    // NOTICE: if the behavior of running the init changes to run in the
    // context of the update, this test will succeed and has to be changed.
    // It is here exactly to make us notice the difference
    assert_vm_status!(result, StatusCode::CONSTRAINT_NOT_SATISFIED);
}

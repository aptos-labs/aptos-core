// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Tests for upgrade compatibility
//!
//! TODO: currently this contains only tests for friend entry functions, this should be extended
//!   to test all compatibility rules in one place.

// Note: this module uses parameterized tests via the
// [`rstest` crate](https://crates.io/crates/rstest)
// to test for multiple feature combinations.
//
// Currently, we are testing both the old and the new compatibility checker

use crate::{assert_success, assert_vm_status, MoveHarness};
use aptos_framework::BuildOptions;
use aptos_package_builder::PackageBuilder;
use aptos_types::{
    account_address::AccountAddress, on_chain_config::FeatureFlag, transaction::TransactionStatus,
};
use move_core_types::vm_status::StatusCode;
use rstest::rstest;

#[rstest(use_new_checker, case(false), case(true))]
fn private_non_entry(use_new_checker: bool) {
    let result = check_upgrade("fun f(){}", "fun f(u: u16){}", use_new_checker);
    assert_success!(result)
}

#[rstest(use_new_checker, case(false), case(true))]
fn remove_function(use_new_checker: bool) {
    let result = check_upgrade("fun f(){}", "", use_new_checker);
    assert_success!(result);

    let result = check_upgrade("public fun f(){}", "", use_new_checker);
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE);

    let result = check_upgrade("public(friend) fun f(){}", "", use_new_checker);
    assert_success!(result);

    let result = check_upgrade("entry fun f(){}", "", use_new_checker);
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE);

    let result = check_upgrade("public entry fun f(){}", "", use_new_checker);
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE);

    let result = check_upgrade("public(friend) entry fun f(){}", "", use_new_checker);
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE);
}

#[rstest(use_new_checker, case(false), case(true))]
fn change_function_signature(use_new_checker: bool) {
    let result = check_upgrade("fun f(){}", "fun f(u: u16){}", use_new_checker);
    assert_success!(result);

    let result = check_upgrade(
        "public fun f(){}",
        "public fun f(u: u16){}",
        use_new_checker,
    );
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE);

    let result = check_upgrade(
        "public(friend) fun f(){}",
        "public(friend) fun f(u: u16){}",
        use_new_checker,
    );
    assert_success!(result);

    let result = check_upgrade("entry fun f(){}", "entry fun f(u: u16){}", use_new_checker);
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE);

    let result = check_upgrade(
        "public entry fun f(){}",
        "public entry fun f(u: u16){}",
        use_new_checker,
    );
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE);

    let result = check_upgrade(
        "public(friend) entry fun f(){}",
        "public(friend) entry fun f(u: u16){}",
        use_new_checker,
    );
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE);
}

#[rstest(use_new_checker, case(false), case(true))]
fn friend_add_entry(use_new_checker: bool) {
    let result = check_upgrade(
        "public(friend) fun f(){}",
        "public(friend) entry fun f(){}",
        use_new_checker,
    );
    assert_success!(result)
}

#[rstest(use_new_checker, case(false), case(true))]
fn friend_remove_entry_failure(use_new_checker: bool) {
    let result = check_upgrade(
        "public(friend) entry fun f(){}",
        "public(friend) fun f(){}",
        use_new_checker,
    );
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE)
}

#[rstest(use_new_checker, case(false), case(true))]
fn friend_remove_failure(use_new_checker: bool) {
    let result = check_upgrade("public(friend) entry fun f(){}", "", use_new_checker);
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE)
}

#[rstest(use_new_checker, case(false), case(true))]
fn friend_entry_change_sig_failure(use_new_checker: bool) {
    let result = check_upgrade(
        "public(friend) entry fun f(){}",
        "public(friend) entry fun f(_s: &signer){}",
        use_new_checker,
    );
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE)
}

#[test]
fn persistent_fun_change_success1() {
    let result = check_upgrade_latest_move("fun f(){}", "#[persistent] fun f(){}");
    assert_success!(result)
}

#[test]
fn persistent_fun_change_success2() {
    let result = check_upgrade_latest_move("#[persistent] fun f(){}", "#[persistent] fun f(){}");
    assert_success!(result)
}

#[test]
fn persistent_fun_change_failure1() {
    let result = check_upgrade_latest_move("#[persistent] fun f(){}", "fun f(){}");
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE)
}

#[test]
fn persistent_fun_change_failure2() {
    let result =
        check_upgrade_latest_move("#[persistent] fun f(){}", "#[persistent] fun f(_x: u64){}");
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE)
}

#[test]
fn persistent_fun_change_failure3() {
    let result = check_upgrade_latest_move("#[persistent] fun f(){}", "#[persistent] fun g(){}");
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE)
}

fn check_upgrade(old_decls: &str, new_decls: &str, use_new_checker: bool) -> TransactionStatus {
    check_upgrade_internal(old_decls, new_decls, use_new_checker, false)
}

fn check_upgrade_latest_move(old_decls: &str, new_decls: &str) -> TransactionStatus {
    check_upgrade_internal(old_decls, new_decls, true, true)
}

fn check_upgrade_internal(
    old_decls: &str,
    new_decls: &str,
    use_new_checker: bool,
    latest_move: bool,
) -> TransactionStatus {
    let (enabled, disabled) = if use_new_checker {
        (vec![FeatureFlag::USE_COMPATIBILITY_CHECKER_V2], vec![])
    } else {
        (vec![], vec![FeatureFlag::USE_COMPATIBILITY_CHECKER_V2])
    };
    let mut builder = PackageBuilder::new("Package");
    let mut h = MoveHarness::new_with_features(enabled, disabled);
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x815").unwrap());

    // Publish for first time
    builder.add_source(
        "m.move",
        &format!(
            r#"
            module 0x815::m {{
              {}
            }}
            "#,
            old_decls
        ),
    );
    let path = builder.write_to_temp().unwrap();
    let opts = if latest_move {
        BuildOptions::move_2().set_latest_language()
    } else {
        BuildOptions::move_2()
    };
    assert_success!(h.publish_package_with_options(&acc, path.path(), opts.clone()));

    // Now upgrade
    let mut builder = PackageBuilder::new("Package");
    builder.add_source(
        "m.move",
        &format!(
            r#"
            module 0x815::m {{
              {}
            }}
            "#,
            new_decls
        ),
    );
    let path = builder.write_to_temp().unwrap();
    h.publish_package_with_options(&acc, path.path(), opts)
}

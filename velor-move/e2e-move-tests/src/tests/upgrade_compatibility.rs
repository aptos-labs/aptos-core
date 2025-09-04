// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! Tests for upgrade compatibility
//!
//! TODO: currently this contains only tests for friend entry functions, this should be extended
//!   to test all compatibility rules in one place.

use crate::{assert_success, assert_vm_status, MoveHarness};
use velor_framework::BuildOptions;
use velor_package_builder::PackageBuilder;
use velor_types::{account_address::AccountAddress, transaction::TransactionStatus};
use move_core_types::vm_status::StatusCode;

#[test]
fn private_non_entry() {
    let result = check_upgrade("fun f(){}", "fun f(u: u16){}");
    assert_success!(result)
}

#[test]
fn remove_function() {
    let result = check_upgrade("fun f(){}", "");
    assert_success!(result);

    let result = check_upgrade("public fun f(){}", "");
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE);

    let result = check_upgrade("public(friend) fun f(){}", "");
    assert_success!(result);

    let result = check_upgrade("entry fun f(){}", "");
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE);

    let result = check_upgrade("public entry fun f(){}", "");
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE);

    let result = check_upgrade("public(friend) entry fun f(){}", "");
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE);
}

#[test]
fn change_function_signature() {
    let result = check_upgrade("fun f(){}", "fun f(u: u16){}");
    assert_success!(result);

    let result = check_upgrade("public fun f(){}", "public fun f(u: u16){}");
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE);

    let result = check_upgrade("public(friend) fun f(){}", "public(friend) fun f(u: u16){}");
    assert_success!(result);

    let result = check_upgrade("entry fun f(){}", "entry fun f(u: u16){}");
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE);

    let result = check_upgrade("public entry fun f(){}", "public entry fun f(u: u16){}");
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE);

    let result = check_upgrade(
        "public(friend) entry fun f(){}",
        "public(friend) entry fun f(u: u16){}",
    );
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE);
}

#[test]
fn friend_add_entry() {
    let result = check_upgrade("public(friend) fun f(){}", "public(friend) entry fun f(){}");
    assert_success!(result)
}

#[test]
fn friend_remove_entry_failure() {
    let result = check_upgrade("public(friend) entry fun f(){}", "public(friend) fun f(){}");
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE)
}

#[test]
fn friend_remove_failure() {
    let result = check_upgrade("public(friend) entry fun f(){}", "");
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE)
}

#[test]
fn friend_entry_change_sig_failure() {
    let result = check_upgrade(
        "public(friend) entry fun f(){}",
        "public(friend) entry fun f(_s: &signer){}",
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

fn check_upgrade(old_decls: &str, new_decls: &str) -> TransactionStatus {
    check_upgrade_internal(old_decls, new_decls, false)
}

fn check_upgrade_latest_move(old_decls: &str, new_decls: &str) -> TransactionStatus {
    check_upgrade_internal(old_decls, new_decls, true)
}

fn check_upgrade_internal(
    old_decls: &str,
    new_decls: &str,
    latest_move: bool,
) -> TransactionStatus {
    let mut builder = PackageBuilder::new("Package");
    let mut h = MoveHarness::new();
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

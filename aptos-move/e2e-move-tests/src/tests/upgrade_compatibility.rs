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

// Note[Orderless]: Done
use crate::{assert_success, assert_vm_status, MoveHarness};
use aptos_framework::BuildOptions;
use aptos_package_builder::PackageBuilder;
use aptos_types::{
    account_address::AccountAddress, on_chain_config::FeatureFlag, transaction::TransactionStatus,
};
use move_core_types::vm_status::StatusCode;
use rstest::rstest;

#[rstest(use_new_checker, stateless_account, use_txn_payload_v2_format, use_orderless_transactions,
    case(true, true, false, false),
    case(true, true, true, false),
    case(true, true, true, true),
    case(true, false, false, false),
    case(true, false, true, false),
    case(true, false, true, true),
    case(false, true, false, false),
    case(false, true, true, false),
    case(false, true, true, true),
    case(false, false, false, false),
    case(false, false, true, false),
    case(false, false, true, true),
)]
fn private_non_entry(use_new_checker: bool, stateless_account: bool, use_txn_payload_v2_format: bool, use_orderless_transactions: bool) {
    let result = check_upgrade("fun f(){}", "fun f(u: u16){}", use_new_checker, stateless_account, use_txn_payload_v2_format, use_orderless_transactions);
    assert_success!(result)
}

#[rstest(use_new_checker, stateless_account, use_txn_payload_v2_format, use_orderless_transactions,
    case(true, true, false, false),
    case(true, true, true, false),
    case(true, true, true, true),
    case(true, false, false, false),
    case(true, false, true, false),
    case(true, false, true, true),
    case(false, true, false, false),
    case(false, true, true, false),
    case(false, true, true, true),
    case(false, false, false, false),
    case(false, false, true, false),
    case(false, false, true, true),
)]
fn remove_function(use_new_checker: bool, stateless_account: bool, use_txn_payload_v2_format: bool, use_orderless_transactions: bool) {
    let result = check_upgrade("fun f(){}", "", use_new_checker, stateless_account, use_txn_payload_v2_format, use_orderless_transactions);
    assert_success!(result);

    let result = check_upgrade("public fun f(){}", "", use_new_checker, stateless_account, use_txn_payload_v2_format, use_orderless_transactions);
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE);

    let result = check_upgrade("public(friend) fun f(){}", "", use_new_checker, stateless_account, use_txn_payload_v2_format, use_orderless_transactions);
    assert_success!(result);

    let result = check_upgrade("entry fun f(){}", "", use_new_checker, stateless_account, use_txn_payload_v2_format, use_orderless_transactions);
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE);

    let result = check_upgrade("public entry fun f(){}", "", use_new_checker, stateless_account, use_txn_payload_v2_format, use_orderless_transactions);
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE);

    let result = check_upgrade("public(friend) entry fun f(){}", "", use_new_checker, stateless_account, use_txn_payload_v2_format, use_orderless_transactions);
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE);
}

#[rstest(use_new_checker, stateless_account, use_txn_payload_v2_format, use_orderless_transactions,
    case(true, true, false, false),
    case(true, true, true, false),
    case(true, true, true, true),
    case(true, false, false, false),
    case(true, false, true, false),
    case(true, false, true, true),
    case(false, true, false, false),
    case(false, true, true, false),
    case(false, true, true, true),
    case(false, false, false, false),
    case(false, false, true, false),
    case(false, false, true, true),
)]
fn change_function_signature(use_new_checker: bool, stateless_account: bool, use_txn_payload_v2_format: bool, use_orderless_transactions: bool) {
    let result = check_upgrade("fun f(){}", "fun f(u: u16){}", use_new_checker, stateless_account, use_txn_payload_v2_format, use_orderless_transactions);
    assert_success!(result);

    let result = check_upgrade(
        "public fun f(){}",
        "public fun f(u: u16){}",
        use_new_checker,
        stateless_account,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE);

    let result = check_upgrade(
        "public(friend) fun f(){}",
        "public(friend) fun f(u: u16){}",
        use_new_checker,
        stateless_account,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    assert_success!(result);

    let result = check_upgrade("entry fun f(){}", "entry fun f(u: u16){}", use_new_checker, stateless_account, use_txn_payload_v2_format, use_orderless_transactions);
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE);

    let result = check_upgrade(
        "public entry fun f(){}",
        "public entry fun f(u: u16){}",
        use_new_checker,
        stateless_account,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE);

    let result = check_upgrade(
        "public(friend) entry fun f(){}",
        "public(friend) entry fun f(u: u16){}",
        use_new_checker,
        stateless_account,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE);
}

#[rstest(use_new_checker, stateless_account, use_txn_payload_v2_format, use_orderless_transactions,
    case(true, true, false, false),
    case(true, true, true, false),
    case(true, true, true, true),
    case(true, false, false, false),
    case(true, false, true, false),
    case(true, false, true, true),
    case(false, true, false, false),
    case(false, true, true, false),
    case(false, true, true, true),
    case(false, false, false, false),
    case(false, false, true, false),
    case(false, false, true, true),
)]
fn friend_add_entry(use_new_checker: bool, stateless_account: bool, use_txn_payload_v2_format: bool, use_orderless_transactions: bool) {
    let result = check_upgrade(
        "public(friend) fun f(){}",
        "public(friend) entry fun f(){}",
        use_new_checker,
        stateless_account,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    assert_success!(result)
}

#[rstest(use_new_checker, stateless_account, use_txn_payload_v2_format, use_orderless_transactions,
    case(true, true, false, false),
    case(true, true, true, false),
    case(true, true, true, true),
    case(true, false, false, false),
    case(true, false, true, false),
    case(true, false, true, true),
    case(false, true, false, false),
    case(false, true, true, false),
    case(false, true, true, true),
    case(false, false, false, false),
    case(false, false, true, false),
    case(false, false, true, true),
)]
fn friend_remove_entry_failure(use_new_checker: bool, stateless_account: bool, use_txn_payload_v2_format: bool, use_orderless_transactions: bool) {
    let result = check_upgrade(
        "public(friend) entry fun f(){}",
        "public(friend) fun f(){}",
        use_new_checker,
        stateless_account,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE)
}

#[rstest(use_new_checker, stateless_account, use_txn_payload_v2_format, use_orderless_transactions,
    case(true, true, false, false),
    case(true, true, true, false),
    case(true, true, true, true),
    case(true, false, false, false),
    case(true, false, true, false),
    case(true, false, true, true),
    case(false, true, false, false),
    case(false, true, true, false),
    case(false, true, true, true),
    case(false, false, false, false),
    case(false, false, true, false),
    case(false, false, true, true),
)]
fn friend_remove_failure(use_new_checker: bool, stateless_account: bool, use_txn_payload_v2_format: bool, use_orderless_transactions: bool) {
    let result = check_upgrade("public(friend) entry fun f(){}", "", use_new_checker, stateless_account, use_txn_payload_v2_format, use_orderless_transactions);
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE)
}

#[rstest(use_new_checker, stateless_account, use_txn_payload_v2_format, use_orderless_transactions,
    case(true, true, false, false),
    case(true, true, true, false),
    case(true, true, true, true),
    case(true, false, false, false),
    case(true, false, true, false),
    case(true, false, true, true),
    case(false, true, false, false),
    case(false, true, true, false),
    case(false, true, true, true),
    case(false, false, false, false),
    case(false, false, true, false),
    case(false, false, true, true),
)]
fn friend_entry_change_sig_failure(use_new_checker: bool, stateless_account: bool, use_txn_payload_v2_format: bool, use_orderless_transactions: bool) {
    let result = check_upgrade(
        "public(friend) entry fun f(){}",
        "public(friend) entry fun f(_s: &signer){}",
        use_new_checker,
        stateless_account,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE)
}

fn check_upgrade(old_decls: &str, new_decls: &str, use_new_checker: bool, stateless_account: bool, use_txn_payload_v2_format: bool, use_orderless_transactions: bool) -> TransactionStatus {
    let (enabled, disabled) = if use_new_checker {
        (vec![FeatureFlag::USE_COMPATIBILITY_CHECKER_V2], vec![])
    } else {
        (vec![], vec![FeatureFlag::USE_COMPATIBILITY_CHECKER_V2])
    };
    let mut builder = PackageBuilder::new("Package");
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    h.enable_features(enabled, disabled);
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x815").unwrap(), if stateless_account { None } else { Some(0) });

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
    assert_success!(h.publish_package_with_options(&acc, path.path(), BuildOptions::move_2()));

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
    h.publish_package_with_options(&acc, path.path(), BuildOptions::move_2())
}

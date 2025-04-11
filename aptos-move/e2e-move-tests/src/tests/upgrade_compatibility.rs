// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Tests for upgrade compatibility
//!
//! TODO: currently this contains only tests for friend entry functions, this should be extended
//!   to test all compatibility rules in one place.

// Note[Orderless]: Done
use crate::{assert_success, assert_vm_status, MoveHarness};
use aptos_framework::BuildOptions;
use aptos_package_builder::PackageBuilder;
use aptos_types::transaction::TransactionStatus;
use move_core_types::vm_status::StatusCode;
use rstest::rstest;

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn private_non_entry(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let result = check_upgrade(
        "fun f(){}",
        "fun f(u: u16){}",
        stateless_account,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    assert_success!(result)
}

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn remove_function(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let result = check_upgrade(
        "fun f(){}",
        "",
        stateless_account,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    assert_success!(result);

    let result = check_upgrade(
        "public fun f(){}",
        "",
        stateless_account,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE);

    let result = check_upgrade(
        "public(friend) fun f(){}",
        "",
        stateless_account,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    assert_success!(result);

    let result = check_upgrade(
        "entry fun f(){}",
        "",
        stateless_account,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE);

    let result = check_upgrade(
        "public entry fun f(){}",
        "",
        stateless_account,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE);

    let result = check_upgrade(
        "public(friend) entry fun f(){}",
        "",
        stateless_account,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE);
}

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn change_function_signature(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let result = check_upgrade(
        "fun f(){}",
        "fun f(u: u16){}",
        stateless_account,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    assert_success!(result);

    let result = check_upgrade(
        "public fun f(){}",
        "public fun f(u: u16){}",
        stateless_account,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE);

    let result = check_upgrade(
        "public(friend) fun f(){}",
        "public(friend) fun f(u: u16){}",
        stateless_account,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    assert_success!(result);

    let result = check_upgrade(
        "entry fun f(){}",
        "entry fun f(u: u16){}",
        stateless_account,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE);

    let result = check_upgrade(
        "public entry fun f(){}",
        "public entry fun f(u: u16){}",
        stateless_account,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE);

    let result = check_upgrade(
        "public(friend) entry fun f(){}",
        "public(friend) entry fun f(u: u16){}",
        stateless_account,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE);
}

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn friend_add_entry(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let result = check_upgrade(
        "public(friend) fun f(){}",
        "public(friend) entry fun f(){}",
        stateless_account,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    assert_success!(result)
}

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn friend_remove_entry_failure(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let result = check_upgrade(
        "public(friend) entry fun f(){}",
        "public(friend) fun f(){}",
        stateless_account,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE)
}

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn friend_remove_failure(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let result = check_upgrade(
        "public(friend) entry fun f(){}",
        "",
        stateless_account,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE)
}

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn friend_entry_change_sig_failure(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let result = check_upgrade(
        "public(friend) entry fun f(){}",
        "public(friend) entry fun f(_s: &signer){}",
        stateless_account,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE)
}

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn persistent_fun_change_success1(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let result = check_upgrade_latest_move(
        "fun f(){}",
        "#[persistent] fun f(){}",
        stateless_account,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    assert_success!(result)
}

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn persistent_fun_change_success2(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let result = check_upgrade_latest_move(
        "#[persistent] fun f(){}",
        "#[persistent] fun f(){}",
        stateless_account,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    assert_success!(result)
}

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn persistent_fun_change_failure1(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let result = check_upgrade_latest_move(
        "#[persistent] fun f(){}",
        "fun f(){}",
        stateless_account,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE)
}

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn persistent_fun_change_failure2(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let result = check_upgrade_latest_move(
        "#[persistent] fun f(){}",
        "#[persistent] fun f(_x: u64){}",
        stateless_account,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE)
}

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn persistent_fun_change_failure3(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let result = check_upgrade_latest_move(
        "#[persistent] fun f(){}",
        "#[persistent] fun g(){}",
        stateless_account,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE)
}

fn check_upgrade(
    old_decls: &str,
    new_decls: &str,
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) -> TransactionStatus {
    check_upgrade_internal(
        old_decls,
        new_decls,
        false,
        stateless_account,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    )
}

fn check_upgrade_latest_move(
    old_decls: &str,
    new_decls: &str,
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) -> TransactionStatus {
    check_upgrade_internal(
        old_decls,
        new_decls,
        true,
        stateless_account,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    )
}

fn check_upgrade_internal(
    old_decls: &str,
    new_decls: &str,
    latest_move: bool,
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) -> TransactionStatus {
    let mut builder = PackageBuilder::new("Package");
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    let acc = h.new_account_with_key_pair(if stateless_account { None } else { Some(0) });

    // Publish for first time
    builder.add_source(
        "m.move",
        &format!(
            r#"
            module {}::m {{
              {}
            }}
            "#,
            acc.address(),
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
            module {}::m {{
              {}
            }}
            "#,
            acc.address(),
            new_decls
        ),
    );
    let path = builder.write_to_temp().unwrap();
    h.publish_package_with_options(&acc, path.path(), opts)
}

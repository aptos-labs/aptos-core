// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! E2E upgrade tests for `#[immutable]` on functions.

use crate::{assert_success, assert_vm_status, MoveHarness};
use aptos_framework::BuildOptions;
use aptos_language_e2e_tests::account::Account;
use aptos_package_builder::PackageBuilder;
use aptos_types::{account_address::AccountAddress, transaction::TransactionStatus};
use move_core_types::vm_status::StatusCode;

fn publish(h: &mut MoveHarness, account: &Account, source: &str) -> TransactionStatus {
    let mut builder = PackageBuilder::new("Package");
    builder.add_source("m.move", source);
    let path = builder.write_to_temp().unwrap();
    h.publish_package_with_options(
        account,
        path.path(),
        BuildOptions::move_2().set_latest_language(),
    )
}

// ---------------------------------------------------------------------------
// #[immutable] on public functions
// ---------------------------------------------------------------------------

/// Upgrading an `#[immutable]` function with the same body is allowed.
#[test]
fn immutable_fun_same_body_ok() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x900").unwrap());

    assert_success!(publish(
        &mut h,
        &acc,
        r#"
        module 0x900::m {
            #[immutable]
            public fun value(): u64 { 42 }
        }
    "#,
    ));

    // Re-publishing with identical body is compatible.
    assert_success!(publish(
        &mut h,
        &acc,
        r#"
        module 0x900::m {
            #[immutable]
            public fun value(): u64 { 42 }
        }
    "#,
    ));
}

/// Changing the body of an `#[immutable]` function is rejected.
#[test]
fn immutable_fun_body_changed_rejected() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x901").unwrap());

    assert_success!(publish(
        &mut h,
        &acc,
        r#"
        module 0x901::m {
            #[immutable]
            public fun value(): u64 { 42 }
        }
    "#,
    ));

    // Changed return value → body differs → incompatible.
    assert_vm_status!(
        publish(
            &mut h,
            &acc,
            r#"
            module 0x901::m {
                #[immutable]
                public fun value(): u64 { 99 }
            }
        "#,
        ),
        StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE
    );
}

/// Removing `#[immutable]` from a public function is rejected (attribute cannot be removed).
#[test]
fn immutable_fun_attribute_removed_rejected() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x902").unwrap());

    assert_success!(publish(
        &mut h,
        &acc,
        r#"
        module 0x902::m {
            #[immutable]
            public fun value(): u64 { 42 }
        }
    "#,
    ));

    // Dropping #[immutable] is incompatible.
    assert_vm_status!(
        publish(
            &mut h,
            &acc,
            r#"
            module 0x902::m {
                public fun value(): u64 { 42 }
            }
        "#,
        ),
        StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE
    );
}

/// Adding `#[immutable]` to a previously non-immutable public function is allowed,
/// and the body may be changed in the same upgrade that seals it.
#[test]
fn immutable_fun_add_attribute_ok() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x903").unwrap());

    assert_success!(publish(
        &mut h,
        &acc,
        r#"
        module 0x903::m {
            public fun value(): u64 { 42 }
        }
    "#,
    ));

    // Adding #[immutable] (with the same body) is compatible.
    assert_success!(publish(
        &mut h,
        &acc,
        r#"
        module 0x903::m {
            #[immutable]
            public fun value(): u64 { 43 }
        }
    "#,
    ));
}

/// Without `#[immutable]`, changing a public function body is allowed.
#[test]
fn non_immutable_fun_body_change_ok() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x904").unwrap());

    assert_success!(publish(
        &mut h,
        &acc,
        r#"
        module 0x904::m {
            public fun value(): u64 { 1 }
        }
    "#,
    ));

    // No #[immutable] → body change is compatible.
    assert_success!(publish(
        &mut h,
        &acc,
        r#"
        module 0x904::m {
            public fun value(): u64 { 999 }
        }
    "#,
    ));
}

// ---------------------------------------------------------------------------
// #[immutable] on non-public (private) functions
// ---------------------------------------------------------------------------

/// A private `#[immutable]` helper with the same body is allowed on upgrade.
#[test]
fn immutable_private_fun_same_body_ok() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x905").unwrap());

    assert_success!(publish(
        &mut h,
        &acc,
        r#"
        module 0x905::m {
            #[immutable]
            fun helper(): u64 { 42 }
            #[immutable]
            public fun value(): u64 { helper() }
        }
    "#,
    ));

    assert_success!(publish(
        &mut h,
        &acc,
        r#"
        module 0x905::m {
            #[immutable]
            fun helper(): u64 { 42 }
            #[immutable]
            public fun value(): u64 { helper() }
        }
    "#,
    ));
}

/// Changing the body of a private `#[immutable]` function is rejected.
#[test]
fn immutable_private_fun_body_changed_rejected() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x906").unwrap());

    assert_success!(publish(
        &mut h,
        &acc,
        r#"
        module 0x906::m {
            #[immutable]
            fun helper(): u64 { 1 }
            #[immutable]
            public fun value(): u64 { helper() }
        }
    "#,
    ));

    assert_vm_status!(
        publish(
            &mut h,
            &acc,
            r#"
            module 0x906::m {
                #[immutable]
                fun helper(): u64 { 2 }
                #[immutable]
                public fun value(): u64 { helper() }
            }
        "#,
        ),
        StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE
    );
}

/// Removing `#[immutable]` from a private function is rejected.
#[test]
fn immutable_private_fun_attribute_removed_rejected() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x907").unwrap());

    assert_success!(publish(
        &mut h,
        &acc,
        r#"
        module 0x907::m {
            #[immutable]
            fun helper(): u64 { 1 }
            #[immutable]
            public fun value(): u64 { helper() }
        }
    "#,
    ));

    assert_vm_status!(
        publish(
            &mut h,
            &acc,
            r#"
            module 0x907::m {
                fun helper(): u64 { 1 }
                #[immutable]
                public fun value(): u64 { 1 }
            }
        "#,
        ),
        StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE
    );
}

/// Adding `#[immutable]` to a private function is allowed.
#[test]
fn immutable_private_fun_add_attribute_ok() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x908").unwrap());

    assert_success!(publish(
        &mut h,
        &acc,
        r#"
        module 0x908::m {
            fun helper(): u64 { 1 }
            public fun value(): u64 { helper() }
        }
    "#,
    ));

    assert_success!(publish(
        &mut h,
        &acc,
        r#"
        module 0x908::m {
            #[immutable]
            fun helper(): u64 { 1 }
            #[immutable]
            public fun value(): u64 { helper() }
        }
    "#,
    ));
}

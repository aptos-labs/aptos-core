// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests for upgrade compatibility of public/package/friend constants.

use crate::{assert_success, assert_vm_status, MoveHarness};
use aptos_framework::BuildOptions;
use aptos_language_e2e_tests::account::Account;
use aptos_package_builder::PackageBuilder;
use aptos_types::{
    account_address::AccountAddress, on_chain_config::FeatureFlag, transaction::TransactionStatus,
};
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
// public const upgrade tests
// ---------------------------------------------------------------------------

/// Adding a new public constant to an existing module is always compatible.
#[test]
fn public_const_upgrade_add() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x820").unwrap());

    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x820::m {
            public const A: u64 = 1;
        }
    "#,
    );
    assert_success!(result);

    // Adding a second public constant is backward-compatible.
    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x820::m {
            public const A: u64 = 1;
            public const B: u64 = 2;
        }
    "#,
    );
    assert_success!(result);
}

/// Removing a public constant breaks callers that reference its accessor function.
#[test]
fn public_const_upgrade_remove() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x821").unwrap());

    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x821::m {
            public const VALUE: u64 = 42;
        }
    "#,
    );
    assert_success!(result);

    // Removing VALUE removes its const$VALUE accessor function — incompatible.
    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x821::m {
        }
    "#,
    );
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE);
}

/// Changing the value of a public constant is compatible: callers call through
/// the const$NAME accessor function, so they automatically get the new value.
/// The accessor function's signature (name, visibility, return type) does not change.
#[test]
fn public_const_upgrade_value_change() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x822").unwrap());

    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x822::m {
            public const VALUE: u64 = 10;
        }
    "#,
    );
    assert_success!(result);

    // Changing the value is compatible: the accessor's signature is unchanged.
    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x822::m {
            public const VALUE: u64 = 99;
        }
    "#,
    );
    assert_success!(result);
}

/// Narrowing visibility from `public` to private is incompatible because it
/// removes the `public` const$NAME accessor function.
#[test]
fn public_const_upgrade_narrow_to_private() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x823").unwrap());

    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x823::m {
            public const VALUE: u64 = 1;
        }
    "#,
    );
    assert_success!(result);

    // Narrowing to private drops the public accessor — incompatible.
    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x823::m {
            const VALUE: u64 = 1;
        }
    "#,
    );
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE);
}

/// Narrowing visibility from `public` to `package` is incompatible because it
/// downgrades the accessor from `public` to `friend` visibility.
#[test]
fn public_const_upgrade_narrow_to_package() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x824").unwrap());

    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x824::m {
            public const VALUE: u64 = 1;
        }
    "#,
    );
    assert_success!(result);

    // Downgrading public → package lowers the accessor's visibility — incompatible.
    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x824::m {
            package const VALUE: u64 = 1;
        }
    "#,
    );
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE);
}

// ---------------------------------------------------------------------------
// package const upgrade tests
// ---------------------------------------------------------------------------

/// Adding a new package constant to an existing module is always compatible.
#[test]
fn package_const_upgrade_add() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x825").unwrap());

    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x825::m {
            package const A: u64 = 10;
        }
    "#,
    );
    assert_success!(result);

    // Adding another package constant is compatible.
    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x825::m {
            package const A: u64 = 10;
            package const B: u64 = 20;
        }
    "#,
    );
    assert_success!(result);
}

/// Whether removing a package constant is compatible depends on the
/// `TREAT_FRIEND_AS_PRIVATE` feature flag.
///
/// The `package const` accessor has `friend` visibility.  When
/// `TREAT_FRIEND_AS_PRIVATE` is **enabled** (the production default) the
/// compatibility checker treats friend functions as private, so removing the
/// accessor is allowed.  When the flag is **disabled**, friend-linking is
/// enforced and removing the accessor is incompatible.
#[test]
fn package_const_upgrade_remove_treat_friend_as_private_on() {
    // Default harness has TREAT_FRIEND_AS_PRIVATE enabled → removing is OK.
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x826").unwrap());

    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x826::m {
            package const PKG: u64 = 77;
        }
    "#,
    );
    assert_success!(result);

    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x826::m {
        }
    "#,
    );
    // Friend functions treated as private → removal is compatible.
    assert_success!(result);
}

#[test]
fn package_const_upgrade_remove_treat_friend_as_private_off() {
    // Disable TREAT_FRIEND_AS_PRIVATE → friend-linking is enforced.
    let mut h = MoveHarness::new_with_features(vec![], vec![FeatureFlag::TREAT_FRIEND_AS_PRIVATE]);
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x826").unwrap());

    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x826::m {
            package const PKG: u64 = 77;
        }
    "#,
    );
    assert_success!(result);

    // Removing PKG removes its friend-visibility accessor — incompatible.
    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x826::m {
        }
    "#,
    );
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE);
}

/// Changing the value of a package constant is compatible.
#[test]
fn package_const_upgrade_value_change() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x827").unwrap());

    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x827::m {
            package const PKG: u64 = 1;
        }
    "#,
    );
    assert_success!(result);

    // Changing the value is compatible.
    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x827::m {
            package const PKG: u64 = 999;
        }
    "#,
    );
    assert_success!(result);
}

/// Upgrading a package constant to public is compatible (visibility broadened).
#[test]
fn package_const_upgrade_to_public() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x828").unwrap());

    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x828::m {
            package const VALUE: u64 = 5;
        }
    "#,
    );
    assert_success!(result);

    // Upgrading package → public broadens access — compatible.
    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x828::m {
            public const VALUE: u64 = 5;
        }
    "#,
    );
    assert_success!(result);
}

/// Narrowing a package constant to private removes the friend accessor.
/// Compatibility depends on the `TREAT_FRIEND_AS_PRIVATE` feature flag,
/// for the same reasons as `package_const_upgrade_remove_*` above.
#[test]
fn package_const_upgrade_narrow_to_private_treat_friend_as_private_on() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x829").unwrap());

    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x829::m {
            package const VALUE: u64 = 5;
        }
    "#,
    );
    assert_success!(result);

    // Friend accessor removed, but treated as private → compatible.
    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x829::m {
            const VALUE: u64 = 5;
        }
    "#,
    );
    assert_success!(result);
}

#[test]
fn package_const_upgrade_narrow_to_private_treat_friend_as_private_off() {
    let mut h = MoveHarness::new_with_features(vec![], vec![FeatureFlag::TREAT_FRIEND_AS_PRIVATE]);
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x829").unwrap());

    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x829::m {
            package const VALUE: u64 = 5;
        }
    "#,
    );
    assert_success!(result);

    // Narrowing package → private removes the friend accessor — incompatible.
    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x829::m {
            const VALUE: u64 = 5;
        }
    "#,
    );
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE);
}

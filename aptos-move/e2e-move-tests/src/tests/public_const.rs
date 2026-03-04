// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! E2E tests for public constants feature (language version 2.5+).
//!
//! Verifies that `public const` declarations compile and are accessible
//! cross-module, with their values correctly inlined at the call site.

use crate::{assert_success, MoveHarness};
use aptos_language_e2e_tests::account::Account;
use aptos_package_builder::PackageBuilder;
use aptos_types::{account_address::AccountAddress, transaction::TransactionStatus};
use serde::{Deserialize, Serialize};

/// Mirrors `0xCAFE::consumer::Result` in the test Move package.
#[derive(Serialize, Deserialize, Debug)]
struct Result {
    max_value: u64,
    version: u8,
    enabled: bool,
}

fn publish(h: &mut MoveHarness, account: &Account, sources: &[(&str, &str)]) -> TransactionStatus {
    let mut builder = PackageBuilder::new("Package");
    for (name, source) in sources {
        builder.add_source(name, source);
    }
    let path = builder.write_to_temp().unwrap();
    h.publish_package_with_options(
        account,
        path.path(),
        aptos_framework::BuildOptions::move_2().set_latest_language(),
    )
}

/// Test that a module with public constants compiles, publishes, and can be
/// consumed from another module with the correct values.
#[test]
fn test_public_const_cross_module() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    assert_success!(publish(&mut h, &acc, &[
        (
            "constants.move",
            r#"
            module 0xcafe::constants {
                public const MAX_VALUE: u64 = 100;
                public const VERSION: u8 = 42;
                public const ENABLED: bool = true;
                const PRIVATE_SECRET: u64 = 999;
                public fun get_private(): u64 { PRIVATE_SECRET }
            }
        "#
        ),
        (
            "consumer.move",
            r#"
            module 0xcafe::consumer {
                use 0xcafe::constants;
                struct Result has key { max_value: u64, version: u8, enabled: bool }
                public entry fun store_constants(account: &signer) {
                    move_to(account, Result {
                        max_value: constants::MAX_VALUE,
                        version: constants::VERSION,
                        enabled: constants::ENABLED,
                    });
                }
            }
        "#
        ),
    ]));

    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::consumer::store_constants").unwrap(),
        vec![],
        vec![],
    ));

    let result: Result = h
        .read_resource(
            acc.address(),
            str::parse("0xcafe::consumer::Result").unwrap(),
        )
        .unwrap();

    assert_eq!(result.max_value, 100);
    assert_eq!(result.version, 42);
    assert!(result.enabled);
}

/// Test that `package const` is accessible cross-module when both modules are compiled
/// together in the same package.
#[test]
fn test_package_const_cross_module() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    assert_success!(publish(&mut h, &acc, &[(
        "m.move",
        r#"
            module 0xcafe::provider {
                package const PKG_VALUE: u64 = 77;
            }
            module 0xcafe::consumer {
                use 0xcafe::provider;
                public entry fun check(account: &signer) {
                    assert!(provider::PKG_VALUE == 77, 1);
                }
            }
        "#
    ),]));

    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::consumer::check").unwrap(),
        vec![],
        vec![],
    ));
}

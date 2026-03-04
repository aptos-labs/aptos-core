// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! E2E tests for public constants feature (language version 2.5+).
//!
//! Verifies that `public const` declarations compile and are accessible
//! cross-module, with their values correctly inlined at the call site.

use crate::{assert_success, tests::common, MoveHarness};
use aptos_framework::BuildOptions;
use aptos_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};

/// Mirrors `0xCAFE::consumer::Result` in the test Move package.
#[derive(Serialize, Deserialize, Debug)]
struct Result {
    max_value: u64,
    version: u8,
    enabled: bool,
}

fn build_opts() -> BuildOptions {
    BuildOptions::move_2().set_latest_language()
}

/// Test that a module with public constants compiles, publishes, and can be
/// consumed from another module with the correct values inlined.
#[test]
fn test_public_const_cross_module() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    // Publish the package (both `constants` and `consumer` modules).
    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("public_const.data"),
        build_opts(),
    ));

    // Call consumer::store_constants which inlines MAX_VALUE, VERSION, ENABLED.
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::consumer::store_constants").unwrap(),
        vec![],
        vec![],
    ));

    // Read the stored Result resource and verify the constants were inlined correctly.
    let result: Result = h
        .read_resource(
            acc.address(),
            str::parse("0xcafe::consumer::Result").unwrap(),
        )
        .unwrap();

    assert_eq!(result.max_value, 100, "MAX_VALUE should be 100");
    assert_eq!(result.version, 42, "VERSION should be 42");
    assert!(result.enabled, "ENABLED should be true");
}

// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests for struct definition count limits per module.

use crate::{assert_success, assert_vm_status, MoveHarness};
use aptos_framework::BuildOptions;
use aptos_package_builder::PackageBuilder;
use aptos_types::{
    account_address::AccountAddress, on_chain_config::TimedFeatureFlag,
    transaction::TransactionStatus,
};
use move_core_types::vm_status::StatusCode;

/// Generates Move source for a module with `n` empty structs.
fn struct_defs_source(n: usize) -> String {
    let structs: String = (0..n)
        .map(|i| format!("    struct S{} has copy, drop, store {{}}\n", i))
        .collect();
    format!(
        r#"module 0xbeef::StructDefs {{
{structs}}}"#
    )
}

/// Builds and publishes a package containing `n` struct definitions.
fn publish_struct_package(h: &mut MoveHarness, n: usize) -> TransactionStatus {
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xbeef").unwrap());
    let mut builder = PackageBuilder::new("StructDefs");
    builder.add_source("struct_defs", &struct_defs_source(n));
    let path = builder.write_to_temp().unwrap();
    h.publish_package_with_options(
        &acc,
        path.path(),
        BuildOptions::move_2().set_latest_language(),
    )
}

fn test_for_max(max: usize, config: impl Fn(&mut MoveHarness)) {
    let mut h = MoveHarness::new_testnet();
    config(&mut h);
    assert_success!(publish_struct_package(&mut h, max));
    let mut h = MoveHarness::new_testnet();
    config(&mut h);
    assert_vm_status!(
        publish_struct_package(&mut h, max + 1),
        StatusCode::MAX_STRUCT_DEFINITIONS_REACHED
    );
}

#[test]
fn test_struct_def_bounds() {
    // Before strict bounds: no limit enforced by verifier.
    // Verify that publishing above the strict limit (201) succeeds.
    {
        let mut h = MoveHarness::new_testnet();
        h.set_timed_feature(TimedFeatureFlag::EnableStrictBoundsInProdConfig, false);
        assert_success!(publish_struct_package(&mut h, 201));
    }
    // After strict bounds: limit is 200
    test_for_max(200, |h: &mut MoveHarness| {
        h.set_timed_feature(TimedFeatureFlag::EnableStrictBoundsInProdConfig, true)
    });
    // After revised bounds: limit is 1100
    test_for_max(1100, |h: &mut MoveHarness| {
        h.set_timed_feature(TimedFeatureFlag::RevisedBoundsInProdConfig, true)
    });
}

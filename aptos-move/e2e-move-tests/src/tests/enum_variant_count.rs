// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests for enum variant counts

use crate::{assert_success, assert_vm_status, tests::common, MoveHarness};
use aptos_framework::BuildOptions;
use aptos_package_builder::PackageBuilder;
use aptos_types::{
    account_address::AccountAddress,
    on_chain_config::TimedFeatureFlag,
    transaction::{ExecutionStatus, TransactionStatus},
};
use move_core_types::{value::VARIANT_COUNT_MAX, vm_status::StatusCode};

/// Generates Move source for a module with an enum of `n` variants.
fn enum_variants_source(n: usize) -> String {
    let variants: String = (0..n).map(|i| format!("        V{}(u64),\n", i)).collect();
    format!(
        r#"module 0xbeef::VersionModule {{
    use std::signer;

    enum Versions has copy, drop, store {{
{variants}    }}

    struct VersionHolder has key, store {{
        version: Versions,
    }}

    public entry fun store_version(account: &signer) {{
        let version = Versions::V1(1);
        move_to(account, VersionHolder {{ version }});
    }}

    public entry fun get_version(account: &signer) acquires VersionHolder {{
        let holder = &VersionHolder[signer::address_of(account)];
        assert!(holder.version == Versions::V1(1), 42);
    }}
}}"#
    )
}

/// Builds and publishes a package containing an enum with `n` variants.
fn publish_enum_package(h: &mut MoveHarness, n: usize) -> TransactionStatus {
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xbeef").unwrap());
    let mut builder = PackageBuilder::new("EnumVariants");
    builder.add_source("enum_variants", &enum_variants_source(n));
    builder.add_local_dep(
        "MoveStdlib",
        &common::framework_dir_path("move-stdlib").to_string_lossy(),
    );
    let path = builder.write_to_temp().unwrap();
    let status = h.publish_package_with_options(
        &acc,
        path.path(),
        BuildOptions::move_2().set_latest_language(),
    );

    if status == TransactionStatus::Keep(ExecutionStatus::Success) {
        let txn = h.create_entry_function(
            &acc,
            str::parse("0xbeef::VersionModule::store_version").unwrap(),
            vec![],
            vec![],
        );
        let output = h.run_block_get_output(vec![txn]).pop().unwrap();
        assert_eq!(
            *output.status(),
            TransactionStatus::Keep(ExecutionStatus::Success)
        );

        let txn = h.create_entry_function(
            &acc,
            str::parse("0xbeef::VersionModule::get_version").unwrap(),
            vec![],
            vec![],
        );
        let output = h.run_block_get_output(vec![txn]).pop().unwrap();
        assert_eq!(
            *output.status(),
            TransactionStatus::Keep(ExecutionStatus::Success)
        );
    }

    status
}

fn test_for_max(max: usize, config: impl Fn(&mut MoveHarness)) {
    let mut h = MoveHarness::new_testnet();
    config(&mut h);
    assert_success!(publish_enum_package(&mut h, max));
    let mut h = MoveHarness::new_testnet();
    config(&mut h);
    if max < (VARIANT_COUNT_MAX as usize) {
        // Can only test if hard limit is not reached
        assert_vm_status!(
            publish_enum_package(&mut h, max + 1),
            StatusCode::MAX_STRUCT_VARIANTS_REACHED
        );
    }
}

#[test]
fn test_enum_bounds() {
    test_for_max(VARIANT_COUNT_MAX as usize, |h: &mut MoveHarness| {
        h.set_timed_feature(TimedFeatureFlag::EnableStrictBoundsInProdConfig, false)
    });
    test_for_max(64, |h: &mut MoveHarness| {
        h.set_timed_feature(TimedFeatureFlag::EnableStrictBoundsInProdConfig, true)
    });
    test_for_max(VARIANT_COUNT_MAX as usize, |h: &mut MoveHarness| {
        h.set_timed_feature(TimedFeatureFlag::RevisedBoundsInProdConfig, true)
    });
}

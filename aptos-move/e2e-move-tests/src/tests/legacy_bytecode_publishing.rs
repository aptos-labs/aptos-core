// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests for the legacy module publishing bans:
//!   - `TimedFeatureFlag::RejectV5ModulePublishing` bans v5.
//!   - From gas feature version 1.50, the `txn.min_module_bytecode_version` gas parameter sets
//!     the minimum publishable bytecode version.
//!
//! The honest toolchain no longer emits old versions, so these tests craft legacy bytecode by
//! building the module at the current version and re-serializing it at the target version.

use crate::{assert_success, assert_vm_status, MoveHarness};
use aptos_cached_packages::aptos_stdlib;
use aptos_framework::{BuildOptions, BuiltPackage};
use aptos_gas_schedule::gas_feature_versions::{RELEASE_V1_49, RELEASE_V1_50};
use aptos_package_builder::PackageBuilder;
use aptos_types::{
    account_address::AccountAddress, on_chain_config::TimedFeatureFlag,
    transaction::TransactionStatus,
};
use move_binary_format::file_format_common::{VERSION_5, VERSION_7, VERSION_8, VERSION_MAX};
use move_core_types::vm_status::StatusCode;
use tempfile::TempDir;

const ADDR: &str = "0xa11ce";

// Trivial, dependency-free module so the build is fast.
const SOURCE: &str = r#"
module 0xa11ce::legacy {
    struct Marker has key { v: u64 }

    public entry fun create(s: &signer) {
        move_to(s, Marker { v: 1 });
    }
}
"#;

fn addr() -> AccountAddress {
    AccountAddress::from_hex_literal(ADDR).unwrap()
}

/// The test package built at the latest language and bytecode version.
struct TestPackage {
    package: BuiltPackage,
    /// Serialized package metadata.
    metadata: Vec<u8>,
    /// Keeps the sources on disk: metadata/code extraction reads them back.
    _dir: TempDir,
}

fn build_package() -> TestPackage {
    let mut builder = PackageBuilder::new("Legacy");
    builder.add_source("legacy.move", SOURCE);
    let dir = builder.write_to_temp().unwrap();
    let package = BuiltPackage::build(
        dir.path().to_path_buf(),
        BuildOptions::move_2().set_latest_language(),
    )
    .expect("building package must succeed");
    let metadata = bcs::to_bytes(&package.extract_metadata().unwrap()).unwrap();
    TestPackage {
        package,
        metadata,
        _dir: dir,
    }
}

/// Re-serializes each module at the given bytecode version, stripping the newer file-format
/// fields the current compiler emits (which can't exist at older versions).
fn code_at_version(package: &BuiltPackage, version: u32) -> Vec<Vec<u8>> {
    package
        .modules()
        .map(|module| {
            let mut module = module.clone();
            module.version = version;
            for handle in &mut module.function_handles {
                handle.attributes.clear();
                if version < VERSION_7 {
                    handle.access_specifiers = None;
                }
            }
            let mut bytes = vec![];
            module
                .serialize_for_version(Some(version), &mut bytes)
                .expect("re-serializing module at older version must succeed");
            bytes
        })
        .collect()
}

/// Publishes `code` with the package metadata from a fresh account.
fn publish(h: &mut MoveHarness, metadata: &[u8], code: &[Vec<u8>]) -> TransactionStatus {
    let account = h.new_account_at(addr());
    h.run_transaction_payload(
        &account,
        aptos_stdlib::code_publish_package_txn(metadata.to_vec(), code.to_vec()),
    )
}

/// Publishes the package as built, at the latest bytecode version.
fn publish_latest(h: &mut MoveHarness, package: &BuiltPackage) -> TransactionStatus {
    let account = h.new_account_at(addr());
    let txn = h.create_publish_built_package(&account, package, |_| {});
    h.run(txn)
}

#[test]
fn legacy_v5_module_publishing_is_gated() {
    let pkg = build_package();
    let (package, metadata) = (&pkg.package, &pkg.metadata);
    let code_v5 = code_at_version(package, VERSION_5);

    // The gas schedule gate is pinned to the version before it existed, so only the timed
    // feature is under test here.
    let harness = |timed_feature_enabled: bool| {
        let mut h = MoveHarness::new_testnet();
        h.modify_gas_schedule_raw(|schedule| schedule.feature_version = RELEASE_V1_49);
        h.set_timed_feature(
            TimedFeatureFlag::RejectV5ModulePublishing,
            timed_feature_enabled,
        );
        h
    };

    // Gate off: v5 publishes.
    let mut h = harness(false);
    assert_success!(publish(&mut h, metadata, &code_v5));

    // Gate on: v5 is rejected.
    let mut h = harness(true);
    assert_vm_status!(
        publish(&mut h, metadata, &code_v5),
        StatusCode::CONSTRAINT_NOT_SATISFIED
    );

    // Gate on: a modern (v6+) build still publishes (no false positives).
    let mut h = harness(true);
    assert_success!(publish_latest(&mut h, package));
}

#[test]
fn min_module_bytecode_version_is_gated_by_gas_schedule() {
    let pkg = build_package();
    let (package, metadata) = (&pkg.package, &pkg.metadata);
    let code_v7 = code_at_version(package, VERSION_7);
    let code_v8 = code_at_version(package, VERSION_8);

    // Before 1.50 the gas parameter does not exist, so v7 publishes.
    let mut h = MoveHarness::new_testnet();
    h.modify_gas_schedule_raw(|schedule| schedule.feature_version = RELEASE_V1_49);
    assert_success!(publish(&mut h, metadata, &code_v7));

    // From 1.50 the initial value is VERSION_MAX - 2: v7 is rejected.
    let mut h = MoveHarness::new_testnet();
    let (feature_version, gas_params) = h.get_gas_params();
    assert!(feature_version >= RELEASE_V1_50);
    assert_eq!(
        u64::from(gas_params.vm.txn.min_module_bytecode_version),
        u64::from(VERSION_MAX - 2)
    );
    assert_vm_status!(
        publish(&mut h, metadata, &code_v7),
        StatusCode::CONSTRAINT_NOT_SATISFIED
    );

    // From 1.50: v8 (exactly the minimum) still publishes.
    let mut h = MoveHarness::new_testnet();
    assert_success!(publish(&mut h, metadata, &code_v8));

    // From 1.50: the latest version publishes (no false positives).
    let mut h = MoveHarness::new_testnet();
    assert_success!(publish_latest(&mut h, package));

    // The minimum is read from the on-chain gas schedule: raising it to VERSION_MAX rejects v8
    // but still allows the latest version.
    let mut h = MoveHarness::new_testnet();
    h.modify_gas_schedule(|gas_params| {
        gas_params.vm.txn.min_module_bytecode_version = u64::from(VERSION_MAX).into();
    });
    assert_vm_status!(
        publish(&mut h, metadata, &code_v8),
        StatusCode::CONSTRAINT_NOT_SATISFIED
    );
    assert_success!(publish_latest(&mut h, package));
}

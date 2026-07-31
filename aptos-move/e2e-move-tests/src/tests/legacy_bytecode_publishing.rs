// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests for the v5 module publishing ban (`TimedFeatureFlag::RejectV5ModulePublishing`).
//!
//! The honest toolchain no longer emits v5, so these tests craft v5 bytecode by building the
//! module at a current version and re-serializing it at version 5.

use crate::{assert_success, assert_vm_status, MoveHarness};
use aptos_cached_packages::aptos_stdlib;
use aptos_framework::{BuildOptions, BuiltPackage};
use aptos_package_builder::PackageBuilder;
use aptos_types::{account_address::AccountAddress, on_chain_config::TimedFeatureFlag};
use move_core_types::vm_status::StatusCode;

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

/// Re-serializes each module at bytecode version 5, stripping the v7/v8 file-format fields the
/// current compiler emits (which can't exist at v5).
fn code_at_v5(package: &BuiltPackage) -> Vec<Vec<u8>> {
    package
        .modules()
        .map(|module| {
            let mut module = module.clone();
            module.version = 5;
            for handle in &mut module.function_handles {
                handle.attributes.clear();
                handle.access_specifiers = None;
            }
            let mut bytes = vec![];
            module
                .serialize_for_version(Some(5), &mut bytes)
                .expect("re-serializing module at v5 must succeed");
            bytes
        })
        .collect()
}

#[test]
fn legacy_v5_module_publishing_is_gated() {
    let mut builder = PackageBuilder::new("Legacy");
    builder.add_source("legacy.move", SOURCE);
    // Keep `pkg_dir` alive: metadata/code extraction reads the sources back from disk.
    let pkg_dir = builder.write_to_temp().unwrap();
    let package = BuiltPackage::build(
        pkg_dir.path().to_path_buf(),
        BuildOptions::move_2().set_latest_language(),
    )
    .expect("building package must succeed");
    let metadata = bcs::to_bytes(&package.extract_metadata().unwrap()).unwrap();
    let code_v5 = code_at_v5(&package);

    // Gate off: v5 publishes.
    {
        let mut h = MoveHarness::new_testnet();
        h.set_timed_feature(TimedFeatureFlag::RejectV5ModulePublishing, false);
        let account = h.new_account_at(addr());
        assert_success!(h.run_transaction_payload(
            &account,
            aptos_stdlib::code_publish_package_txn(metadata.clone(), code_v5.clone()),
        ));
    }

    // Gate on: v5 is rejected.
    {
        let mut h = MoveHarness::new_testnet();
        h.set_timed_feature(TimedFeatureFlag::RejectV5ModulePublishing, true);
        let account = h.new_account_at(addr());
        assert_vm_status!(
            h.run_transaction_payload(
                &account,
                aptos_stdlib::code_publish_package_txn(metadata.clone(), code_v5.clone()),
            ),
            StatusCode::CONSTRAINT_NOT_SATISFIED
        );
    }

    // Gate on: a modern (v6+) build still publishes (no false positives).
    {
        let mut h = MoveHarness::new_testnet();
        h.set_timed_feature(TimedFeatureFlag::RejectV5ModulePublishing, true);
        let account = h.new_account_at(addr());
        let txn = h.create_publish_built_package(&account, &package, |_| {});
        assert_success!(h.run(txn));
    }
}

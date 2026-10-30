// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! End-to-end tests for the borrow field handle shape check gated by
//! `FeatureFlag::INVARIANT_VIOLATION_FIXES`.
//!
//! Cross-module field borrows compile to calls of the defining module's borrow field
//! wrappers, so the caller's imported handle carries a `BorrowFieldMutable` attribute.
//! With the flag enabled, a compiled cross-module borrow still publishes, and a handle
//! tampered to a different signature is rejected with `INVALID_STRUCT_API_CODE`.

use crate::{assert_success, assert_vm_status, tests::common, MoveHarness};
use aptos_framework::{BuildOptions, BuiltPackage};
use aptos_package_builder::PackageBuilder;
use aptos_types::{
    account_address::AccountAddress, on_chain_config::FeatureFlag, transaction::TransactionStatus,
};
use move_binary_format::{
    file_format::{FunctionAttribute, SignatureIndex},
    CompiledModule,
};
use move_core_types::vm_status::StatusCode;
use tempfile::TempDir;

const PROVIDER: &str = r#"
module 0xcafe::provider {
    public struct S has copy, drop {
        x: u64,
    }

    public fun make(): S {
        S { x: 1 }
    }
}
"#;

const USER: &str = r#"
module 0xcafe::user {
    use 0xcafe::provider::S;

    public fun bump(s: &mut S) {
        s.x = s.x + 1;
    }
}
"#;

fn options() -> BuildOptions {
    BuildOptions::move_2().set_latest_language()
}

fn package_dir(sources: &[(&str, &str)]) -> TempDir {
    let mut builder = PackageBuilder::new("Package");
    for (name, source) in sources {
        builder.add_source(name, source);
    }
    builder.add_local_dep(
        "MoveStdlib",
        &common::framework_dir_path("move-stdlib").to_string_lossy(),
    );
    builder
        .write_to_temp()
        .expect("package written to temp dir")
}

fn harness(flag_enabled: bool) -> MoveHarness {
    let flags = vec![FeatureFlag::INVARIANT_VIOLATION_FIXES];
    if flag_enabled {
        MoveHarness::new_with_features(flags, vec![])
    } else {
        MoveHarness::new_with_features(vec![], flags)
    }
}

/// Publishes `provider`, then a `user` package whose imported `borrow_mut` handle takes
/// `()` instead of `&mut S`, and returns the status of the second publish.
fn publish_tampered_user(flag_enabled: bool) -> TransactionStatus {
    let mut h = harness(flag_enabled);
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").expect("valid address"));
    let provider = package_dir(&[("provider.move", PROVIDER)]);
    assert_success!(h.publish_package_with_options(&acc, provider.path(), options()));

    // Metadata extraction reads sources from disk, so `dir` must outlive `package`.
    let dir = package_dir(&[("provider.move", PROVIDER), ("user.move", USER)]);
    let package = BuiltPackage::build(dir.path().to_path_buf(), options()).expect("package builds");
    let code = package
        .extract_code()
        .into_iter()
        .map(|bytes| {
            let mut module =
                CompiledModule::deserialize(&bytes).expect("compiled module deserializes");
            if module.self_id().name().as_str() == "user" {
                tamper_borrow_mut_handle(&mut module);
            }
            let mut out = vec![];
            module
                .serialize(&mut out)
                .expect("tampered module serializes");
            out
        })
        .collect();
    let metadata = package.extract_metadata().expect("package metadata");
    let payload = aptos_cached_packages::aptos_stdlib::code_publish_package_txn(
        bcs::to_bytes(&metadata).expect("metadata serializes"),
        code,
    );
    h.without_mono_move(|h| h.run_transaction_payload(&acc, payload))
}

/// Points the imported `borrow_mut` handle at the empty signature already in the pool
/// (used by `bump`'s unit return); the pool must not contain duplicates.
fn tamper_borrow_mut_handle(module: &mut CompiledModule) {
    let empty = module
        .signatures
        .iter()
        .position(|s| s.0.is_empty())
        .expect("unit return signature exists");
    let handle = module
        .function_handles
        .iter_mut()
        .find(|h| {
            h.attributes
                .iter()
                .any(|a| matches!(a, FunctionAttribute::BorrowFieldMutable(_)))
        })
        .expect("cross-module field mutation must import a borrow_mut wrapper");
    handle.parameters = SignatureIndex(empty as u16);
}

#[test]
fn compiled_cross_module_borrow_publishes_with_flag_enabled() {
    let mut h = harness(true);
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").expect("valid address"));
    let provider = package_dir(&[("provider.move", PROVIDER)]);
    assert_success!(h.publish_package_with_options(&acc, provider.path(), options()));
    let both = package_dir(&[("provider.move", PROVIDER), ("user.move", USER)]);
    assert_success!(h.publish_package_with_options(&acc, both.path(), options()));
}

#[test]
fn tampered_borrow_handle_rejected_cleanly_with_flag_enabled() {
    assert_vm_status!(
        publish_tampered_user(true),
        StatusCode::INVALID_STRUCT_API_CODE
    );
}

/// With the flag disabled, the tampered handle is rejected as before, by the stack usage
/// check: the caller still pushes a `&mut S` argument the handle no longer takes.
#[test]
fn tampered_borrow_handle_keeps_previous_outcome_with_flag_disabled() {
    assert_vm_status!(
        publish_tampered_user(false),
        StatusCode::POSITIVE_STACK_SIZE_AT_BLOCK_END
    );
}

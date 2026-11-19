// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests for `FeatureFlag::REJECT_TRAILING_BYTES_IN_INCOMING_CODE`: published modules and executed
//! scripts must not have bytes after the end of their binary, while modules published before
//! the flag was enabled stay loadable.

use crate::{assert_success, assert_vm_status, MoveHarness};
use aptos_cached_packages::aptos_stdlib;
use aptos_framework::{BuildOptions, BuiltPackage};
use aptos_language_e2e_tests::account::Account;
use aptos_package_builder::PackageBuilder;
use aptos_types::{
    account_address::AccountAddress, on_chain_config::FeatureFlag, transaction::TransactionStatus,
};
use move_core_types::vm_status::StatusCode;

const MODULE_SOURCE: &str = r#"
module 0xa11ce::trailing {
    public entry fun run() {}
}
"#;

const SCRIPT_SOURCE: &str = r#"
script {
    fun main() {}
}
"#;

const TRAILING_BYTES: &[u8] = b"payload";

struct TestPackage {
    /// Serialized package metadata.
    metadata: Vec<u8>,
    module_code: Vec<u8>,
    script_code: Vec<u8>,
}

fn build_package() -> TestPackage {
    let mut builder = PackageBuilder::new("Trailing");
    builder.add_source("trailing.move", MODULE_SOURCE);
    builder.add_source("script.move", SCRIPT_SOURCE);
    let dir = builder.write_to_temp().unwrap();
    let package = BuiltPackage::build(
        dir.path().to_path_buf(),
        BuildOptions::move_2().set_latest_language(),
    )
    .expect("building package must succeed");
    let [module_code]: [Vec<u8>; 1] = package
        .extract_code()
        .try_into()
        .expect("package must have exactly one module");
    TestPackage {
        metadata: bcs::to_bytes(&package.extract_metadata().unwrap()).unwrap(),
        module_code,
        script_code: package.extract_script_code().pop().unwrap(),
    }
}

fn with_trailing_bytes(code: &[u8]) -> Vec<u8> {
    [code, TRAILING_BYTES].concat()
}

/// Returns a harness with the flag disabled, and the account at the package address.
fn new_harness() -> (MoveHarness, Account) {
    let mut harness = MoveHarness::new_with_features(vec![], vec![
        FeatureFlag::REJECT_TRAILING_BYTES_IN_INCOMING_CODE,
    ]);
    let account = harness.new_account_at(AccountAddress::from_hex_literal("0xa11ce").unwrap());
    (harness, account)
}

fn enable_flag(harness: &mut MoveHarness) {
    harness.enable_features(
        vec![FeatureFlag::REJECT_TRAILING_BYTES_IN_INCOMING_CODE],
        vec![],
    );
}

fn publish(
    harness: &mut MoveHarness,
    account: &Account,
    metadata: &[u8],
    code: Vec<u8>,
) -> TransactionStatus {
    let payload = aptos_stdlib::code_publish_package_txn(metadata.to_vec(), vec![code]);
    harness.without_mono_move(|harness| harness.run_transaction_payload(account, payload))
}

fn run_entry(harness: &mut MoveHarness, account: &Account) -> TransactionStatus {
    harness.run_entry_function(
        account,
        str::parse("0xa11ce::trailing::run").unwrap(),
        vec![],
        vec![],
    )
}

fn run_script(harness: &mut MoveHarness, account: &Account, code: Vec<u8>) -> TransactionStatus {
    let txn = harness.create_script(account, code, vec![], vec![]);
    harness.run(txn)
}

#[test]
fn module_with_trailing_bytes_is_rejected_at_publishing_but_stays_loadable() {
    let package = build_package();
    let (mut harness, account) = new_harness();
    assert_success!(publish(
        &mut harness,
        &account,
        &package.metadata,
        with_trailing_bytes(&package.module_code)
    ));

    enable_flag(&mut harness);
    assert_success!(run_entry(&mut harness, &account));

    // An upgrade must not have trailing bytes, even if the existing module does.
    assert_vm_status!(
        publish(
            &mut harness,
            &account,
            &package.metadata,
            with_trailing_bytes(&package.module_code)
        ),
        StatusCode::CODE_DESERIALIZATION_ERROR
    );
    assert_success!(publish(
        &mut harness,
        &account,
        &package.metadata,
        package.module_code.clone()
    ));
    assert_success!(run_entry(&mut harness, &account));
}

#[test]
fn script_with_trailing_bytes_is_rejected() {
    let package = build_package();
    let (mut harness, account) = new_harness();
    assert_success!(run_script(
        &mut harness,
        &account,
        with_trailing_bytes(&package.script_code)
    ));

    enable_flag(&mut harness);
    assert_vm_status!(
        run_script(
            &mut harness,
            &account,
            with_trailing_bytes(&package.script_code)
        ),
        StatusCode::CODE_DESERIALIZATION_ERROR
    );
    assert_success!(run_script(
        &mut harness,
        &account,
        package.script_code.clone()
    ));
}

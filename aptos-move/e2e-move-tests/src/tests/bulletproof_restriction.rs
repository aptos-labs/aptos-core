// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, tests::common, MoveHarness};
use aptos_framework::{BuildOptions, BuiltPackage};
use aptos_types::{
    on_chain_config::FeatureFlag,
    transaction::{ExecutionStatus, TransactionOutput, TransactionStatus},
};
use claims::assert_matches;
use move_core_types::{
    account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
    vm_status::AbortLocation,
};
use std::str::FromStr;

#[test]
fn system_entry_function_should_work_when_restriction_is_off() {
    let mut harness = MoveHarness::new();
    harness.enable_features(
        vec![FeatureFlag::UNRESTRICTED_BULLETPROOFS_BATCH_NATIVES],
        vec![],
    );
    let framework = harness.new_account_at(AccountAddress::ONE);
    assert_success!(harness.publish_package_cache_building(
        &framework,
        &common::test_dir_path("bulletproof_restriction.data/system-package")
    ));
    let func_id = str::parse("0x1::module1::func1").unwrap();
    let status = harness.run_entry_function(&framework, func_id, vec![], vec![]);
    assert_success!(status);
}

#[test]
fn non_system_entry_function_should_work_when_restriction_is_off() {
    let mut harness = MoveHarness::new();
    harness.enable_features(
        vec![FeatureFlag::UNRESTRICTED_BULLETPROOFS_BATCH_NATIVES],
        vec![],
    );
    let user_account = harness.new_account_at(AccountAddress::from_str("0xcafe").unwrap());
    assert_success!(harness.publish_package_cache_building(
        &user_account,
        &common::test_dir_path("bulletproof_restriction.data/user-package")
    ));
    let func_id = str::parse("0xcafe::module2::func2").unwrap();
    let status = harness.run_entry_function(&user_account, func_id, vec![], vec![]);
    assert_success!(status);
}

#[test]
fn script_should_work_when_restriction_is_off() {
    let mut harness = MoveHarness::new();
    let output = run_a_bulletproof_script(&mut harness);
    assert_success!(output.status().clone());
}

#[test]
fn system_entry_function_should_work_when_restriction_is_on() {
    let mut harness = MoveHarness::new();
    harness.enable_features(vec![], vec![
        FeatureFlag::UNRESTRICTED_BULLETPROOFS_BATCH_NATIVES,
    ]);
    let framework = harness.new_account_at(AccountAddress::ONE);
    assert_success!(harness.publish_package_cache_building(
        &framework,
        &common::test_dir_path("bulletproof_restriction.data/system-package")
    ));
    let func_id = str::parse("0x1::module1::func1").unwrap();
    let status = harness.run_entry_function(&framework, func_id, vec![], vec![]);
    assert_success!(status);
}

#[test]
fn non_system_entry_function_should_abort_when_restriction_is_on() {
    let mut harness = MoveHarness::new();
    harness.enable_features(vec![], vec![
        FeatureFlag::UNRESTRICTED_BULLETPROOFS_BATCH_NATIVES,
    ]);
    let user_account = harness.new_account_at(AccountAddress::from_str("0xcafe").unwrap());
    assert_success!(harness.publish_package_cache_building(
        &user_account,
        &common::test_dir_path("bulletproof_restriction.data/user-package")
    ));
    let func_id = str::parse("0xcafe::module2::func2").unwrap();
    let status = harness.run_entry_function(&user_account, func_id, vec![], vec![]);
    assert_abort(&status);
}

#[test]
fn script_should_abort_when_restriction_is_on() {
    let mut harness = MoveHarness::new();
    harness.enable_features(vec![], vec![
        FeatureFlag::UNRESTRICTED_BULLETPROOFS_BATCH_NATIVES,
    ]);
    let output = run_a_bulletproof_script(&mut harness);
    assert_abort(output.status());
}

fn run_a_bulletproof_script(harness: &mut MoveHarness) -> TransactionOutput {
    let package = BuiltPackage::build(
        common::test_dir_path("bulletproof_restriction.data/script"),
        BuildOptions::default(),
    )
    .unwrap();
    let script = package.extract_script_code().pop().unwrap();
    let alice = harness.new_account_at(AccountAddress::from_hex_literal("0xa11ce").unwrap());
    let txn = harness.create_script(&alice, script.clone(), vec![], vec![]);
    harness.run_raw(txn)
}

fn assert_abort(status: &TransactionStatus) {
    let module_id = ModuleId::new(
        AccountAddress::ONE,
        Identifier::from_str("ristretto255_bulletproofs").unwrap(),
    );
    assert_matches!(status.as_kept_status().unwrap(), ExecutionStatus::MoveAbort { location, code, .. } if location == AbortLocation::Module(module_id) && code == 0x030008);
}

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, tests::common, MoveHarness};
use aptos_framework::{BuildOptions, BuiltPackage};
use aptos_language_e2e_tests::executor::FakeExecutor;
use aptos_types::{move_utils::MemberId, on_chain_config::FeatureFlag};
use move_core_types::account_address::AccountAddress;
use rstest::rstest;
use std::str::FromStr;

fn initialize(h: &mut MoveHarness) {
    let build_options = BuildOptions::move_2().set_latest_language();
    let path = common::test_dir_path("string_utils.data/pack");

    let framework_account = h.aptos_framework_account();
    let status = h.publish_package_with_options(&framework_account, path.as_path(), build_options);
    assert_success!(status);
}

#[rstest(enabled, disabled,
    case(vec![FeatureFlag::NEW_OPTION_MODULE], vec![]),
    case(vec![], vec![FeatureFlag::NEW_OPTION_MODULE]),
)]
fn test_function_value_formatting_in_modules(
    enabled: Vec<FeatureFlag>,
    disabled: Vec<FeatureFlag>,
) {
    let mut h = MoveHarness::new_with_executor(FakeExecutor::from_head_genesis());
    h.enable_features(enabled, disabled);
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x123").unwrap());
    initialize(&mut h);

    let status = h.run_entry_function(
        &acc,
        MemberId::from_str("0x1::string_utils_test::run_all").unwrap(),
        vec![],
        vec![],
    );
    assert_success!(status);
}

#[test]
fn test_function_value_formatting_in_scripts() {
    let build_options = BuildOptions::move_2().set_latest_language();
    let path = common::test_dir_path("string_utils.data/pack");
    let package = BuiltPackage::build(path.to_owned(), build_options.clone())
        .expect("Building a package must succeed");

    let mut scripts = package.extract_script_code();
    assert_eq!(scripts.len(), 1);
    let script = scripts.pop().expect("Script exists");

    let mut h = MoveHarness::new_with_executor(FakeExecutor::from_head_genesis());
    let framework_account = h.aptos_framework_account();
    let txn = h.create_publish_built_package(&framework_account, &package, |_| {});
    assert_success!(h.run(txn));

    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x123").unwrap());
    let txn = h.create_script(&acc, script, vec![], vec![]);
    assert_success!(h.run(txn));
}

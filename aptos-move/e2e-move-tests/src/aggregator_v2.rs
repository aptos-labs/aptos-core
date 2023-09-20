// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, harness::MoveHarness};
use aptos_language_e2e_tests::{account::Account, executor::FakeExecutor};
use aptos_types::{
    account_address::AccountAddress, on_chain_config::FeatureFlag, transaction::SignedTransaction,
};
use std::path::PathBuf;

pub fn initialize(path: PathBuf) -> (MoveHarness, Account) {
    // Aggregator tests should use parallel execution.
    let executor = FakeExecutor::from_head_genesis().set_parallel();

    let mut harness = MoveHarness::new_with_executor(executor);
    harness.enable_features(vec![FeatureFlag::AGGREGATOR_SNAPSHOTS], vec![]);
    let account = harness.new_account_at(AccountAddress::ONE);
    assert_success!(harness.publish_package(&account, &path));
    (harness, account)
}

pub fn verify_copy_snapshot(harness: &mut MoveHarness, account: &Account) -> SignedTransaction {
    harness.create_entry_function(
        account,
        str::parse("0x1::aggregator_v2_test::verify_copy_snapshot").unwrap(),
        vec![],
        vec![],
    )
}

pub fn verify_copy_string_snapshot(
    harness: &mut MoveHarness,
    account: &Account,
) -> SignedTransaction {
    harness.create_entry_function(
        account,
        str::parse("0x1::aggregator_v2_test::verify_copy_string_snapshot").unwrap(),
        vec![],
        vec![],
    )
}

pub fn verify_string_concat(harness: &mut MoveHarness, account: &Account) -> SignedTransaction {
    harness.create_entry_function(
        account,
        str::parse("0x1::aggregator_v2_test::verify_string_concat").unwrap(),
        vec![],
        vec![],
    )
}

pub fn verify_string_snapshot_concat(
    harness: &mut MoveHarness,
    account: &Account,
) -> SignedTransaction {
    harness.create_entry_function(
        account,
        str::parse("0x1::aggregator_v2_test::verify_string_snapshot_concat").unwrap(),
        vec![],
        vec![],
    )
}

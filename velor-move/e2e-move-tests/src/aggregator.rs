// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, harness::MoveHarness};
use velor_language_e2e_tests::{account::Account, executor::FakeExecutor};
use velor_types::{account_address::AccountAddress, transaction::SignedTransaction};
use std::path::PathBuf;

pub fn initialize(path: PathBuf) -> (MoveHarness, Account) {
    // Aggregator tests should use parallel execution.
    let executor = FakeExecutor::from_head_genesis();

    let mut harness = MoveHarness::new_with_executor(executor);
    let account = harness.new_account_at(AccountAddress::ONE);
    assert_success!(harness.publish_package_cache_building(&account, &path));
    assert_success!(harness.run_entry_function(
        &account,
        str::parse("0x1::aggregator_test::initialize").unwrap(),
        vec![],
        vec![],
    ));
    (harness, account)
}

pub fn check(
    harness: &mut MoveHarness,
    account: &Account,
    index: u64,
    expected: u128,
) -> SignedTransaction {
    harness.create_entry_function(
        account,
        str::parse("0x1::aggregator_test::check").unwrap(),
        vec![],
        vec![
            bcs::to_bytes(&index).unwrap(),
            bcs::to_bytes(&expected).unwrap(),
        ],
    )
}

pub fn new(harness: &mut MoveHarness, account: &Account, index: u64) -> SignedTransaction {
    harness.create_entry_function(
        account,
        str::parse("0x1::aggregator_test::new").unwrap(),
        vec![],
        vec![
            bcs::to_bytes(&index).unwrap(),
            bcs::to_bytes(&u128::MAX).unwrap(),
        ],
    )
}

pub fn add(
    harness: &mut MoveHarness,
    account: &Account,
    index: u64,
    value: u128,
) -> SignedTransaction {
    harness.create_entry_function(
        account,
        str::parse("0x1::aggregator_test::add").unwrap(),
        vec![],
        vec![
            bcs::to_bytes(&index).unwrap(),
            bcs::to_bytes(&value).unwrap(),
        ],
    )
}

pub fn sub(
    harness: &mut MoveHarness,
    account: &Account,
    index: u64,
    value: u128,
) -> SignedTransaction {
    harness.create_entry_function(
        account,
        str::parse("0x1::aggregator_test::sub").unwrap(),
        vec![],
        vec![
            bcs::to_bytes(&index).unwrap(),
            bcs::to_bytes(&value).unwrap(),
        ],
    )
}

pub fn sub_add(
    harness: &mut MoveHarness,
    account: &Account,
    index: u64,
    a: u128,
    b: u128,
) -> SignedTransaction {
    harness.create_entry_function(
        account,
        str::parse("0x1::aggregator_test::sub_add").unwrap(),
        vec![],
        vec![
            bcs::to_bytes(&index).unwrap(),
            bcs::to_bytes(&a).unwrap(),
            bcs::to_bytes(&b).unwrap(),
        ],
    )
}

pub fn destroy(harness: &mut MoveHarness, account: &Account, index: u64) -> SignedTransaction {
    harness.create_entry_function(
        account,
        str::parse("0x1::aggregator_test::destroy").unwrap(),
        vec![],
        vec![bcs::to_bytes(&index).unwrap()],
    )
}

pub fn materialize(harness: &mut MoveHarness, account: &Account, index: u64) -> SignedTransaction {
    harness.create_entry_function(
        account,
        str::parse("0x1::aggregator_test::materialize").unwrap(),
        vec![],
        vec![bcs::to_bytes(&index).unwrap()],
    )
}

pub fn materialize_and_add(
    harness: &mut MoveHarness,
    account: &Account,
    index: u64,
    value: u128,
) -> SignedTransaction {
    harness.create_entry_function(
        account,
        str::parse("0x1::aggregator_test::materialize_and_add").unwrap(),
        vec![],
        vec![
            bcs::to_bytes(&index).unwrap(),
            bcs::to_bytes(&value).unwrap(),
        ],
    )
}

pub fn materialize_and_sub(
    harness: &mut MoveHarness,
    account: &Account,
    index: u64,
    value: u128,
) -> SignedTransaction {
    harness.create_entry_function(
        account,
        str::parse("0x1::aggregator_test::materialize_and_sub").unwrap(),
        vec![],
        vec![
            bcs::to_bytes(&index).unwrap(),
            bcs::to_bytes(&value).unwrap(),
        ],
    )
}

pub fn add_and_materialize(
    harness: &mut MoveHarness,
    account: &Account,
    index: u64,
    value: u128,
) -> SignedTransaction {
    harness.create_entry_function(
        account,
        str::parse("0x1::aggregator_test::add_and_materialize").unwrap(),
        vec![],
        vec![
            bcs::to_bytes(&index).unwrap(),
            bcs::to_bytes(&value).unwrap(),
        ],
    )
}

pub fn sub_and_materialize(
    harness: &mut MoveHarness,
    account: &Account,
    index: u64,
    value: u128,
) -> SignedTransaction {
    harness.create_entry_function(
        account,
        str::parse("0x1::aggregator_test::sub_and_materialize").unwrap(),
        vec![],
        vec![
            bcs::to_bytes(&index).unwrap(),
            bcs::to_bytes(&value).unwrap(),
        ],
    )
}

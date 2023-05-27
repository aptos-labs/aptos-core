// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, harness::MoveHarness};
use aptos_language_e2e_tests::account::Account;
use aptos_types::{account_address::AccountAddress, transaction::SignedTransaction};
use std::path::PathBuf;

pub fn initialize(path: PathBuf) -> (MoveHarness, Account) {
    let mut harness = MoveHarness::new();
    let account = harness.new_account_at(AccountAddress::ONE);
    assert_success!(harness.publish_package(&account, &path));
    (harness, account)
}

pub fn create_many_guids(
    harness: &mut MoveHarness,
    account: &Account,
    count: u64,
) -> SignedTransaction {
    harness.create_entry_function(
        account,
        str::parse("0x1::transaction_context_test::create_many_guids").unwrap(),
        vec![],
        vec![bcs::to_bytes(&count).unwrap()],
    )
}

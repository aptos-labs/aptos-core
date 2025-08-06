// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_cached_packages::aptos_stdlib;
use aptos_language_e2e_tests::{account::Account, executor::FakeExecutor};
use aptos_types::transaction::{ExecutionStatus, TransactionStatus};
use rstest::rstest;

#[rstest(stateless_account, case(true), case(false))]
fn mint_to_new_account(stateless_account: bool) {
    let mut executor = FakeExecutor::from_head_genesis();
    let mut root = Account::new_aptos_root();
    let (private_key, public_key) = aptos_vm_genesis::GENESIS_KEYPAIR.clone();
    root.rotate_key(private_key, public_key);

    // Create and publish a sender with TXN_RESERVED coins, also note how
    // many were there before.
    let new_account =
        executor.create_raw_account_data(0, if stateless_account { None } else { Some(0) });
    executor.add_account_data(&new_account);
    let supply_before = executor.read_coin_supply().unwrap();

    let mint_amount = 1_000_000;
    let txn = root
        .transaction()
        .payload(aptos_stdlib::aptos_coin_mint(
            *new_account.address(),
            mint_amount,
        ))
        .gas_unit_price(0)
        .sequence_number(0)
        .sign();
    let output = executor.execute_transaction(txn);

    // Check that supply changed.
    executor.apply_write_set(output.write_set());
    let supply_after = executor.read_coin_supply().unwrap();
    assert_eq!(supply_after, supply_before + (mint_amount as u128));

    assert_eq!(
        output.status(),
        &TransactionStatus::Keep(ExecutionStatus::Success),
    );
}

// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

// Note[Orderless]: Done
use aptos_cached_packages::aptos_stdlib;
use aptos_language_e2e_tests::{
    account::Account, executor::FakeExecutor, feature_flags_for_orderless,
};
use aptos_types::transaction::{ExecutionStatus, TransactionStatus};
use rstest::rstest;

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn mint_to_new_account(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut executor = FakeExecutor::from_head_genesis();
    executor.enable_features(
        feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
        vec![],
    );
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
        .upgrade_payload(use_txn_payload_v2_format, use_orderless_transactions)
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

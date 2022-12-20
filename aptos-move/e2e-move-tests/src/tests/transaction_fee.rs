// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, get_stake_pool, setup_staking, transaction_fee, MoveHarness};
use aptos_cached_packages::aptos_stdlib;
use aptos_types::{
    account_address::AccountAddress, on_chain_config::FeatureFlag, transaction::SignedTransaction,
};

/// Helper which creates two dummy accounts and transactions that send transfers between these accounts.
fn p2p_txns_for_test(harness: &mut MoveHarness, num_txn_pairs: usize) -> Vec<SignedTransaction> {
    let alice = harness.new_account_at(AccountAddress::from_hex_literal("0xa11ce").unwrap());
    let bob = harness.new_account_at(AccountAddress::from_hex_literal("0xb0b").unwrap());

    let mut result = vec![];
    (0..num_txn_pairs).for_each(|_| {
        // Make transfers both ways to ensure that balances stay the same.
        result.push(harness.create_transaction_payload(
            &alice,
            aptos_stdlib::aptos_coin_transfer(*bob.address(), 10),
        ));
        result.push(harness.create_transaction_payload(
            &bob,
            aptos_stdlib::aptos_coin_transfer(*alice.address(), 10),
        ));
    });
    result
}

fn test_fee_collection_and_distribution(burn_percentage: u8) {
    // Initialize fee collection and distribution.
    let mut harness = MoveHarness::new();
    transaction_fee::initialize_fee_collection_and_distribution(&mut harness, burn_percentage);
    harness.enable_features(vec![FeatureFlag::COLLECT_AND_DISTRIBUTE_GAS_FEES], vec![]);

    // Initialize a validator.
    let rewards_per_epoch = 285;
    let mut stake_amount = 25_000_000;
    let validator = harness.new_account_at(AccountAddress::from_hex_literal("0x123").unwrap());
    assert_success!(setup_staking(&mut harness, &validator, stake_amount));
    harness.new_epoch();

    // Run a block of p2p transactions and collect fees.
    let txns = p2p_txns_for_test(&mut harness, 1000);
    let gas_used = harness.run_block_with_metadata(*validator.address(), vec![], txns);
    harness.new_epoch();

    // Make sure calculations match.
    let burnt_amount = (burn_percentage as u64) * gas_used / 100;
    let collected_amount = gas_used - burnt_amount;
    stake_amount += rewards_per_epoch + collected_amount;
    assert_eq!(
        get_stake_pool(&harness, validator.address()).active,
        stake_amount
    );
}

#[test]
fn test_fee_collection_and_distribution_for_burn_percentages() {
    // Test multiple burn percentages including the corner cases of 0 and 100.
    for burn_percentage in [100, 75, 25, 0] {
        test_fee_collection_and_distribution(burn_percentage)
    }
}

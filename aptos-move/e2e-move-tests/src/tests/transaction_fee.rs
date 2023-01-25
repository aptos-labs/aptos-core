// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    assert_success, get_stake_pool, setup_staking, tests::common, transaction_fee, MoveHarness,
};
use aptos_cached_packages::aptos_stdlib;
use aptos_language_e2e_tests::account::Account;
use aptos_types::{
    account_address::AccountAddress,
    on_chain_config::FeatureFlag,
    transaction::{SignedTransaction, TransactionArgument},
};
use once_cell::sync::Lazy;

/// Estimates the cost of simple p2p transactions in order to calculate the gas prices
/// of proposals.
static P2P_TXN_GAS_COST: Lazy<u64> = Lazy::new(|| {
    let mut harness = MoveHarness::new();
    let p1 = harness.new_account_at(AccountAddress::from_hex_literal("0x123").unwrap());
    let p2 = harness.new_account_at(AccountAddress::from_hex_literal("0x456").unwrap());
    harness.evaluate_gas(&p1, aptos_stdlib::aptos_coin_transfer(*p2.address(), 10))
});

/// Creates transactions that send transfers between 2 accounts.
fn p2p_txns_for_test(
    harness: &mut MoveHarness,
    peer1: &Account,
    peer2: &Account,
    num_txn_pairs: usize,
) -> Vec<SignedTransaction> {
    let mut result = vec![];
    (0..num_txn_pairs).for_each(|_| {
        // For these tests we do not care about what these transactions do, as
        // long as they are charged gas. Therefore, make transfers both ways to
        // ensure that balances stay the same.
        result.push(harness.create_transaction_payload(
            peer1,
            aptos_stdlib::aptos_coin_transfer(*peer2.address(), 10),
        ));
        result.push(harness.create_transaction_payload(
            peer2,
            aptos_stdlib::aptos_coin_transfer(*peer1.address(), 10),
        ));
    });
    result
}

/// Compiles the script at the given package and returns it as a signed
/// transaction.
fn create_script(
    harness: &mut MoveHarness,
    package_name: &str,
    sender: &Account,
    args: Vec<TransactionArgument>,
) -> SignedTransaction {
    // Each script has to live in their own package, and there is no real
    // workaround here but to compile all packages separately.
    let package = aptos_framework::BuiltPackage::build(
        common::test_dir_path(format!("transaction_fee.data/{}", package_name).as_str()),
        aptos_framework::BuildOptions::default(),
    )
    .expect("building packages with scripts must succeed");
    let script = package.extract_script_code()[0].clone();
    harness.create_script(sender, script, vec![], args)
}

/// Tests a standard flow of collecting fees without any edge cases.
fn test_fee_collection_and_distribution_flow(burn_percentage: u8) {
    let mut harness = MoveHarness::new();
    transaction_fee::initialize_fee_collection_and_distribution(&mut harness, burn_percentage);
    harness.enable_features(vec![FeatureFlag::COLLECT_AND_DISTRIBUTE_GAS_FEES], vec![]);

    let alice = harness.new_account_at(AccountAddress::from_hex_literal("0xa11ce").unwrap());
    let bob = harness.new_account_at(AccountAddress::from_hex_literal("0xb0b").unwrap());

    // Initialize a validator.
    let rewards_per_epoch = 285;
    let mut stake_amount = 25_000_000;
    let validator = harness.new_account_at(AccountAddress::from_hex_literal("0x123").unwrap());
    assert_success!(setup_staking(&mut harness, &validator, stake_amount));
    harness.new_epoch();

    let mut total_supply = harness.read_total_supply();

    // Run a single block and record how much gas it costs. Since fee collection
    // is enabled, this amount is stored in aggregatable coin.
    let txns = p2p_txns_for_test(&mut harness, &alice, &bob, 1000);
    let gas_used = harness.run_block_with_metadata(*validator.address(), vec![], txns);
    let burnt_amount = (burn_percentage as u64) * gas_used / 100;
    let collected_amount = gas_used - burnt_amount;

    // Make sure we call `new_block_with_metadata` to drain aggregatable coin.
    // Simply calling `new_epoch` will not work because we fast-forward without
    // draining!
    harness.new_block_with_metadata(*validator.address(), vec![]);
    total_supply -= burnt_amount as u128;

    // Check that the right fraction was burnt.
    assert_eq!(harness.read_total_supply(), total_supply);

    // On the new epoch, the collected fees are processed and added to the stake
    // pool.
    harness.new_epoch();
    stake_amount += rewards_per_epoch + collected_amount;
    assert_eq!(
        get_stake_pool(&harness, validator.address()).active,
        stake_amount
    );
}

/// Tests if fees collection can be enabled by the governance proposal and how
/// fees are collected on the block boundary.
fn test_initialize_and_enable_fee_collection_and_distribution(burn_percentage: u8) {
    let mut harness = MoveHarness::new();
    let alice = harness.new_account_at(AccountAddress::from_hex_literal("0xa11ce").unwrap());
    let bob = harness.new_account_at(AccountAddress::from_hex_literal("0xb0b").unwrap());
    let core_resources =
        harness.new_account_at(AccountAddress::from_hex_literal("0xA550C18").unwrap());

    // Initialize a validator.
    let rewards_per_epoch = 285;
    let mut stake_amount = 25_000_000;
    let validator = harness.new_account_at(AccountAddress::from_hex_literal("0x123").unwrap());
    assert_success!(setup_staking(&mut harness, &validator, stake_amount));
    harness.new_epoch();

    // Create transactions to initialize resources for gas fees collection &
    // distribution and enable it.
    let init_script = create_script(
        &mut harness,
        "initialize_collection",
        &core_resources,
        vec![TransactionArgument::U8(burn_percentage)],
    );
    let enable_script = create_script(&mut harness, "enable_collection", &core_resources, vec![]);

    // Create a block of transactions such that:
    //   1. First 10 transactions are p2p.
    //   2. A single transaction placing resources on chain.
    //   3. Another 10 transactions are p2p.
    //   4. A single transaction enabling fees collection.
    //   5. Remaining transactions are p2p (should end up being Retry).
    let mut txns = p2p_txns_for_test(&mut harness, &alice, &bob, 20);
    txns.insert(10, init_script);
    txns.insert(21, enable_script);

    // Simulate block execution.
    let mut total_supply = harness.read_total_supply();
    let gas_used = harness.run_block_with_metadata(*validator.address(), vec![], txns);

    // Reconfiguration triggers distributing rewards.
    stake_amount += rewards_per_epoch;
    total_supply += rewards_per_epoch as u128;
    assert_eq!(
        get_stake_pool(&harness, validator.address()).active,
        stake_amount
    );

    // In the previous block, the fee was only collected for the last script
    // transaction which enabled the feature. In this block, we drain
    // aggregatable coin and try to assign the fee to the validator. Since the
    // proposer is not set (when feature flag was enabled), the fee is simply
    // burnt and the stake pool should have the same value.
    harness.new_block_with_metadata(*validator.address(), vec![]);
    total_supply -= gas_used as u128;
    assert_eq!(harness.read_total_supply(), total_supply);
    assert_eq!(
        get_stake_pool(&harness, validator.address()).active,
        stake_amount
    );
}

/// Tests fee collection can be safely disabled. The corner case here is that by disabling
/// the flag, we cannot distribute fees anymore unless it is done beforehand.
fn test_disable_fee_collection(burn_percentage: u8) {
    let mut harness = MoveHarness::new();
    transaction_fee::initialize_fee_collection_and_distribution(&mut harness, burn_percentage);
    harness.enable_features(vec![FeatureFlag::COLLECT_AND_DISTRIBUTE_GAS_FEES], vec![]);

    let alice = harness.new_account_at(AccountAddress::from_hex_literal("0xa11ce").unwrap());
    let bob = harness.new_account_at(AccountAddress::from_hex_literal("0xb0b").unwrap());
    let core_resources =
        harness.new_account_at(AccountAddress::from_hex_literal("0xA550C18").unwrap());

    // Initialize a validator.
    let rewards_per_epoch = 285;
    let mut stake_amount = 25_000_000;
    let validator = harness.new_account_at(AccountAddress::from_hex_literal("0x123").unwrap());
    assert_success!(setup_staking(&mut harness, &validator, stake_amount));
    harness.new_epoch();

    let disable_script = create_script(&mut harness, "disable_collection", &core_resources, vec![]);

    // Create a block of transactions such that:
    //   1. First 10 transactions are p2p.
    //   2. A single transaction disabling fees collection.
    //   3. Remaining transactions are p2p (should end up being Retry).
    let proposal_idx = 10;
    let mut txns = p2p_txns_for_test(&mut harness, &alice, &bob, 10);
    txns.insert(proposal_idx, disable_script);

    // Simulate block execution.
    let mut total_supply = harness.read_total_supply();
    let gas_used = harness.run_block_with_metadata(*validator.address(), vec![], txns);
    let gas_used_for_p2p = *P2P_TXN_GAS_COST * proposal_idx as u64;
    let gas_used_for_proposal = gas_used - gas_used_for_p2p;

    // Calculate the fees taht are supposed to be collected before the feature
    // is disabled.
    let burnt_amount = (burn_percentage as u64) * gas_used_for_p2p / 100;
    let collected_amount = gas_used_for_p2p - burnt_amount;

    // Reconfiguration triggers distribution of both rewards and fees.
    stake_amount += rewards_per_epoch + collected_amount;
    total_supply += rewards_per_epoch as u128;
    total_supply -= burnt_amount as u128;
    assert_eq!(
        get_stake_pool(&harness, validator.address()).active,
        stake_amount
    );

    // Gas for the proposal should be burnt together with the fraction of the
    // fees.
    harness.new_block_with_metadata(*validator.address(), vec![]);
    total_supply -= gas_used_for_proposal as u128;
    assert_eq!(harness.read_total_supply(), total_supply);
}

/// Tests that the fees collected prior to the upgrade use the right burn
/// percentage for calculations.
fn test_upgrade_burn_percentage(burn_percentage: u8) {
    let mut harness = MoveHarness::new();
    transaction_fee::initialize_fee_collection_and_distribution(&mut harness, burn_percentage);
    harness.enable_features(vec![FeatureFlag::COLLECT_AND_DISTRIBUTE_GAS_FEES], vec![]);

    let alice = harness.new_account_at(AccountAddress::from_hex_literal("0xa11ce").unwrap());
    let bob = harness.new_account_at(AccountAddress::from_hex_literal("0xb0b").unwrap());
    let core_resources =
        harness.new_account_at(AccountAddress::from_hex_literal("0xA550C18").unwrap());

    // Initialize a validator.
    let rewards_per_epoch = 285;
    let mut stake_amount = 25_000_000;
    let validator = harness.new_account_at(AccountAddress::from_hex_literal("0x123").unwrap());
    assert_success!(setup_staking(&mut harness, &validator, stake_amount));
    harness.new_epoch();

    // Upgrade to the opposite value.
    let new_burn_percentage = 100 - burn_percentage;
    let upgrade_script = create_script(
        &mut harness,
        "upgrade_burn_percentage",
        &core_resources,
        vec![TransactionArgument::U8(new_burn_percentage)],
    );

    // Create a block of transactions such that:
    //   1. First 10 transactions are p2p.
    //   2. A single transaction upgrading the burn percentage.
    //   3. Remaining transactions are p2p (should end up being Retry).
    let proposal_idx = 10;
    let mut txns = p2p_txns_for_test(&mut harness, &alice, &bob, 10);
    txns.insert(proposal_idx, upgrade_script);

    // Simulate block execution.
    let mut total_supply = harness.read_total_supply();
    let gas_used = harness.run_block_with_metadata(*validator.address(), vec![], txns);
    let gas_used_for_p2p = *P2P_TXN_GAS_COST * proposal_idx as u64;
    let gas_used_for_proposal = gas_used - gas_used_for_p2p;

    let burnt_amount = (burn_percentage as u64) * gas_used_for_p2p / 100;
    let collected_amount = gas_used_for_p2p - burnt_amount;

    // Reconfiguration triggers distribution of rewards and fees.
    stake_amount += rewards_per_epoch + collected_amount;
    total_supply += rewards_per_epoch as u128;
    total_supply -= burnt_amount as u128;
    assert_eq!(
        get_stake_pool(&harness, validator.address()).active,
        stake_amount
    );

    // Gas for the proposal should be burnt together with fraction of fees.
    harness.new_block_with_metadata(*validator.address(), vec![]);
    total_supply -= gas_used_for_proposal as u128;
    assert_eq!(harness.read_total_supply(), total_supply);

    // Now check that the new burn percentage works correctly. Make sure to
    // reread the total supply because account creation mints coins.
    let carol = harness.new_account_at(AccountAddress::from_hex_literal("0xc0101").unwrap());
    let david = harness.new_account_at(AccountAddress::from_hex_literal("0xda51d").unwrap());
    total_supply = harness.read_total_supply();

    let txns = p2p_txns_for_test(&mut harness, &carol, &david, 20);
    let gas_used = harness.run_block_with_metadata(*validator.address(), vec![], txns);

    let burnt_amount = (new_burn_percentage as u64) * gas_used / 100;
    let collected_amount = gas_used - burnt_amount;

    // Check that the new fraction of fees is burnt.
    harness.new_block_with_metadata(*validator.address(), vec![]);
    total_supply -= burnt_amount as u128;
    assert_eq!(harness.read_total_supply(), total_supply);

    // Check fees are distributed during the next epoch.
    harness.new_epoch();
    stake_amount += rewards_per_epoch + collected_amount;
    assert_eq!(
        get_stake_pool(&harness, validator.address()).active,
        stake_amount
    );
}

/// Tests that if validator running the block is removed, it still receives
/// previously collected fees.
fn test_leaving_validator_is_rewarded(burn_percentage: u8) {
    let mut harness = MoveHarness::new();
    transaction_fee::initialize_fee_collection_and_distribution(&mut harness, burn_percentage);
    harness.enable_features(vec![FeatureFlag::COLLECT_AND_DISTRIBUTE_GAS_FEES], vec![]);

    let alice = harness.new_account_at(AccountAddress::from_hex_literal("0xa11ce").unwrap());
    let bob = harness.new_account_at(AccountAddress::from_hex_literal("0xb0b").unwrap());
    let core_resources =
        harness.new_account_at(AccountAddress::from_hex_literal("0xA550C18").unwrap());

    let rewards_per_epoch = 285;
    let mut stake_amount = 25_000_000;
    let validator = harness.new_account_at(AccountAddress::from_hex_literal("0x123").unwrap());
    let other_validator =
        harness.new_account_at(AccountAddress::from_hex_literal("0x234").unwrap());
    assert_success!(setup_staking(&mut harness, &validator, stake_amount));
    assert_success!(setup_staking(&mut harness, &other_validator, stake_amount));
    harness.new_epoch();

    // This script will simulate a proposal which removes validator from the
    // set.
    let remove_script = create_script(&mut harness, "remove_validator", &core_resources, vec![
        TransactionArgument::Address(*validator.address()),
    ]);

    // Create a block of transactions such that:
    //   1. First 10 transactions are p2p.
    //   2. A single transaction removing the validator.
    //   3. Remaining transactions are p2p (should end up being Retry).
    let proposal_idx = 10;
    let mut txns = p2p_txns_for_test(&mut harness, &alice, &bob, 10);
    txns.insert(proposal_idx, remove_script);

    // Simulate block execution and calculate how much gas was used for
    // transactions and for the governance proposal.
    let mut total_supply = harness.read_total_supply();
    let gas_used = harness.run_block_with_metadata(*validator.address(), vec![], txns);
    let gas_used_for_p2p = *P2P_TXN_GAS_COST * proposal_idx as u64;
    let gas_used_for_proposal = gas_used - gas_used_for_p2p;

    let burnt_amount = (burn_percentage as u64) * gas_used_for_p2p / 100;
    let collected_amount = gas_used_for_p2p - burnt_amount;

    // Reconfiguration triggers distributing rewards and fees.
    stake_amount += rewards_per_epoch + collected_amount;
    total_supply += rewards_per_epoch as u128;
    total_supply -= burnt_amount as u128;
    assert_eq!(
        get_stake_pool(&harness, validator.address()).active,
        stake_amount
    );

    harness.new_block_with_metadata(*other_validator.address(), vec![]);
    total_supply -= gas_used_for_proposal as u128;
    assert_eq!(harness.read_total_supply(), total_supply);
}

#[test]
fn test_fee_collection_and_distribution_for_burn_percentages() {
    // Test multiple burn percentages including the cases of 0 and 100.
    for burn_percentage in [0, 25, 75, 100] {
        test_fee_collection_and_distribution_flow(burn_percentage);
        test_initialize_and_enable_fee_collection_and_distribution(burn_percentage);
        test_disable_fee_collection(burn_percentage);
        test_upgrade_burn_percentage(burn_percentage);
        test_leaving_validator_is_rewarded(burn_percentage);
    }
}

#[test]
/// Tests that fees for proposals are never leaked to the next block and are
/// always burnt.
fn test_block_single_proposal() {
    let mut harness = MoveHarness::new();
    transaction_fee::initialize_fee_collection_and_distribution(&mut harness, 100);
    let core_resources =
        harness.new_account_at(AccountAddress::from_hex_literal("0xA550C18").unwrap());

    let rewards_per_epoch = 285;
    let stake_amount = 25_000_000;
    let validator = harness.new_account_at(AccountAddress::from_hex_literal("0x123").unwrap());
    assert_success!(setup_staking(&mut harness, &validator, stake_amount));
    harness.new_epoch();

    // Create block with a single transaction: governance proposal to enable
    // fee collection. This proposal ends the epoch.
    let mut total_supply = harness.executor.read_coin_supply().unwrap();
    let txn = create_script(&mut harness, "enable_collection", &core_resources, vec![]);
    let gas_used = harness.run_block_with_metadata(*validator.address(), vec![], vec![txn]);

    // After reconfiguration rewards will be distributed. Because there are no
    // other transactions, there is nothing to drain. However, this still should
    // unset the proposer so that the next block burns the proposal fee.
    total_supply += rewards_per_epoch;
    assert_eq!(harness.read_total_supply(), total_supply);

    // Ensure the fees are not leaked to the next block. This block must burn
    // the proposal fee.
    harness.new_block_with_metadata(*validator.address(), vec![]);
    total_supply -= gas_used as u128;
    assert_eq!(harness.read_total_supply(), total_supply);
}

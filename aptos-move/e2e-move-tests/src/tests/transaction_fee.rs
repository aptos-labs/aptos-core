// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    assert_success, get_stake_pool, setup_staking, tests::common, transaction_fee, MoveHarness,
};
use aptos_cached_packages::aptos_stdlib;
use aptos_language_e2e_tests::{account::Account, executor};
use aptos_types::{
    account_address::AccountAddress,
    on_chain_config::FeatureFlag,
    transaction::{SignedTransaction, TransactionArgument},
};
use once_cell::sync::Lazy;

static P2P_TXN_GAS_COST: Lazy<u64> = Lazy::new(|| {
    let mut harness = MoveHarness::new();
    let alice = harness.new_account_at(AccountAddress::from_hex_literal("0xa11ce").unwrap());
    let bob = harness.new_account_at(AccountAddress::from_hex_literal("0xb0b").unwrap());
    harness.evaluate_gas(&alice, aptos_stdlib::aptos_coin_transfer(*bob.address(), 10))
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

fn test_fee_collection_and_distribution_flow(burn_percentage: u8) {
    let mut harness = MoveHarness::new();
    let alice = harness.new_account_at(AccountAddress::from_hex_literal("0xa11ce").unwrap());
    let bob = harness.new_account_at(AccountAddress::from_hex_literal("0xb0b").unwrap());

    // Initialize fee collection and distribution.
    transaction_fee::initialize_fee_collection_and_distribution(&mut harness, burn_percentage);
    harness.enable_features(vec![FeatureFlag::COLLECT_AND_DISTRIBUTE_GAS_FEES], vec![]);

    // Initialize a validator.
    let rewards_per_epoch = 285;
    let mut stake_amount = 25_000_000;
    let validator = harness.new_account_at(AccountAddress::from_hex_literal("0x123").unwrap());
    assert_success!(setup_staking(&mut harness, &validator, stake_amount));
    harness.new_epoch();

    let supply_before = harness.executor.read_coin_supply().unwrap();

    // Run a single block and record how much gas it costs. Since fee collection
    // is enabled, this amount is stored in aggregatable coin.
    let txns = p2p_txns_for_test(&mut harness, &alice, &bob, 1000);
    let gas_used = harness.run_block_with_metadata(*validator.address(), vec![], txns);

    // Make sure we call `new_block_with_metadata` to drain aggregatable coin.
    // Simply calling `new_epoch` will not work because we fast-forward without
    // draining!
    harness.new_block_with_metadata(*validator.address(), vec![]);

    // Check that the right fraction was burnt.
    let supply_after = harness.executor.read_coin_supply().unwrap();
    let burnt_amount = (burn_percentage as u64) * gas_used / 100;
    assert_eq!(
        supply_after.abs_diff(supply_before - burnt_amount as u128),
        0
    );

    // On the new epoch, the collected fees are processed and added to the stake
    // pool.
    harness.new_epoch();
    let collected_amount = gas_used - burnt_amount;
    stake_amount += rewards_per_epoch + collected_amount;
    assert_eq!(
        get_stake_pool(&harness, validator.address()).active,
        stake_amount
    );
}

fn test_initialize_and_enable_fee_collection_and_distribution(burn_percentage: u8) {
    let mut harness = MoveHarness::new();
    let alice = harness.new_account_at(AccountAddress::from_hex_literal("0xa11ce").unwrap());
    let bob = harness.new_account_at(AccountAddress::from_hex_literal("0xb0b").unwrap());

    // Create a core resources account so that we can send imitations of
    // accepted proposal scripts.
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
    let supply_before = harness.executor.read_coin_supply().unwrap();
    let gas_used = harness.run_block_with_metadata(*validator.address(), vec![], txns);

    // Reconfiguration triggers distributing rewards.
    stake_amount += rewards_per_epoch;
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
    let supply_after = harness.executor.read_coin_supply().unwrap();
    assert_eq!(
        supply_after.abs_diff(supply_before - gas_used as u128 + rewards_per_epoch as u128),
        0
    );
    assert_eq!(
        get_stake_pool(&harness, validator.address()).active,
        stake_amount
    );

    // While fees collection is enabled...
    let supply_before = harness.executor.read_coin_supply().unwrap();
    let txns = p2p_txns_for_test(&mut harness, &alice, &bob, 100);
    let gas_used = harness.run_block_with_metadata(*validator.address(), vec![], txns);

    // Run an empty block to drain the aggregatable coin. Fees are assigned to
    // validators and therefore the total supply changes only becoase some
    // percentage was burnt.
    harness.new_block_with_metadata(*validator.address(), vec![]);
    let supply_after = harness.executor.read_coin_supply().unwrap();
    let burnt_amount = (burn_percentage as u64) * gas_used / 100;
    assert_eq!(
        supply_after.abs_diff(supply_before - burnt_amount as u128),
        0
    );
}

fn test_disable_fee_collection(burn_percentage: u8) {
    let mut harness = MoveHarness::new();
    let alice = harness.new_account_at(AccountAddress::from_hex_literal("0xa11ce").unwrap());
    let bob = harness.new_account_at(AccountAddress::from_hex_literal("0xb0b").unwrap());

    transaction_fee::initialize_fee_collection_and_distribution(&mut harness, burn_percentage);
    harness.enable_features(vec![FeatureFlag::COLLECT_AND_DISTRIBUTE_GAS_FEES], vec![]);

    let core_resources =
        harness.new_account_at(AccountAddress::from_hex_literal("0xA550C18").unwrap());
    
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
    let mut txns = p2p_txns_for_test(&mut harness, &alice, &bob, 10);
    txns.insert(10, disable_script);

    // Simulate block execution.
    let supply_before = harness.executor.read_coin_supply().unwrap();
    let gas_used = harness.run_block_with_metadata(*validator.address(), vec![], txns);
    let gas_used_for_proposal = gas_used - *P2P_TXN_GAS_COST * 10;

    let burnt_amount = (burn_percentage as u64) * (gas_used - gas_used_for_proposal) / 100;
    let collected_amount = gas_used - gas_used_for_proposal - burnt_amount;

    // Reconfiguration triggers distributing rewards and fees.
    stake_amount += rewards_per_epoch + collected_amount;
    assert_eq!(
        get_stake_pool(&harness, validator.address()).active,
        stake_amount
    );

    harness.new_block_with_metadata(*validator.address(), vec![]);
    let supply_after = harness.executor.read_coin_supply().unwrap();

    // Gas for the proposal should is burnt together with fraction of fees.
    assert_eq!(
        supply_after.abs_diff(supply_before - gas_used_for_proposal as u128 - burnt_amount as u128 + rewards_per_epoch as u128),
        0
    );
}

fn test_upgrade_burn_percentage(burn_percentage: u8) {
    let mut harness = MoveHarness::new();
    let alice = harness.new_account_at(AccountAddress::from_hex_literal("0xa11ce").unwrap());
    let bob = harness.new_account_at(AccountAddress::from_hex_literal("0xb0b").unwrap());

    transaction_fee::initialize_fee_collection_and_distribution(&mut harness, burn_percentage);
    harness.enable_features(vec![FeatureFlag::COLLECT_AND_DISTRIBUTE_GAS_FEES], vec![]);

    let core_resources =
        harness.new_account_at(AccountAddress::from_hex_literal("0xA550C18").unwrap());
    
    let rewards_per_epoch = 285;
    let mut stake_amount = 25_000_000;
    let validator = harness.new_account_at(AccountAddress::from_hex_literal("0x123").unwrap());
    assert_success!(setup_staking(&mut harness, &validator, stake_amount));
    harness.new_epoch();

    let upgrade_script = create_script(
        &mut harness, 
        "upgrade_burn_percentage",
        &core_resources,
        vec![TransactionArgument::U8(100 - burn_percentage)]
    );

    // Create a block of transactions such that:
    //   1. First 10 transactions are p2p.
    //   2. A single transaction upgrading burn percentage.
    //   3. Remaining transactions are p2p (should end up being Retry).
    let mut txns = p2p_txns_for_test(&mut harness, &alice, &bob, 10);
    txns.insert(10, upgrade_script);

    // Simulate block execution.
    let supply_before = harness.executor.read_coin_supply().unwrap();
    let gas_used = harness.run_block_with_metadata(*validator.address(), vec![], txns);
    let gas_used_for_proposal = gas_used - *P2P_TXN_GAS_COST * 10;

    let burnt_amount = (burn_percentage as u64) * (gas_used - gas_used_for_proposal) / 100;
    let collected_amount = gas_used - gas_used_for_proposal - burnt_amount;
    // println!("burnt: {}", burnt_amount);
    // println!("collected: {}", collected_amount);

    // Reconfiguration triggers distributing rewards and fees.
    stake_amount += rewards_per_epoch + collected_amount;
    assert_eq!(
        get_stake_pool(&harness, validator.address()).active,
        stake_amount
    );

    harness.new_block_with_metadata(*validator.address(), vec![]);
    let supply_after = harness.executor.read_coin_supply().unwrap();

    // Gas for the proposal should is burnt together with fraction of fees.
    assert_eq!(
        supply_after.abs_diff(supply_before - gas_used_for_proposal as u128 - burnt_amount as u128 + rewards_per_epoch as u128),
        0
    );

    // While fees collection is enabled...
    let carol = harness.new_account_at(AccountAddress::from_hex_literal("0xc0101").unwrap());
    let david = harness.new_account_at(AccountAddress::from_hex_literal("0xda51d").unwrap());
    let supply_before = harness.executor.read_coin_supply().unwrap();
    let txns = p2p_txns_for_test(&mut harness, &carol, &david, 10);
    let gas_used = harness.run_block_with_metadata(*validator.address(), vec![], txns);

    // Run an empty block to drain the aggregatable coin. Fees are assigned to
    // validators and therefore the total supply changes only because some
    // percentage was burnt.
    harness.new_block_with_metadata(*validator.address(), vec![]);
    let supply_after = harness.executor.read_coin_supply().unwrap();
    let burnt_amount = ((100 - burn_percentage) as u64) * gas_used / 100;
    let collected_amount = gas_used - burnt_amount;
    assert_eq!(
        supply_after.abs_diff(supply_before - burnt_amount as u128),
        0
    );
    // println!("burnt: {}", burnt_amount);
    // println!("collected: {}", collected_amount);

    harness.new_epoch();
    stake_amount += rewards_per_epoch + collected_amount;
    assert_eq!(
        get_stake_pool(&harness, validator.address()).active,
        stake_amount
    );
}

#[test]
fn test_fee_collection_and_distribution_for_burn_percentages() {
    // Test multiple burn percentages including the cases of 0 and 100.
    for burn_percentage in [100, 75, 25, 0] {
        test_fee_collection_and_distribution_flow(burn_percentage);
        test_initialize_and_enable_fee_collection_and_distribution(burn_percentage);
        test_disable_fee_collection(burn_percentage);
        test_upgrade_burn_percentage(burn_percentage);
    }
}

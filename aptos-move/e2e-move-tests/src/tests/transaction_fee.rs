// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    assert_success, get_stake_pool, setup_staking, tests::common, transaction_fee, MoveHarness,
};
use aptos_cached_packages::aptos_stdlib;
use aptos_language_e2e_tests::{account::Account, executor::FakeExecutor};
use aptos_types::{
    account_address::AccountAddress,
    on_chain_config::FeatureFlag,
    transaction::{SignedTransaction, TransactionArgument, TransactionStatus},
};
use once_cell::sync::Lazy;
use rand::{
    distributions::Uniform,
    rngs::{OsRng, StdRng},
    Rng, SeedableRng,
};
use std::collections::BTreeMap;

pub static PROPOSAL_SCRIPTS: Lazy<BTreeMap<String, Vec<u8>>> = Lazy::new(build_scripts);

fn build_scripts() -> BTreeMap<String, Vec<u8>> {
    let package_folder = "transaction_fee.data";
    let package_names = vec![
        "initialize_collection",
        "enable_collection",
        "disable_collection",
        "upgrade_burn_percentage",
        "remove_validator",
    ];
    common::build_scripts(package_folder, package_names)
}

// Constants for calculating rewards for validators at the end of each epoch.
const INITIAL_STAKE_AMOUNT: u64 = 25_000_000;
const REWARDS_RATE_DENOMINATOR: u64 = 1_000_000_000;

// Each epoch takes 1 hour in genesis config for tests.
const NUM_EPOCHS_IN_A_YEAR: u64 = 365 * 24;
const REWARDS_RATE: u64 = (10 * REWARDS_RATE_DENOMINATOR / 100) / NUM_EPOCHS_IN_A_YEAR;

/// Holds all information about the current state of the chain, including
/// accounts of users, validators, etc.
struct TestUniverse {
    harness: MoveHarness,
    core_resources: Account,
    validators: Vec<Account>,
    users: Vec<Account>,
}

// Make sure the number of users in this test universe is large enough.
const NUM_USERS: usize = 200;

impl TestUniverse {
    /// Creates a new testing universe with all necessary accounts created.
    pub fn new(num_validators: usize) -> Self {
        let executor = FakeExecutor::from_head_genesis().set_parallel();
        let mut harness = MoveHarness::new_with_executor(executor);
        harness.set_default_gas_unit_price(1);
        let core_resources =
            harness.new_account_at(AccountAddress::from_hex_literal("0xA550C18").unwrap());
        let validators: Vec<Account> = (0..num_validators)
            .map(|idx| {
                harness.new_account_at(
                    AccountAddress::from_hex_literal(format!("0xa{}", idx).as_str()).unwrap(),
                )
            })
            .collect();
        let users: Vec<Account> = (0..NUM_USERS)
            .map(|idx| {
                harness.new_account_at(
                    AccountAddress::from_hex_literal(format!("0xb{}", idx).as_str()).unwrap(),
                )
            })
            .collect();
        Self {
            harness,
            core_resources,
            validators,
            users,
        }
    }

    /// Initializes validators with the given stake amount.
    pub fn set_up_validators(&mut self, stake_amount: u64) {
        self.validators.iter().for_each(|validator| {
            assert_success!(setup_staking(&mut self.harness, validator, stake_amount));
        });
        self.harness.new_epoch();
    }

    /// Creates a block of p2p transactions in the universe.
    pub fn create_block(&mut self, num_txns: usize) -> Vec<SignedTransaction> {
        let mut rng = StdRng::from_seed(OsRng.gen());
        let num_users = self.users.len();
        (0..num_txns)
            .map(|_| {
                // Select random users.
                let src_account = &self.users[rng.sample(Uniform::new(0, num_users))];
                let dst_account = &self.users[rng.sample(Uniform::new(0, num_users))];

                // Create a new p2p transaction.
                self.harness.create_transaction_payload(
                    src_account,
                    aptos_stdlib::aptos_coin_transfer(*dst_account.address(), 1),
                )
            })
            .collect()
    }

    /// Injects a governance proposal script into transaction block.
    pub fn inject_proposal_into_block(
        &mut self,
        block: &mut Vec<SignedTransaction>,
        proposal_idx: usize,
        package_name: &str,
        args: Vec<TransactionArgument>,
    ) {
        let script_code = PROPOSAL_SCRIPTS
            .get(package_name)
            .expect("proposal script should be built");
        let sender = &self.core_resources;
        let txn = self
            .harness
            .create_script(sender, script_code.clone(), vec![], args);

        debug_assert!(proposal_idx <= block.len());
        block.insert(proposal_idx, txn);
    }

    /// Returns the total supply of AptosCoin in the universe.
    pub fn read_total_supply(&self) -> u128 {
        self.harness.executor.read_coin_supply().unwrap()
    }
}

/// For the given transaction outputs, calculates how much gas the block costs.
/// If there was a reconfiguration during the block execution, the cost is split
/// into before transaction triggering reconfiguration, and the cost of
/// transaction (governance proposal) triggering the reconfiguration. Note that
/// all outputs after reconfiguration should be Retry.
fn calculate_gas_used(outputs: Vec<(TransactionStatus, u64)>) -> (u64, u64) {
    let mut found_reconfig = false;
    let mut gas_used_for_reconfig = 0;
    let mut total_gas_used = 0;
    for (status, gas_used) in outputs {
        total_gas_used += gas_used;

        if let TransactionStatus::Retry = status {
            if !found_reconfig {
                found_reconfig = true;
            }
            debug_assert!(gas_used == 0);
        } else if !found_reconfig {
            gas_used_for_reconfig = gas_used;
        }
    }
    if !found_reconfig {
        gas_used_for_reconfig = 0;
    }
    (
        total_gas_used - gas_used_for_reconfig,
        gas_used_for_reconfig,
    )
}

/// Tests a standard flow of collecting fees without any edge cases.
fn test_fee_collection_and_distribution_flow(burn_percentage: u8) {
    let num_validators = 1;
    let mut universe = TestUniverse::new(num_validators);
    transaction_fee::initialize_fee_collection_and_distribution(
        &mut universe.harness,
        burn_percentage,
    );
    universe
        .harness
        .enable_features(vec![FeatureFlag::COLLECT_AND_DISTRIBUTE_GAS_FEES], vec![]);

    let mut stake_amount = INITIAL_STAKE_AMOUNT;
    universe.set_up_validators(stake_amount);
    let rewards_per_epoch = stake_amount * REWARDS_RATE / REWARDS_RATE_DENOMINATOR;
    assert_eq!(rewards_per_epoch, 285);

    // Run a single block and record how much gas it costs. Since fee collection
    // is enabled, this amount is stored in aggregatable coin.
    let validator_addr = *universe.validators[0].address();
    let txns = universe.create_block(1000);

    let mut total_supply = universe.read_total_supply();
    let outputs = universe
        .harness
        .run_block_with_metadata(validator_addr, vec![], txns);
    let (p2p_gas, proposal_gas) = calculate_gas_used(outputs);
    assert_eq!(proposal_gas, 0);

    let burnt_amount = (burn_percentage as u64) * p2p_gas / 100;
    let collected_amount = p2p_gas - burnt_amount;

    // Drain aggregatable coin in the next block.
    universe
        .harness
        .new_block_with_metadata(validator_addr, vec![]);
    total_supply -= burnt_amount as u128;

    // Check that the right fraction was burnt.
    assert_eq!(universe.read_total_supply(), total_supply);

    // On the new epoch, the collected fees are processed and added to the stake
    // pool.
    universe.harness.new_epoch();
    stake_amount += rewards_per_epoch + collected_amount;
    assert_eq!(
        get_stake_pool(&universe.harness, &validator_addr).active,
        stake_amount
    );
}

/// Tests if fees collection can be enabled by the governance proposal and how
/// fees are collected on the block boundary.
fn test_initialize_and_enable_fee_collection_and_distribution(burn_percentage: u8) {
    let num_validators = 1;
    let mut universe = TestUniverse::new(num_validators);

    let mut stake_amount = INITIAL_STAKE_AMOUNT;
    universe.set_up_validators(stake_amount);
    let rewards_per_epoch = stake_amount * REWARDS_RATE / REWARDS_RATE_DENOMINATOR;
    assert_eq!(rewards_per_epoch, 285);

    // Create a block of transactions such that:
    //   1. First 10 transactions are p2p.
    //   2. A single transaction placing resources on chain.
    //   3. Another 10 transactions are p2p.
    //   4. A single transaction enabling fees collection.
    //   5. Remaining transactions are p2p (should end up being Retry).
    let mut txns = universe.create_block(50);
    universe.inject_proposal_into_block(&mut txns, 10, "initialize_collection", vec![
        TransactionArgument::U8(burn_percentage),
    ]);
    universe.inject_proposal_into_block(&mut txns, 21, "enable_collection", vec![]);

    // Simulate block execution.
    let mut total_supply = universe.read_total_supply();
    let validator_addr = *universe.validators[0].address();
    let outputs = universe
        .harness
        .run_block_with_metadata(validator_addr, vec![], txns);
    let (gas_used, proposal_gas) = calculate_gas_used(outputs);

    // Reconfiguration triggers distributing rewards.
    total_supply -= gas_used as u128;
    total_supply += rewards_per_epoch as u128;
    stake_amount += rewards_per_epoch;
    assert_eq!(universe.read_total_supply(), total_supply);
    assert_eq!(
        get_stake_pool(&universe.harness, &validator_addr).active,
        stake_amount
    );

    // In the previous block, the fee was only collected for the last script
    // transaction which enabled the feature. In this block, we drain
    // aggregatable coin and try to assign the fee to the validator. Since the
    // proposer is not set (when feature flag was enabled), the fee is simply
    // burnt and the stake pool should have the same value.
    universe
        .harness
        .new_block_with_metadata(validator_addr, vec![]);
    total_supply -= proposal_gas as u128;
    assert_eq!(universe.read_total_supply(), total_supply);
    assert_eq!(
        get_stake_pool(&universe.harness, &validator_addr).active,
        stake_amount
    );
}

/// Tests fee collection can be safely disabled. The corner case here is that by disabling
/// the flag, we cannot distribute fees anymore unless it is done beforehand.
fn test_disable_fee_collection(burn_percentage: u8) {
    let num_validators = 1;
    let mut universe = TestUniverse::new(num_validators);
    transaction_fee::initialize_fee_collection_and_distribution(
        &mut universe.harness,
        burn_percentage,
    );
    universe
        .harness
        .enable_features(vec![FeatureFlag::COLLECT_AND_DISTRIBUTE_GAS_FEES], vec![]);

    let mut stake_amount = INITIAL_STAKE_AMOUNT;
    universe.set_up_validators(stake_amount);
    let rewards_per_epoch = stake_amount * REWARDS_RATE / REWARDS_RATE_DENOMINATOR;
    assert_eq!(rewards_per_epoch, 285);

    // Create a block of transactions such that:
    //   1. First 10 transactions are p2p.
    //   2. A single transaction disabling fees collection.
    //   3. Remaining transactions are p2p (should end up being Retry).
    let mut txns = universe.create_block(100);
    universe.inject_proposal_into_block(&mut txns, 10, "disable_collection", vec![]);
    let validator_addr = *universe.validators[0].address();

    // Simulate block execution.
    let mut total_supply = universe.read_total_supply();
    let outputs = universe
        .harness
        .run_block_with_metadata(validator_addr, vec![], txns);
    let (p2p_gas, proposal_gas) = calculate_gas_used(outputs);

    // Calculate the fees taht are supposed to be collected before the feature
    // is disabled.
    let burnt_amount = (burn_percentage as u64) * p2p_gas / 100;
    let collected_amount = p2p_gas - burnt_amount;

    // Reconfiguration triggers distribution of both rewards and fees.
    stake_amount += rewards_per_epoch + collected_amount;
    total_supply += rewards_per_epoch as u128;
    total_supply -= burnt_amount as u128;
    assert_eq!(
        get_stake_pool(&universe.harness, &validator_addr).active,
        stake_amount
    );

    // Gas for the proposal should be burnt together with the fraction of the
    // fees.
    universe
        .harness
        .new_block_with_metadata(validator_addr, vec![]);
    total_supply -= proposal_gas as u128;
    assert_eq!(universe.read_total_supply(), total_supply);
}

/// Tests that the fees collected prior to the upgrade use the right burn
/// percentage for calculations.
fn test_upgrade_burn_percentage(burn_percentage: u8) {
    let num_validators = 2;
    let mut universe = TestUniverse::new(num_validators);
    transaction_fee::initialize_fee_collection_and_distribution(
        &mut universe.harness,
        burn_percentage,
    );
    universe
        .harness
        .enable_features(vec![FeatureFlag::COLLECT_AND_DISTRIBUTE_GAS_FEES], vec![]);

    let mut stake_amount = INITIAL_STAKE_AMOUNT;
    universe.set_up_validators(stake_amount);

    // Upgrade to the opposite value.
    let new_burn_percentage = 100 - burn_percentage;

    // Create a block of transactions such that:
    //   1. First 10 transactions are p2p.
    //   2. A single transaction upgrading the burn percentage.
    //   3. Remaining transactions are p2p (should end up being Retry).
    let mut txns = universe.create_block(100);
    universe.inject_proposal_into_block(&mut txns, 10, "upgrade_burn_percentage", vec![
        TransactionArgument::U8(new_burn_percentage),
    ]);
    let validator_addr = *universe.validators[0].address();

    // Simulate block execution.
    let mut total_supply = universe.read_total_supply();
    let outputs = universe
        .harness
        .run_block_with_metadata(validator_addr, vec![], txns);
    let (p2p_gas, proposal_gas) = calculate_gas_used(outputs);

    let burnt_amount = (burn_percentage as u64) * p2p_gas / 100;
    let collected_amount = p2p_gas - burnt_amount;

    // Compute rewards for this epoch.
    let rewards_per_epoch = stake_amount * REWARDS_RATE / REWARDS_RATE_DENOMINATOR;
    assert_eq!(rewards_per_epoch, 285);

    // Reconfiguration triggers distribution of rewards and fees.
    stake_amount += rewards_per_epoch + collected_amount;
    total_supply += rewards_per_epoch as u128;
    total_supply -= burnt_amount as u128;
    assert_eq!(
        get_stake_pool(&universe.harness, &validator_addr).active,
        stake_amount
    );

    // Gas for the proposal should be burnt together with fraction of fees.
    universe
        .harness
        .new_block_with_metadata(validator_addr, vec![]);
    total_supply -= proposal_gas as u128;
    assert_eq!(universe.read_total_supply(), total_supply);

    // Now check that the new burn percentage works correctly.
    let txns = universe.create_block(100);
    total_supply = universe.read_total_supply();
    let outputs = universe
        .harness
        .run_block_with_metadata(validator_addr, vec![], txns);
    let (gas_used, proposal_gas) = calculate_gas_used(outputs);
    assert_eq!(proposal_gas, 0);

    let burnt_amount = (new_burn_percentage as u64) * gas_used / 100;
    let collected_amount = gas_used - burnt_amount;

    // Check that the new fraction of fees is burnt.
    universe
        .harness
        .new_block_with_metadata(validator_addr, vec![]);
    total_supply -= burnt_amount as u128;
    assert_eq!(universe.read_total_supply(), total_supply);

    // Check fees are distributed during the next epoch. Make sure to
    // recalculate the rewards as well.
    universe.harness.new_epoch();

    // Compute rewards for this epoch.
    let rewards_per_epoch = stake_amount * REWARDS_RATE / REWARDS_RATE_DENOMINATOR;
    stake_amount += rewards_per_epoch + collected_amount;
    assert_eq!(
        get_stake_pool(&universe.harness, &validator_addr).active,
        stake_amount
    );
}

/// Tests that if validator running the block is removed, it still receives
/// previously collected fees.
fn test_leaving_validator_is_rewarded(burn_percentage: u8) {
    let num_validators = 2;
    let mut universe = TestUniverse::new(num_validators);
    transaction_fee::initialize_fee_collection_and_distribution(
        &mut universe.harness,
        burn_percentage,
    );
    universe
        .harness
        .enable_features(vec![FeatureFlag::COLLECT_AND_DISTRIBUTE_GAS_FEES], vec![]);

    let mut stake_amount = INITIAL_STAKE_AMOUNT;
    universe.set_up_validators(stake_amount);
    let rewards_per_epoch = stake_amount * REWARDS_RATE / REWARDS_RATE_DENOMINATOR;
    assert_eq!(rewards_per_epoch, 285);

    // Create a block of transactions such that:
    //   1. First 10 transactions are p2p.
    //   2. A single transaction removing the validator.
    //   3. Remaining transactions are p2p (should end up being Retry).
    let removed_validator_addr = *universe.validators[0].address();
    let mut txns = universe.create_block(20);
    universe.inject_proposal_into_block(&mut txns, 10, "remove_validator", vec![
        TransactionArgument::Address(removed_validator_addr),
    ]);

    // Simulate block execution and calculate how much gas was used for
    // transactions and for the governance proposal.
    let mut total_supply = universe.read_total_supply();
    let outputs = universe
        .harness
        .run_block_with_metadata(removed_validator_addr, vec![], txns);
    let (p2p_gas, proposal_gas) = calculate_gas_used(outputs);

    let burnt_amount = (burn_percentage as u64) * p2p_gas / 100;
    let collected_amount = p2p_gas - burnt_amount;

    // Reconfiguration triggers distributing rewards and fees.
    stake_amount += rewards_per_epoch + collected_amount;
    total_supply += rewards_per_epoch as u128;
    total_supply -= burnt_amount as u128;
    assert_eq!(
        get_stake_pool(&universe.harness, &removed_validator_addr).active,
        stake_amount
    );

    let remaining_validator_addr = *universe.validators[1].address();
    universe
        .harness
        .new_block_with_metadata(remaining_validator_addr, vec![]);
    total_supply -= proposal_gas as u128;
    assert_eq!(universe.read_total_supply(), total_supply);
}

#[test]
fn test_fee_collection_and_distribution_for_burn_percentages() {
    // Test multiple burn percentages including the cases of 0 and 100.
    for burn_percentage in [0, 50, 100] {
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
    let num_validators = 1;
    let mut universe = TestUniverse::new(num_validators);
    transaction_fee::initialize_fee_collection_and_distribution(&mut universe.harness, 100);

    let stake_amount = INITIAL_STAKE_AMOUNT;
    universe.set_up_validators(stake_amount);
    let rewards_per_epoch = stake_amount * REWARDS_RATE / REWARDS_RATE_DENOMINATOR;
    assert_eq!(rewards_per_epoch, 285);

    // Create block with a single transaction: governance proposal to enable
    // fee collection. This proposal ends the epoch.
    let mut txns = vec![];
    universe.inject_proposal_into_block(&mut txns, 0, "enable_collection", vec![]);
    let validator_addr = *universe.validators[0].address();

    let mut total_supply = universe.read_total_supply();
    let outputs = universe
        .harness
        .run_block_with_metadata(validator_addr, vec![], txns);
    let proposal_gas = outputs[1].1;

    // After reconfiguration rewards will be distributed. Because there are no
    // other transactions, there is nothing to drain. However, this still should
    // unset the proposer so that the next block burns the proposal fee.
    total_supply += rewards_per_epoch as u128;
    assert_eq!(universe.read_total_supply(), total_supply);

    // Ensure the fees are not leaked to the next block. This block must burn
    // the proposal fee.
    universe
        .harness
        .new_block_with_metadata(validator_addr, vec![]);
    total_supply -= proposal_gas as u128;
    assert_eq!(universe.read_total_supply(), total_supply);
}

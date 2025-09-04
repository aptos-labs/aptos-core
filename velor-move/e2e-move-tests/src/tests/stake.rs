// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    assert_success, get_stake_pool, get_validator_config, get_validator_set, initialize_staking,
    join_validator_set, leave_validator_set, rotate_consensus_key, setup_staking, tests::common,
    unlock_stake, withdraw_stake, MoveHarness,
};
use velor_cached_packages::velor_stdlib;
use velor_types::account_address::{default_stake_pool_address, AccountAddress};
use once_cell::sync::Lazy;
use std::collections::BTreeMap;

pub static PROPOSAL_SCRIPTS: Lazy<BTreeMap<String, Vec<u8>>> = Lazy::new(build_scripts);

fn build_scripts() -> BTreeMap<String, Vec<u8>> {
    let package_folder = "stake.data";
    let package_names = vec!["update_rewards_config"];
    common::build_scripts(package_folder, package_names)
}

fn update_stake_amount_and_assert_with_errors(
    harness: &mut MoveHarness,
    stake_amount: &mut u64,
    validator_address: AccountAddress,
    rewards_rate: u64,
    rewards_rate_denominator: u64,
) {
    *stake_amount += *stake_amount * rewards_rate / rewards_rate_denominator;
    // The calculation uses fixed_point64 so we allow errors up to 1.
    assert!(
        *stake_amount - 1 <= get_stake_pool(harness, &validator_address).active
            && get_stake_pool(harness, &validator_address).active <= *stake_amount + 1
    );
    *stake_amount = get_stake_pool(harness, &validator_address).active;
}

#[test]
fn test_staking_end_to_end() {
    let mut harness = MoveHarness::new();
    let owner = harness.new_account_at(AccountAddress::from_hex_literal("0x123").unwrap());
    let operator = harness.new_account_at(AccountAddress::from_hex_literal("0x234").unwrap());
    let owner_address = *owner.address();
    let operator_address = *operator.address();

    // Initialize and add stake.
    let stake_amount = 50_000_000;
    assert_success!(initialize_staking(
        &mut harness,
        &owner,
        stake_amount,
        operator_address,
        owner_address
    ));
    let stake_pool = get_stake_pool(&harness, &owner_address);
    assert_eq!(stake_pool.active, stake_amount);
    assert_eq!(stake_pool.operator_address, operator_address);
    assert_eq!(stake_pool.delegated_voter, owner_address);

    // Join the validator set.
    assert_success!(rotate_consensus_key(&mut harness, &operator, owner_address));
    assert_success!(join_validator_set(&mut harness, &operator, owner_address));
    harness.new_epoch();

    // Validator should now be locked up.
    let stake_pool = get_stake_pool(&harness, &owner_address);
    assert_eq!(
        stake_pool.locked_until_secs,
        harness.executor.get_block_time_seconds() + 7200
    );

    // Unlock 1/4 stake.
    let amount_to_withdraw = stake_amount / 4;
    let remaining_stake = stake_amount - amount_to_withdraw;
    assert_success!(unlock_stake(&mut harness, &owner, amount_to_withdraw));
    let stake_pool = get_stake_pool(&harness, &owner_address);
    assert_eq!(stake_pool.active, remaining_stake);
    assert_eq!(stake_pool.pending_inactive, amount_to_withdraw);

    // Wait until stake is unlocked.
    harness.fast_forward(7200);
    harness.new_epoch();
    let stake_pool = get_stake_pool(&harness, &owner_address);
    assert_eq!(stake_pool.pending_inactive, 0);
    assert_eq!(stake_pool.inactive, amount_to_withdraw);

    // Withdraw and verify that coins are returned.
    assert_success!(withdraw_stake(&mut harness, &owner, stake_amount / 2));
    let stake_pool = get_stake_pool(&harness, &owner_address);
    assert_eq!(stake_pool.inactive, 0);

    // Verify that lockup has been renewed for remaining stake.
    assert_eq!(stake_pool.active, remaining_stake);
    assert_eq!(
        stake_pool.locked_until_secs,
        harness.executor.get_block_time_seconds() + 7200
    );

    // Validator takes the rest of the stake out.
    assert_success!(unlock_stake(&mut harness, &owner, remaining_stake));
    harness.fast_forward(7200);
    harness.new_epoch();
    assert_success!(withdraw_stake(&mut harness, &owner, remaining_stake));
    let stake_pool = get_stake_pool(&harness, &owner_address);
    assert_eq!(stake_pool.active, 0);
    assert_eq!(stake_pool.inactive, 0);
}

#[test]
fn test_staking_rewards() {
    // Genesis starts with one validator with index 0
    let mut harness = MoveHarness::new();
    let validator_1 = harness.new_account_at(AccountAddress::from_hex_literal("0x123").unwrap());
    let validator_2 = harness.new_account_at(AccountAddress::from_hex_literal("0x234").unwrap());
    let validator_1_address = *validator_1.address();
    let validator_2_address = *validator_2.address();

    // Initialize the validators.
    let rewards_per_epoch = 285;
    let mut stake_amount_2 = 25_000_000;
    assert_success!(setup_staking(&mut harness, &validator_2, stake_amount_2));
    let mut stake_amount_1 = 25_000_000;
    assert_success!(setup_staking(&mut harness, &validator_1, stake_amount_1));
    harness.new_epoch();

    // Both validators propose a block in the current epoch. Both should receive rewards.
    harness.new_block_with_metadata(validator_1_address, vec![]);
    harness.new_block_with_metadata(validator_2_address, vec![]);
    harness.new_epoch();
    stake_amount_1 += rewards_per_epoch;
    stake_amount_2 += rewards_per_epoch;
    assert_eq!(
        get_stake_pool(&harness, &validator_1_address).active,
        stake_amount_1
    );
    assert_eq!(
        get_stake_pool(&harness, &validator_2_address).active,
        stake_amount_2
    );

    let index_1 = get_validator_config(&harness, &validator_1_address).validator_index as u32;

    // Each validator proposes in their own epoch. They receive the rewards at the end of each epoch
    harness.new_block_with_metadata(validator_1_address, vec![]);
    harness.new_epoch();
    stake_amount_1 += rewards_per_epoch;
    assert_eq!(
        get_stake_pool(&harness, &validator_1_address).active,
        stake_amount_1
    );
    assert_eq!(
        get_stake_pool(&harness, &validator_2_address).active,
        stake_amount_2
    );
    harness.new_block_with_metadata(validator_2_address, vec![]);
    harness.new_epoch();
    assert_eq!(
        get_stake_pool(&harness, &validator_1_address).active,
        stake_amount_1
    );
    stake_amount_2 += rewards_per_epoch;
    assert_eq!(
        get_stake_pool(&harness, &validator_2_address).active,
        stake_amount_2
    );

    // Validator 1 misses one proposal and thus receives no rewards while validator 2 didn't miss
    // any so they receive full rewards.
    harness.new_block_with_metadata(validator_2_address, vec![index_1]);
    harness.new_epoch();
    assert_eq!(
        get_stake_pool(&harness, &validator_1_address).active,
        stake_amount_1
    );
    stake_amount_2 += rewards_per_epoch;
    assert_eq!(
        get_stake_pool(&harness, &validator_2_address).active,
        stake_amount_2
    );

    // Validator 1 misses one proposal but has one successful so they receive half of the rewards.
    harness.new_block_with_metadata(validator_1_address, vec![index_1]);
    harness.new_epoch();
    stake_amount_1 += rewards_per_epoch / 2;
    assert_eq!(
        get_stake_pool(&harness, &validator_1_address).active,
        stake_amount_1
    );

    harness.new_block_with_metadata(validator_1_address, vec![]);
    harness.new_block_with_metadata(validator_2_address, vec![]);

    // Enable rewards rate decrease and change rewards config. In production it requires governance.
    let core_resources =
        harness.new_account_at(AccountAddress::from_hex_literal("0xA550C18").unwrap());
    let script_code = PROPOSAL_SCRIPTS
        .get("update_rewards_config")
        .expect("proposal script should be built");
    let txn = harness.create_script(&core_resources, script_code.clone(), vec![], vec![]);
    assert_success!(harness.run(txn));

    // Parameters from the proposal.
    let one_year_in_secs: u64 = 365 * 24 * 60 * 60;
    let mut rewards_rate: u64 = 100;
    let min_rewards_rate: u64 = 30;
    let rewards_rate_denominator: u64 = 10000;
    let rewards_rate_decrease_rate_bps: u64 = 5000;
    let bps_denominator: u64 = 10000;

    // The initialize script calls reconfigure(). This epoch ends immediately.
    // Both validators propose a block in the current epoch. Both should receive rewards at the
    // new rewards rate, 1% every epoch.
    update_stake_amount_and_assert_with_errors(
        &mut harness,
        &mut stake_amount_1,
        validator_1_address,
        rewards_rate,
        rewards_rate_denominator,
    );
    update_stake_amount_and_assert_with_errors(
        &mut harness,
        &mut stake_amount_2,
        validator_2_address,
        rewards_rate,
        rewards_rate_denominator,
    );

    // 0.5 year passed. Rewards rate halves. Rewards rate doesn't change.
    // Both validators propose a block in the current epoch. Both should receive rewards.
    harness.new_block_with_metadata(validator_1_address, vec![]);
    harness.new_block_with_metadata(validator_2_address, vec![]);
    harness.fast_forward(one_year_in_secs / 2);
    harness.new_epoch();
    update_stake_amount_and_assert_with_errors(
        &mut harness,
        &mut stake_amount_1,
        validator_1_address,
        rewards_rate,
        rewards_rate_denominator,
    );
    update_stake_amount_and_assert_with_errors(
        &mut harness,
        &mut stake_amount_2,
        validator_2_address,
        rewards_rate,
        rewards_rate_denominator,
    );

    // Another 0.5 year passed. Rewards rate halves. New rewards after this epoch rate is 0.5% every epoch.
    // Both validators propose a block in the current epoch. Both should receive rewards.
    harness.new_block_with_metadata(validator_1_address, vec![]);
    harness.new_block_with_metadata(validator_2_address, vec![]);
    harness.fast_forward(one_year_in_secs / 2);
    harness.new_epoch();
    update_stake_amount_and_assert_with_errors(
        &mut harness,
        &mut stake_amount_1,
        validator_1_address,
        rewards_rate,
        rewards_rate_denominator,
    );
    update_stake_amount_and_assert_with_errors(
        &mut harness,
        &mut stake_amount_2,
        validator_2_address,
        rewards_rate,
        rewards_rate_denominator,
    );
    rewards_rate = rewards_rate * rewards_rate_decrease_rate_bps / bps_denominator;

    // Another new epoch, both validators receive rewards in 0.5% every epoch.
    // Another year passed. Rewards rate halves but it cannot be lower than 0.3%.
    // New rewards rate of the next epoch is 0.3% every epoch.
    harness.new_block_with_metadata(validator_1_address, vec![]);
    harness.new_block_with_metadata(validator_2_address, vec![]);
    harness.fast_forward(one_year_in_secs);
    harness.new_epoch();
    update_stake_amount_and_assert_with_errors(
        &mut harness,
        &mut stake_amount_1,
        validator_1_address,
        rewards_rate,
        rewards_rate_denominator,
    );
    update_stake_amount_and_assert_with_errors(
        &mut harness,
        &mut stake_amount_2,
        validator_2_address,
        rewards_rate,
        rewards_rate_denominator,
    );

    // Validator 1 misses one proposal but has one successful so they receive half of the rewards.
    harness.new_block_with_metadata(validator_1_address, vec![index_1]);
    harness.new_epoch();
    rewards_rate = min_rewards_rate / 2;
    update_stake_amount_and_assert_with_errors(
        &mut harness,
        &mut stake_amount_1,
        validator_1_address,
        rewards_rate,
        rewards_rate_denominator,
    );
    assert_eq!(
        get_stake_pool(&harness, &validator_2_address).active,
        stake_amount_2
    );
}

#[test]
fn test_staking_rewards_pending_inactive() {
    let mut harness = MoveHarness::new();
    let validator = harness.new_account_at(AccountAddress::from_hex_literal("0x123").unwrap());
    let validator_address = *validator.address();

    // Initialize the validator.
    let stake_amount = 50_000_000;
    setup_staking(&mut harness, &validator, stake_amount);
    harness.new_epoch();

    // Validator requests to leave.
    leave_validator_set(&mut harness, &validator, validator_address);
    let validator_set = get_validator_set(&harness);
    assert_eq!(
        validator_set.pending_inactive[0].account_address,
        validator_address
    );

    // Validator proposes a block in the current epoch and should receive rewards despite
    // being pending_inactive.
    harness.new_block_with_metadata(validator_address, vec![]);
    harness.new_epoch();
    assert_eq!(
        get_stake_pool(&harness, &validator_address).active,
        stake_amount + 570
    );
}

#[test]
fn test_staking_contract() {
    let mut harness = MoveHarness::new();
    let staker = harness.new_account_at(AccountAddress::from_hex_literal("0x11").unwrap());
    let operator_1 = harness.new_account_at(AccountAddress::from_hex_literal("0x21").unwrap());
    let operator_2 = harness.new_account_at(AccountAddress::from_hex_literal("0x22").unwrap());
    let amount = 25_000_000;
    let staker_address = *staker.address();
    let operator_1_address = *operator_1.address();
    let operator_2_address = *operator_2.address();
    assert_success!(harness.run_transaction_payload(
        &staker,
        velor_stdlib::staking_contract_create_staking_contract(
            operator_1_address,
            operator_1_address,
            amount,
            10,
            vec![],
        )
    ));
    assert_success!(harness.run_transaction_payload(
        &staker,
        velor_stdlib::staking_contract_add_stake(operator_1_address, amount)
    ));

    // Join validator set.
    let pool_address = default_stake_pool_address(staker_address, operator_1_address);
    assert_success!(rotate_consensus_key(
        &mut harness,
        &operator_1,
        pool_address
    ));
    assert_success!(join_validator_set(&mut harness, &operator_1, pool_address));
    harness.new_epoch();
    let validator_set = get_validator_set(&harness);
    assert_eq!(
        validator_set.active_validators[1].account_address,
        pool_address,
    );

    // Operator requests commissions.
    harness.new_block_with_metadata(pool_address, vec![]);
    harness.new_epoch();
    assert_success!(harness.run_transaction_payload(
        &staker,
        velor_stdlib::staking_contract_request_commission(staker_address, operator_1_address)
    ));

    // Wait until stake is unlocked.
    harness.fast_forward(7200);
    harness.new_epoch();
    assert_success!(harness.run_transaction_payload(
        &staker,
        velor_stdlib::staking_contract_distribute(staker_address, operator_1_address)
    ));

    // Staker unlocks some stake.
    harness.new_block_with_metadata(pool_address, vec![]);
    harness.new_epoch();
    assert_success!(harness.run_transaction_payload(
        &staker,
        velor_stdlib::staking_contract_unlock_stake(operator_1_address, amount)
    ));

    // Wait until stake is unlocked.
    harness.fast_forward(7200);
    harness.new_epoch();
    assert_success!(harness.run_transaction_payload(
        &staker,
        velor_stdlib::staking_contract_distribute(staker_address, operator_1_address)
    ));

    // Switch operators.
    assert_success!(harness.run_transaction_payload(
        &staker,
        velor_stdlib::staking_contract_switch_operator_with_same_commission(
            operator_1_address,
            operator_2_address,
        )
    ));
    // New operator leaves validator set.
    leave_validator_set(&mut harness, &operator_2, pool_address);
}

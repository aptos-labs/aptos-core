// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_types::account_address::AccountAddress;
use e2e_move_tests::{assert_success, enable_golden, MoveHarness};

#[test]
fn test_staking_end_to_end() {
    let mut harness = MoveHarness::new();
    enable_golden!(harness);
    let owner = harness.new_account_at(AccountAddress::from_hex_literal("0x123").unwrap());
    let operator = harness.new_account_at(AccountAddress::from_hex_literal("0x234").unwrap());
    let owner_address = *owner.address();
    let operator_address = *operator.address();

    // Initialize and add stake.
    let stake_amount = 100_000_000;
    assert_success!(harness.initialize_staking(
        &owner,
        stake_amount,
        operator_address,
        owner_address
    ));
    let stake_pool = harness.get_stake_pool(&owner_address);
    assert_eq!(stake_pool.active, stake_amount);
    assert_eq!(stake_pool.operator_address, operator_address);
    assert_eq!(stake_pool.delegated_voter, owner_address);

    // Join the validator set.
    assert_success!(harness.rotate_consensus_key(&operator, owner_address));
    assert_success!(harness.join_validator_set(&operator, owner_address));
    harness.new_epoch();

    // Validator should now be locked up.
    let stake_pool = harness.get_stake_pool(&owner_address);
    assert_eq!(
        stake_pool.locked_until_secs,
        harness.executor.get_block_time_seconds() + 7200
    );

    // Unlock stake.
    assert_success!(harness.unlock_stake(&owner, stake_amount / 2));
    let stake_pool = harness.get_stake_pool(&owner_address);
    assert_eq!(stake_pool.active, stake_amount / 2);
    assert_eq!(stake_pool.pending_inactive, stake_amount / 2);

    // Wait until stake is unlocked.
    harness.fast_forward(7200);
    harness.new_epoch();
    let stake_pool = harness.get_stake_pool(&owner_address);
    assert_eq!(stake_pool.pending_inactive, 0);
    assert_eq!(stake_pool.inactive, stake_amount / 2);

    // Withdraw and verify that coins are returned.
    assert_success!(harness.withdraw_stake(&owner, stake_amount / 2));
    let stake_pool = harness.get_stake_pool(&owner_address);
    assert_eq!(stake_pool.inactive, 0);

    // Verify that lockup has been renewed for remaining stake.
    assert_eq!(stake_pool.active, stake_amount / 2);
    assert_eq!(
        stake_pool.locked_until_secs,
        harness.executor.get_block_time_seconds() + 7200
    );

    // Validator takes the rest of the stake out.
    assert_success!(harness.unlock_stake(&owner, stake_amount / 2));
    harness.fast_forward(7200);
    harness.new_epoch();
    assert_success!(harness.withdraw_stake(&owner, stake_amount / 2));
    let stake_pool = harness.get_stake_pool(&owner_address);
    assert_eq!(stake_pool.active, 0);
    assert_eq!(stake_pool.inactive, 0);
}

#[test]
fn test_staking_rewards() {
    let mut harness = MoveHarness::new();
    enable_golden!(harness);
    let validator_1 = harness.new_account_at(AccountAddress::from_hex_literal("0x123").unwrap());
    let validator_2 = harness.new_account_at(AccountAddress::from_hex_literal("0x234").unwrap());
    let validator_1_address = *validator_1.address();
    let validator_2_address = *validator_2.address();

    // Initialize the validators.
    let rewards_per_epoch = 1141;
    let mut stake_amount_1 = 100_000_000;
    harness.setup_staking(&validator_1, stake_amount_1);
    let mut stake_amount_2 = 100_000_000;
    harness.setup_staking(&validator_2, stake_amount_2);
    harness.new_epoch();

    // Both validators propose a block in the current epoch. Both should receive rewards.
    harness.new_block_with_metadata(Some(0), vec![]);
    harness.new_block_with_metadata(Some(1), vec![]);
    harness.new_epoch();
    stake_amount_1 += rewards_per_epoch;
    stake_amount_2 += rewards_per_epoch;
    assert_eq!(
        harness.get_stake_pool(&validator_1_address).active,
        stake_amount_1
    );
    assert_eq!(
        harness.get_stake_pool(&validator_2_address).active,
        stake_amount_2
    );

    // Each validator proposes in their own epoch. They receive the rewards at the end of each epoch
    harness.new_block_with_metadata(Some(0), vec![]);
    harness.new_epoch();
    stake_amount_1 += rewards_per_epoch;
    assert_eq!(
        harness.get_stake_pool(&validator_1_address).active,
        stake_amount_1
    );
    assert_eq!(
        harness.get_stake_pool(&validator_2_address).active,
        stake_amount_2
    );
    harness.new_block_with_metadata(Some(1), vec![]);
    harness.new_epoch();
    assert_eq!(
        harness.get_stake_pool(&validator_1_address).active,
        stake_amount_1
    );
    stake_amount_2 += rewards_per_epoch;
    assert_eq!(
        harness.get_stake_pool(&validator_2_address).active,
        stake_amount_2
    );

    // Validator 1 misses one proposal and thus receives no rewards while validator 2 didn't miss
    // any so they receive full rewards.
    harness.new_block_with_metadata(Some(1), vec![0]);
    harness.new_epoch();
    assert_eq!(
        harness.get_stake_pool(&validator_1_address).active,
        stake_amount_1
    );
    stake_amount_2 += rewards_per_epoch;
    assert_eq!(
        harness.get_stake_pool(&validator_2_address).active,
        stake_amount_2
    );

    // Validator 1 misses one proposal but has one successful so they receive half of the rewards.
    harness.new_block_with_metadata(Some(0), vec![0]);
    harness.new_epoch();
    stake_amount_1 += rewards_per_epoch / 2;
    assert_eq!(
        harness.get_stake_pool(&validator_1_address).active,
        stake_amount_1
    );
}

#[test]
fn test_staking_rewards_pending_inactive() {
    let mut harness = MoveHarness::new();
    enable_golden!(harness);
    let validator = harness.new_account_at(AccountAddress::from_hex_literal("0x123").unwrap());
    let validator_address = *validator.address();

    // Initialize the validator.
    let stake_amount = 100_000_000;
    harness.setup_staking(&validator, stake_amount);
    harness.new_epoch();

    // Validator requests to leave.
    harness.leave_validator_set(&validator, validator_address);
    let validator_set = harness.get_validator_set();
    assert_eq!(
        validator_set.pending_inactive[0].account_address,
        validator_address
    );

    // Validator proposes a block in the current epoch and should receive rewards despite
    // being pending_inactive.
    harness.new_block_with_metadata(Some(0), vec![]);
    harness.new_epoch();
    assert_eq!(
        harness.get_stake_pool(&validator_address).active,
        stake_amount + 1141
    );
}

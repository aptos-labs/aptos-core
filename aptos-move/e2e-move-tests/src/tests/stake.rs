// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    assert_abort, assert_success, get_stake_pool, get_validator_config, get_validator_set,
    initialize_staking, join_validator_set, leave_validator_set, rotate_consensus_key,
    setup_staking, unlock_stake, withdraw_stake, MoveHarness,
};
use aptos_types::account_address::{default_stake_pool_address, AccountAddress};
use cached_packages::aptos_stdlib;
use move_deps::move_core_types::language_storage::CORE_CODE_ADDRESS;

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
fn test_staking_mainnet() {
    // TODO: Update to have custom validators/accounts with initial balances at genesis.
    let mut harness = MoveHarness::new_mainnet();

    // Validator there's at least one validator in the validator set.
    let validator_set = get_validator_set(&harness);
    assert_eq!(validator_set.active_validators.len(), 1);

    // Verify that aptos framework account cannot mint coins.
    let aptos_framework_account = harness.new_account_at(CORE_CODE_ADDRESS);
    assert_abort!(
        harness.run_transaction_payload(
            &aptos_framework_account,
            aptos_stdlib::aptos_coin_mint(CORE_CODE_ADDRESS, 1000),
        ),
        _
    );

    // Verify that new validators can join post genesis.
    let validator = harness.new_account_at(AccountAddress::from_hex_literal("0x123").unwrap());
    assert_success!(setup_staking(&mut harness, &validator, 100_000_000_000_000));
    harness.new_epoch();
    let validator_set = get_validator_set(&harness);
    assert_eq!(validator_set.active_validators.len(), 2);
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
        aptos_stdlib::staking_contract_create_staking_contract(
            operator_1_address,
            operator_1_address,
            amount,
            10,
            vec![],
        )
    ));
    assert_success!(harness.run_transaction_payload(
        &staker,
        aptos_stdlib::staking_contract_add_stake(operator_1_address, amount)
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
        aptos_stdlib::staking_contract_request_commission(staker_address, operator_1_address)
    ));

    // Wait until stake is unlocked.
    harness.fast_forward(7200);
    harness.new_epoch();
    assert_success!(harness.run_transaction_payload(
        &staker,
        aptos_stdlib::staking_contract_distribute(staker_address, operator_1_address)
    ));

    // Staker unlocks some stake.
    harness.new_block_with_metadata(pool_address, vec![]);
    harness.new_epoch();
    assert_success!(harness.run_transaction_payload(
        &staker,
        aptos_stdlib::staking_contract_unlock_stake(operator_1_address, amount)
    ));

    // Wait until stake is unlocked.
    harness.fast_forward(7200);
    harness.new_epoch();
    assert_success!(harness.run_transaction_payload(
        &staker,
        aptos_stdlib::staking_contract_distribute(staker_address, operator_1_address)
    ));

    // Switch operators.
    assert_success!(harness.run_transaction_payload(
        &staker,
        aptos_stdlib::staking_contract_switch_operator_with_same_commission(
            operator_1_address,
            operator_2_address,
        )
    ));
    // New operator leaves validator set.
    leave_validator_set(&mut harness, &operator_2, pool_address);
}

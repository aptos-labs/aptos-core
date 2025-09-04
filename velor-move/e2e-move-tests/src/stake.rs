// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::harness::MoveHarness;
use velor_cached_packages::velor_stdlib;
use velor_crypto::{bls12381, PrivateKey, Uniform};
use velor_language_e2e_tests::account::Account;
use velor_types::{
    account_address::AccountAddress, account_config::CORE_CODE_ADDRESS,
    on_chain_config::ValidatorSet, stake_pool::StakePool, transaction::TransactionStatus,
    validator_config::ValidatorConfig,
};
use move_core_types::parser::parse_struct_tag;

pub fn setup_staking(
    harness: &mut MoveHarness,
    account: &Account,
    initial_stake_amount: u64,
) -> TransactionStatus {
    let address = *account.address();
    initialize_staking(harness, account, initial_stake_amount, address, address);
    rotate_consensus_key(harness, account, address);
    join_validator_set(harness, account, address)
}

pub fn initialize_staking(
    harness: &mut MoveHarness,
    account: &Account,
    initial_stake_amount: u64,
    operator_address: AccountAddress,
    voter_address: AccountAddress,
) -> TransactionStatus {
    harness.run_transaction_payload(
        account,
        velor_stdlib::stake_initialize_stake_owner(
            initial_stake_amount,
            operator_address,
            voter_address,
        ),
    )
}

pub fn add_stake(harness: &mut MoveHarness, account: &Account, amount: u64) -> TransactionStatus {
    harness.run_transaction_payload(account, velor_stdlib::stake_add_stake(amount))
}

pub fn unlock_stake(
    harness: &mut MoveHarness,
    account: &Account,
    amount: u64,
) -> TransactionStatus {
    harness.run_transaction_payload(account, velor_stdlib::stake_unlock(amount))
}

pub fn withdraw_stake(
    harness: &mut MoveHarness,
    account: &Account,
    amount: u64,
) -> TransactionStatus {
    harness.run_transaction_payload(account, velor_stdlib::stake_withdraw(amount))
}

pub fn join_validator_set(
    harness: &mut MoveHarness,
    account: &Account,
    pool_address: AccountAddress,
) -> TransactionStatus {
    harness.run_transaction_payload(
        account,
        velor_stdlib::stake_join_validator_set(pool_address),
    )
}

pub fn rotate_consensus_key(
    harness: &mut MoveHarness,
    account: &Account,
    pool_address: AccountAddress,
) -> TransactionStatus {
    let consensus_key = bls12381::PrivateKey::generate_for_testing();
    let consensus_pubkey = consensus_key.public_key().to_bytes().to_vec();
    let proof_of_possession = bls12381::ProofOfPossession::create(&consensus_key)
        .to_bytes()
        .to_vec();
    harness.run_transaction_payload(
        account,
        velor_stdlib::stake_rotate_consensus_key(
            pool_address,
            consensus_pubkey,
            proof_of_possession,
        ),
    )
}

pub fn leave_validator_set(
    harness: &mut MoveHarness,
    account: &Account,
    pool_address: AccountAddress,
) -> TransactionStatus {
    harness.run_transaction_payload(
        account,
        velor_stdlib::stake_leave_validator_set(pool_address),
    )
}

pub fn increase_lockup(harness: &mut MoveHarness, account: &Account) -> TransactionStatus {
    harness.run_transaction_payload(account, velor_stdlib::stake_increase_lockup())
}

pub fn get_stake_pool(harness: &MoveHarness, pool_address: &AccountAddress) -> StakePool {
    harness
        .read_resource::<StakePool>(
            pool_address,
            parse_struct_tag("0x1::stake::StakePool").unwrap(),
        )
        .unwrap()
}

pub fn get_validator_config(
    harness: &MoveHarness,
    pool_address: &AccountAddress,
) -> ValidatorConfig {
    harness
        .read_resource::<ValidatorConfig>(
            pool_address,
            parse_struct_tag("0x1::stake::ValidatorConfig").unwrap(),
        )
        .unwrap()
}

pub fn get_validator_set(harness: &MoveHarness) -> ValidatorSet {
    harness
        .read_resource::<ValidatorSet>(
            &CORE_CODE_ADDRESS,
            parse_struct_tag("0x1::stake::ValidatorSet").unwrap(),
        )
        .unwrap()
}

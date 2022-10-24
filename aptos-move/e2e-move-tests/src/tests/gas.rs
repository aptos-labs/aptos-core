// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::tests::common::test_dir_path;
use crate::MoveHarness;
use aptos_crypto::{bls12381, PrivateKey, Uniform};
use aptos_types::account_address::{default_stake_pool_address, AccountAddress};
use cached_packages::{aptos_stdlib, aptos_token_sdk_builder};

/// Run with `cargo test test_gas -- --nocapture` to see output.
#[test]
fn test_gas() {
    // Start with 100 validators.
    let mut harness = MoveHarness::new_with_validators(100);
    let account_1 = &harness.new_account_at(AccountAddress::from_hex_literal("0x121").unwrap());
    let account_2 = &harness.new_account_at(AccountAddress::from_hex_literal("0x122").unwrap());
    let account_3 = &harness.new_account_at(AccountAddress::from_hex_literal("0x123").unwrap());
    let account_1_address = *account_1.address();
    let account_2_address = *account_2.address();
    let account_3_address = *account_3.address();

    print_gas_cost(
        "Transfer",
        harness.evaluate_gas(
            account_1,
            aptos_stdlib::aptos_coin_transfer(account_2_address, 1000),
        ),
    );
    print_gas_cost(
        "CreateAccount",
        harness.evaluate_gas(
            account_1,
            aptos_stdlib::aptos_account_create_account(
                AccountAddress::from_hex_literal("0xcafe1").unwrap(),
            ),
        ),
    );
    print_gas_cost(
        "CreateTransfer",
        harness.evaluate_gas(
            account_1,
            aptos_stdlib::aptos_account_transfer(
                AccountAddress::from_hex_literal("0xcafe2").unwrap(),
                1000,
            ),
        ),
    );
    print_gas_cost(
        "CreateStakePool",
        harness.evaluate_gas(
            account_1,
            aptos_stdlib::staking_contract_create_staking_contract(
                account_2_address,
                account_3_address,
                25_000_000,
                10,
                vec![],
            ),
        ),
    );
    let pool_address = default_stake_pool_address(account_1_address, account_2_address);
    let consensus_key = bls12381::PrivateKey::generate_for_testing();
    let consensus_pubkey = consensus_key.public_key().to_bytes().to_vec();
    let proof_of_possession = bls12381::ProofOfPossession::create(&consensus_key)
        .to_bytes()
        .to_vec();
    print_gas_cost(
        "RotateConsensusKey",
        harness.evaluate_gas(
            account_2,
            aptos_stdlib::stake_rotate_consensus_key(
                pool_address,
                consensus_pubkey,
                proof_of_possession,
            ),
        ),
    );
    print_gas_cost(
        "JoinValidator100",
        harness.evaluate_gas(
            account_2,
            aptos_stdlib::stake_join_validator_set(pool_address),
        ),
    );
    print_gas_cost(
        "AddStake",
        harness.evaluate_gas(
            account_1,
            aptos_stdlib::staking_contract_add_stake(account_2_address, 1000),
        ),
    );
    print_gas_cost(
        "UnlockStake",
        harness.evaluate_gas(
            account_1,
            aptos_stdlib::staking_contract_unlock_stake(account_2_address, 1000),
        ),
    );
    harness.fast_forward(7200);
    harness.new_epoch();
    print_gas_cost(
        "WithdrawStake",
        harness.evaluate_gas(
            account_1,
            aptos_stdlib::staking_contract_distribute(account_1_address, account_2_address),
        ),
    );
    print_gas_cost(
        "LeaveValidatorSet100",
        harness.evaluate_gas(
            account_2,
            aptos_stdlib::stake_leave_validator_set(pool_address),
        ),
    );
    let collection_name = "collection name".to_owned().into_bytes();
    let token_name = "token name".to_owned().into_bytes();
    print_gas_cost(
        "CreateCollection",
        harness.evaluate_gas(
            account_1,
            aptos_token_sdk_builder::token_create_collection_script(
                collection_name.clone(),
                "description".to_owned().into_bytes(),
                "uri".to_owned().into_bytes(),
                20_000_000,
                vec![false, false, false],
            ),
        ),
    );
    print_gas_cost(
        "CreateTokenFirstTime",
        harness.evaluate_gas(
            account_1,
            aptos_token_sdk_builder::token_create_token_script(
                collection_name.clone(),
                token_name.clone(),
                "collection description".to_owned().into_bytes(),
                1,
                4,
                "uri".to_owned().into_bytes(),
                account_1_address,
                1,
                0,
                vec![false, false, false, false, true],
                vec!["age".as_bytes().to_vec()],
                vec!["3".as_bytes().to_vec()],
                vec!["int".as_bytes().to_vec()],
            ),
        ),
    );
    print_gas_cost(
        "MintToken",
        harness.evaluate_gas(
            account_1,
            aptos_token_sdk_builder::token_mint_script(
                account_1_address,
                collection_name.clone(),
                token_name.clone(),
                1,
            ),
        ),
    );
    print_gas_cost(
        "MutateToken",
        harness.evaluate_gas(
            account_1,
            aptos_token_sdk_builder::token_mutate_token_properties(
                account_1_address,
                account_1_address,
                collection_name,
                token_name,
                0,
                1,
                vec![],
                vec![],
                vec![],
            ),
        ),
    );
    let publisher = &harness.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    print_gas_cost(
        "PublishSmall",
        harness.evaluate_publish_gas(
            publisher,
            &test_dir_path("code_publishing.data/pack_initial"),
        ),
    );
    print_gas_cost(
        "UpgradeSmall",
        harness.evaluate_publish_gas(
            publisher,
            &test_dir_path("code_publishing.data/pack_upgrade_compat"),
        ),
    );
    let publisher = &harness.aptos_framework_account();
    print_gas_cost(
        "PublishLarge",
        harness.evaluate_publish_gas(
            publisher,
            &test_dir_path("code_publishing.data/pack_stdlib"),
        ),
    );
}

fn dollar_cost(gas_units: u64, price: u64) -> f64 {
    (gas_units as f64) / 100_000_000_f64 * (price as f64)
}

fn print_gas_cost(function: &str, gas_units: u64) {
    let gas_units = gas_units * 100;
    println!(
        "{:20} | {:8} | {:.3} | {:.3} | {:.3}",
        function,
        gas_units,
        dollar_cost(gas_units, 5),
        dollar_cost(gas_units, 15),
        dollar_cost(gas_units, 30)
    );
}

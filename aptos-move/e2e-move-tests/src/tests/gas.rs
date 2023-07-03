// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{tests::common::test_dir_path, MoveHarness};
use aptos_cached_packages::{aptos_stdlib, aptos_token_sdk_builder};
use aptos_crypto::{bls12381, PrivateKey, Uniform};
use aptos_gas_profiling::TransactionGasLog;
use aptos_types::account_address::{default_stake_pool_address, AccountAddress};
use aptos_vm::AptosVM;
use std::{fmt::Write, fs, path::Path};

fn save_profiling_results(name: &str, log: &TransactionGasLog) {
    let path = Path::new("gas-profiling").join(name);

    if let Err(err) = fs::create_dir_all(&path) {
        match err.kind() {
            std::io::ErrorKind::AlreadyExists => (),
            _ => panic!("failed to create directory {}: {}", path.display(), err),
        }
    }

    if let Some(graph_bytes) = log.exec_io.to_flamegraph(name.to_string()).unwrap() {
        fs::write(path.join("exec_io.svg"), graph_bytes).unwrap();
    }
    if let Some(graph_bytes) = log.storage.to_flamegraph(name.to_string()).unwrap() {
        fs::write(path.join("storage.svg"), graph_bytes).unwrap();
    }

    let mut text = String::new();
    let erased = log.to_erased();

    erased.exec_io.textualize(&mut text, true).unwrap();
    writeln!(text).unwrap();
    writeln!(text).unwrap();
    log.exec_io
        .aggregate_gas_events()
        .textualize(&mut text)
        .unwrap();
    writeln!(text).unwrap();
    writeln!(text).unwrap();

    erased.storage.textualize(&mut text, true).unwrap();

    fs::write(path.join("log.txt"), text).unwrap();
}

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

    // Use the gas profiler unless explicitly disabled by the user.
    //
    // This is to give us some basic code coverage on the gas profile.
    let profile_gas = match std::env::var("PROFILE_GAS") {
        Ok(s) => {
            let s = s.to_lowercase();
            s != "0" && s != "false" && s != "no"
        },
        Err(_) => true,
    };

    let run = |harness: &mut MoveHarness, function, account, payload| {
        if !profile_gas {
            print_gas_cost(function, harness.evaluate_gas(account, payload));
        } else {
            let (log, gas_used) = harness.evaluate_gas_with_profiler(account, payload);
            save_profiling_results(function, &log);
            print_gas_cost(function, gas_used);
        }
    };

    let publish = |harness: &mut MoveHarness, name, account, path: &Path| {
        if !profile_gas {
            print_gas_cost(name, harness.evaluate_publish_gas(account, path));
        } else {
            let (log, gas_used) = harness.evaluate_publish_gas_with_profiler(account, path);
            save_profiling_results(name, &log);
            print_gas_cost(name, gas_used);
        }
    };

    AptosVM::set_paranoid_type_checks(true);

    run(
        &mut harness,
        "Transfer",
        account_1,
        aptos_stdlib::aptos_coin_transfer(account_2_address, 1000),
    );

    run(
        &mut harness,
        "CreateAccount",
        account_1,
        aptos_stdlib::aptos_account_create_account(
            AccountAddress::from_hex_literal("0xcafe1").unwrap(),
        ),
    );
    run(
        &mut harness,
        "CreateTransfer",
        account_1,
        aptos_stdlib::aptos_account_transfer(
            AccountAddress::from_hex_literal("0xcafe2").unwrap(),
            1000,
        ),
    );
    run(
        &mut harness,
        "CreateStakePool",
        account_1,
        aptos_stdlib::staking_contract_create_staking_contract(
            account_2_address,
            account_3_address,
            25_000_000,
            10,
            vec![],
        ),
    );
    let pool_address = default_stake_pool_address(account_1_address, account_2_address);
    let consensus_key = bls12381::PrivateKey::generate_for_testing();
    let consensus_pubkey = consensus_key.public_key().to_bytes().to_vec();
    let proof_of_possession = bls12381::ProofOfPossession::create(&consensus_key)
        .to_bytes()
        .to_vec();
    run(
        &mut harness,
        "RotateConsensusKey",
        account_2,
        aptos_stdlib::stake_rotate_consensus_key(
            pool_address,
            consensus_pubkey,
            proof_of_possession,
        ),
    );
    run(
        &mut harness,
        "JoinValidator100",
        account_2,
        aptos_stdlib::stake_join_validator_set(pool_address),
    );
    run(
        &mut harness,
        "AddStake",
        account_1,
        aptos_stdlib::staking_contract_add_stake(account_2_address, 1000),
    );
    run(
        &mut harness,
        "UnlockStake",
        account_1,
        aptos_stdlib::staking_contract_unlock_stake(account_2_address, 1000),
    );
    harness.fast_forward(7200);
    harness.new_epoch();
    run(
        &mut harness,
        "WithdrawStake",
        account_1,
        aptos_stdlib::staking_contract_distribute(account_1_address, account_2_address),
    );
    run(
        &mut harness,
        "LeaveValidatorSet100",
        account_2,
        aptos_stdlib::stake_leave_validator_set(pool_address),
    );
    let collection_name = "collection name".to_owned().into_bytes();
    let token_name = "token name".to_owned().into_bytes();
    run(
        &mut harness,
        "CreateCollection",
        account_1,
        aptos_token_sdk_builder::token_create_collection_script(
            collection_name.clone(),
            "description".to_owned().into_bytes(),
            "uri".to_owned().into_bytes(),
            20_000_000,
            vec![false, false, false],
        ),
    );
    run(
        &mut harness,
        "CreateTokenFirstTime",
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
    );
    run(
        &mut harness,
        "MintToken",
        account_1,
        aptos_token_sdk_builder::token_mint_script(
            account_1_address,
            collection_name.clone(),
            token_name.clone(),
            1,
        ),
    );
    run(
        &mut harness,
        "MutateToken",
        account_1,
        aptos_token_sdk_builder::token_mutate_token_properties(
            account_1_address,
            account_1_address,
            collection_name.clone(),
            token_name.clone(),
            0,
            1,
            vec!["age".as_bytes().to_vec()],
            vec!["4".as_bytes().to_vec()],
            vec!["int".as_bytes().to_vec()],
        ),
    );
    run(
        &mut harness,
        "MutateToken2ndTime",
        account_1,
        aptos_token_sdk_builder::token_mutate_token_properties(
            account_1_address,
            account_1_address,
            collection_name.clone(),
            token_name.clone(),
            1,
            1,
            vec!["age".as_bytes().to_vec()],
            vec!["5".as_bytes().to_vec()],
            vec!["int".as_bytes().to_vec()],
        ),
    );

    let mut keys = vec![];
    let mut vals = vec![];
    let mut typs = vec![];
    for i in 0..10 {
        keys.push(format!("attr_{}", i).as_bytes().to_vec());
        vals.push(format!("{}", i).as_bytes().to_vec());
        typs.push("u64".as_bytes().to_vec());
    }
    run(
        &mut harness,
        "MutateTokenAdd10NewProperties",
        account_1,
        aptos_token_sdk_builder::token_mutate_token_properties(
            account_1_address,
            account_1_address,
            collection_name.clone(),
            token_name.clone(),
            1,
            1,
            keys.clone(),
            vals.clone(),
            typs.clone(),
        ),
    );
    run(
        &mut harness,
        "MutateTokenMutate10ExistingProperties",
        account_1,
        aptos_token_sdk_builder::token_mutate_token_properties(
            account_1_address,
            account_1_address,
            collection_name,
            token_name,
            1,
            1,
            keys,
            vals,
            typs,
        ),
    );

    let publisher = &harness.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    publish(
        &mut harness,
        "PublishSmall",
        publisher,
        &test_dir_path("code_publishing.data/pack_initial"),
    );
    publish(
        &mut harness,
        "UpgradeSmall",
        publisher,
        &test_dir_path("code_publishing.data/pack_upgrade_compat"),
    );
    let publisher = &harness.aptos_framework_account();
    publish(
        &mut harness,
        "PublishLarge",
        publisher,
        &test_dir_path("code_publishing.data/pack_stdlib"),
    );
}

fn dollar_cost(gas_units: u64, price: u64) -> f64 {
    ((gas_units * 100/* gas unit price */) as f64) / 100_000_000_f64 * (price as f64)
}

pub fn print_gas_cost(function: &str, gas_units: u64) {
    println!(
        "{:20} | {:8} | {:.6} | {:.6} | {:.6}",
        function,
        gas_units,
        dollar_cost(gas_units, 5),
        dollar_cost(gas_units, 15),
        dollar_cost(gas_units, 30)
    );
}

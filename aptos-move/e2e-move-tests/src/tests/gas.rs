// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    tests::{
        common::test_dir_path,
        token_objects::{
            create_mint_hero_payload, create_set_hero_description_payload,
            publish_object_token_example,
        },
    },
    MoveHarness,
};
use aptos_cached_packages::{aptos_stdlib, aptos_token_sdk_builder};
use aptos_crypto::{bls12381, PrivateKey, Uniform};
use aptos_gas_profiling::TransactionGasLog;
use aptos_types::{
    account_address::{default_stake_pool_address, AccountAddress},
    account_config::CORE_CODE_ADDRESS,
    transaction::{EntryFunction, TransactionPayload},
    vm::configs::set_paranoid_type_checks,
};
use move_core_types::{identifier::Identifier, language_storage::ModuleId};
use sha3::{Digest, Sha3_512};
use std::path::Path;

#[test]
fn test_modify_gas_schedule_check_hash() {
    let mut harness = MoveHarness::new();

    let mut gas_schedule = harness.get_gas_schedule();
    let old_hash = Sha3_512::digest(&bcs::to_bytes(&gas_schedule).unwrap()).to_vec();

    const MAGIC: u64 = 42424242;

    let (_, val) = gas_schedule
        .entries
        .iter_mut()
        .find(|(name, _)| name == "instr.nop")
        .unwrap();
    assert_ne!(*val, MAGIC);
    *val = MAGIC;

    harness.executor.exec(
        "gas_schedule",
        "set_for_next_epoch_check_hash",
        vec![],
        vec![
            bcs::to_bytes(&CORE_CODE_ADDRESS).unwrap(),
            bcs::to_bytes(&old_hash).unwrap(),
            bcs::to_bytes(&bcs::to_bytes(&gas_schedule).unwrap()).unwrap(),
        ],
    );

    harness
        .executor
        .exec("reconfiguration_with_dkg", "finish", vec![], vec![
            bcs::to_bytes(&CORE_CODE_ADDRESS).unwrap(),
        ]);

    let (_, gas_params) = harness.get_gas_params();
    assert_eq!(gas_params.vm.instr.nop, MAGIC.into());
}

fn save_profiling_results(name: &str, log: &TransactionGasLog) {
    let path = Path::new("gas-profiling").join(name);
    log.generate_html_report(path, format!("Gas Report - {}", name))
        .unwrap();
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

    set_paranoid_type_checks(true);

    run(
        &mut harness,
        "Transfer",
        account_1,
        aptos_stdlib::aptos_coin_transfer(account_2_address, 1000),
    );

    run(
        &mut harness,
        "2ndTransfer",
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

    publish_object_token_example(&mut harness, account_1_address, account_1);
    run(
        &mut harness,
        "MintTokenV2",
        account_1,
        create_mint_hero_payload(&account_1_address, SHORT_STR),
    );
    run(
        &mut harness,
        "MutateTokenV2",
        account_1,
        create_set_hero_description_payload(&account_1_address, SHORT_STR),
    );
    publish_object_token_example(&mut harness, account_2_address, account_2);
    run(
        &mut harness,
        "MintLargeTokenV2",
        account_2,
        create_mint_hero_payload(&account_2_address, LONG_STR),
    );
    run(
        &mut harness,
        "MutateLargeTokenV2",
        account_2,
        create_set_hero_description_payload(&account_2_address, LONG_STR),
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
        "MintTokenV1",
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
        "MutateTokenV1",
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
        "MutateTokenV12ndTime",
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
    publish(
        &mut harness,
        "PublishLarge",
        publisher,
        &test_dir_path("code_publishing.data/pack_large"),
    );
    publish(
        &mut harness,
        "UpgradeLarge",
        publisher,
        &test_dir_path("code_publishing.data/pack_large_upgrade"),
    );
    publish(
        &mut harness,
        "PublishDependencyChain-1",
        publisher,
        &test_dir_path("dependencies.data/p1"),
    );
    publish(
        &mut harness,
        "PublishDependencyChain-2",
        publisher,
        &test_dir_path("dependencies.data/p2"),
    );
    publish(
        &mut harness,
        "PublishDependencyChain-3",
        publisher,
        &test_dir_path("dependencies.data/p3"),
    );
    run(
        &mut harness,
        "UseDependencyChain-1",
        publisher,
        TransactionPayload::EntryFunction(EntryFunction::new(
            ModuleId::new(
                AccountAddress::from_hex_literal("0xcafe").unwrap(),
                Identifier::new("m1").unwrap(),
            ),
            Identifier::new("run").unwrap(),
            vec![],
            vec![],
        )),
    );
    run(
        &mut harness,
        "UseDependencyChain-2",
        publisher,
        TransactionPayload::EntryFunction(EntryFunction::new(
            ModuleId::new(
                AccountAddress::from_hex_literal("0xcafe").unwrap(),
                Identifier::new("m2").unwrap(),
            ),
            Identifier::new("run").unwrap(),
            vec![],
            vec![],
        )),
    );
    run(
        &mut harness,
        "UseDependencyChain-3",
        publisher,
        TransactionPayload::EntryFunction(EntryFunction::new(
            ModuleId::new(
                AccountAddress::from_hex_literal("0xcafe").unwrap(),
                Identifier::new("m3").unwrap(),
            ),
            Identifier::new("run").unwrap(),
            vec![],
            vec![],
        )),
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

const SHORT_STR: &str = "A hero.";
const LONG_STR: &str = "\
    0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    ";

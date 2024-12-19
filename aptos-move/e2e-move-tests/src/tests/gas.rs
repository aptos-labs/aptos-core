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
use aptos_gas_algebra::GasQuantity;
use aptos_gas_profiling::TransactionGasLog;
use aptos_language_e2e_tests::account::Account;
use aptos_transaction_generator_lib::{
    publishing::{
        module_simple::{LoopType, MultiSigConfig},
        publish_util::PackageHandler,
    },
    EntryPoints,
};
use aptos_types::{
    account_address::{default_stake_pool_address, AccountAddress},
    account_config::CORE_CODE_ADDRESS,
    fee_statement::FeeStatement,
    transaction::{EntryFunction, TransactionPayload},
};
use aptos_vm_environment::prod_configs::set_paranoid_type_checks;
use move_core_types::{identifier::Identifier, language_storage::ModuleId, value::MoveValue};
use rand::{rngs::StdRng, SeedableRng};
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
            MoveValue::Signer(CORE_CODE_ADDRESS)
                .simple_serialize()
                .unwrap(),
            bcs::to_bytes(&old_hash).unwrap(),
            bcs::to_bytes(&bcs::to_bytes(&gas_schedule).unwrap()).unwrap(),
        ],
    );

    harness
        .executor
        .exec("reconfiguration_with_dkg", "finish", vec![], vec![
            MoveValue::Signer(CORE_CODE_ADDRESS)
                .simple_serialize()
                .unwrap(),
        ]);

    let (_, gas_params) = harness.get_gas_params();
    assert_eq!(gas_params.vm.instr.nop, MAGIC.into());
}

fn save_profiling_results(name: &str, log: &TransactionGasLog) {
    let path = Path::new("gas-profiling").join(name);
    log.generate_html_report(path, format!("Gas Report - {}", name))
        .unwrap();
}

pub struct SummaryExeAndIO {
    pub intrinsic_cost: f64,
    pub execution_cost: f64,
    pub read_cost: f64,
    pub write_cost: f64,
}

fn summarize_exe_and_io(log: TransactionGasLog) -> SummaryExeAndIO {
    fn cast<T>(gas: GasQuantity<T>) -> f64 {
        u64::from(gas) as f64
    }

    let scale = cast(log.exec_io.gas_scaling_factor);

    let aggregated = log.exec_io.aggregate_gas_events();

    let execution = aggregated.ops.iter().map(|(_, _, v)| cast(*v)).sum::<f64>();
    let read = aggregated
        .storage_reads
        .iter()
        .map(|(_, _, v)| cast(*v))
        .sum::<f64>();
    let write = aggregated
        .storage_writes
        .iter()
        .map(|(_, _, v)| cast(*v))
        .sum::<f64>();
    SummaryExeAndIO {
        intrinsic_cost: cast(log.exec_io.intrinsic_cost) / scale,
        execution_cost: execution / scale,
        read_cost: read / scale,
        write_cost: write / scale,
    }
}

struct Runner {
    pub harness: MoveHarness,
    profile_gas: bool,
}

impl Runner {
    pub fn run(&mut self, function: &str, account: &Account, payload: TransactionPayload) {
        if !self.profile_gas {
            print_gas_cost(function, self.harness.evaluate_gas(account, payload));
        } else {
            let (log, gas_used, fee_statement) =
                self.harness.evaluate_gas_with_profiler(account, payload);
            save_profiling_results(function, &log);
            print_gas_cost_with_statement(function, gas_used, fee_statement);
        }
    }

    pub fn run_with_tps_estimate(
        &mut self,
        function: &str,
        account: &Account,
        payload: TransactionPayload,
        tps: f64,
    ) {
        if !self.profile_gas {
            print_gas_cost(function, self.harness.evaluate_gas(account, payload));
        } else {
            let (log, gas_used, fee_statement) =
                self.harness.evaluate_gas_with_profiler(account, payload);
            save_profiling_results(function, &log);
            print_gas_cost_with_statement_and_tps(
                function,
                gas_used,
                fee_statement,
                summarize_exe_and_io(log),
                tps,
            );
        }
    }

    pub fn publish(&mut self, name: &str, account: &Account, path: &Path) {
        if !self.profile_gas {
            print_gas_cost(name, self.harness.evaluate_publish_gas(account, path));
        } else {
            let (log, gas_used, fee_statement) = self
                .harness
                .evaluate_publish_gas_with_profiler(account, path);
            save_profiling_results(name, &log);
            print_gas_cost_with_statement(name, gas_used, fee_statement);
        }
    }
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

    let mut runner = Runner {
        harness,
        profile_gas,
    };

    set_paranoid_type_checks(true);

    runner.run(
        "Transfer",
        account_1,
        aptos_stdlib::aptos_coin_transfer(account_2_address, 1000),
    );

    runner.run(
        "2ndTransfer",
        account_1,
        aptos_stdlib::aptos_coin_transfer(account_2_address, 1000),
    );

    runner.run(
        "CreateAccount",
        account_1,
        aptos_stdlib::aptos_account_create_account(
            AccountAddress::from_hex_literal("0xcafe1").unwrap(),
        ),
    );

    runner.run(
        "CreateTransfer",
        account_1,
        aptos_stdlib::aptos_account_transfer(
            AccountAddress::from_hex_literal("0xcafe2").unwrap(),
            1000,
        ),
    );

    publish_object_token_example(&mut runner.harness, account_1_address, account_1);
    runner.run(
        "MintTokenV2",
        account_1,
        create_mint_hero_payload(&account_1_address, SHORT_STR),
    );
    runner.run(
        "MutateTokenV2",
        account_1,
        create_set_hero_description_payload(&account_1_address, SHORT_STR),
    );
    publish_object_token_example(&mut runner.harness, account_2_address, account_2);
    runner.run(
        "MintLargeTokenV2",
        account_2,
        create_mint_hero_payload(&account_2_address, LONG_STR),
    );
    runner.run(
        "MutateLargeTokenV2",
        account_2,
        create_set_hero_description_payload(&account_2_address, LONG_STR),
    );

    runner.run(
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
    runner.run(
        "RotateConsensusKey",
        account_2,
        aptos_stdlib::stake_rotate_consensus_key(
            pool_address,
            consensus_pubkey,
            proof_of_possession,
        ),
    );
    runner.run(
        "JoinValidator100",
        account_2,
        aptos_stdlib::stake_join_validator_set(pool_address),
    );
    runner.run(
        "AddStake",
        account_1,
        aptos_stdlib::staking_contract_add_stake(account_2_address, 1000),
    );
    runner.run(
        "UnlockStake",
        account_1,
        aptos_stdlib::staking_contract_unlock_stake(account_2_address, 1000),
    );
    runner.harness.fast_forward(7200);
    runner.harness.new_epoch();
    runner.run(
        "WithdrawStake",
        account_1,
        aptos_stdlib::staking_contract_distribute(account_1_address, account_2_address),
    );
    runner.run(
        "LeaveValidatorSet100",
        account_2,
        aptos_stdlib::stake_leave_validator_set(pool_address),
    );
    let collection_name = "collection name".to_owned().into_bytes();
    let token_name = "token name".to_owned().into_bytes();
    runner.run(
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
    runner.run(
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
    runner.run(
        "MintTokenV1",
        account_1,
        aptos_token_sdk_builder::token_mint_script(
            account_1_address,
            collection_name.clone(),
            token_name.clone(),
            1,
        ),
    );
    runner.run(
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
    runner.run(
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
    runner.run(
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
    runner.run(
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

    let publisher = &runner
        .harness
        .new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    runner.publish(
        "PublishSmall",
        publisher,
        &test_dir_path("code_publishing.data/pack_initial"),
    );
    runner.publish(
        "UpgradeSmall",
        publisher,
        &test_dir_path("code_publishing.data/pack_upgrade_compat"),
    );
    let publisher = &runner.harness.aptos_framework_account();
    runner.publish(
        "PublishLarge",
        publisher,
        &test_dir_path("code_publishing.data/pack_large"),
    );
    runner.publish(
        "UpgradeLarge",
        publisher,
        &test_dir_path("code_publishing.data/pack_large_upgrade"),
    );
    runner.publish(
        "PublishDependencyChain-1",
        publisher,
        &test_dir_path("dependencies.data/p1"),
    );
    runner.publish(
        "PublishDependencyChain-2",
        publisher,
        &test_dir_path("dependencies.data/p2"),
    );
    runner.publish(
        "PublishDependencyChain-3",
        publisher,
        &test_dir_path("dependencies.data/p3"),
    );
    runner.run(
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
    runner.run(
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
    runner.run(
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
        "{:8} | {:.6} | {:.6} | {:.6} | {}",
        gas_units,
        dollar_cost(gas_units, 5),
        dollar_cost(gas_units, 15),
        dollar_cost(gas_units, 30),
        function,
    );
}

pub fn print_gas_cost_with_statement(
    function: &str,
    gas_units: u64,
    fee_statement: Option<FeeStatement>,
) {
    println!(
        "{:8} | {:.6} | {:.6} | {:.6} | {:8} | {:8} | {:8} | {}",
        gas_units,
        dollar_cost(gas_units, 5),
        dollar_cost(gas_units, 15),
        dollar_cost(gas_units, 30),
        fee_statement.unwrap().execution_gas_used() + fee_statement.unwrap().io_gas_used(),
        fee_statement.unwrap().execution_gas_used(),
        fee_statement.unwrap().io_gas_used(),
        function,
    );
}

pub fn print_gas_cost_with_statement_and_tps_header() {
    println!(
        "{:9} | {:9.6} | {:9.6} | {:9.6} | {:8} | {:8} | {:8} | {:8} | {:8} | {:8} | {:10}",
        "gas units",
        "$ at 5",
        "$ at 15",
        "$ at 30",
        "exe+io g",
        // "exe gas",
        // "io gas",
        "intrins",
        "execut",
        "read",
        "write",
        "gas / s",
        "function",
    );
}

pub fn print_gas_cost_with_statement_and_tps(
    function: &str,
    gas_units: u64,
    fee_statement: Option<FeeStatement>,
    summary: SummaryExeAndIO,
    tps: f64,
) {
    println!(
        "{:9} | {:9.6} | {:9.6} | {:9.6} | {:8} | {:8.2} | {:8.2} | {:8.2} | {:8.2} | {:8.0} | {}",
        gas_units,
        dollar_cost(gas_units, 5),
        dollar_cost(gas_units, 15),
        dollar_cost(gas_units, 30),
        fee_statement.unwrap().execution_gas_used() + fee_statement.unwrap().io_gas_used(),
        // fee_statement.unwrap().execution_gas_used(),
        // fee_statement.unwrap().io_gas_used(),
        summary.intrinsic_cost,
        summary.execution_cost,
        summary.read_cost,
        summary.write_cost,
        (fee_statement.unwrap().execution_gas_used() + fee_statement.unwrap().io_gas_used()) as f64
            * tps,
        function,
    );
}

#[test]
#[ignore]
fn test_txn_generator_workloads_calibrate_gas() {
    // Start with 100 validators.
    let mut harness = MoveHarness::new_with_validators(100);
    let account_1 = &harness.new_account_at(AccountAddress::from_hex_literal("0x121").unwrap());
    let account_2 = &harness.new_account_at(AccountAddress::from_hex_literal("0x122").unwrap());
    let account_2_address = *account_2.address();

    // Use the gas profiler unless explicitly disabled by the user.
    //
    // This is to give us some basic code coverage on the gas profile.
    let profile_gas = match std::env::var("PROFILE_GAS") {
        Ok(s) => {
            let s = s.to_lowercase();
            s == "1" && s == "true" && s == "yes"
        },
        Err(_) => true,
    };

    let mut runner = Runner {
        harness,
        profile_gas,
    };

    set_paranoid_type_checks(true);

    print_gas_cost_with_statement_and_tps_header();

    let use_large_db_numbers = true;

    // Constants here are produced from running
    //   NUMBER_OF_EXECUTION_THREADS=1 testsuite/single_node_performance.py
    // on a prod-spec'd machine.
    let entry_points = vec![
        (2963., 4103., EntryPoints::Nop),
        (2426., 3411., EntryPoints::BytesMakeOrChange {
            data_length: Some(32),
        }),
        (2388., 3270., EntryPoints::IncGlobal),
        (27., 28., EntryPoints::Loop {
            loop_count: Some(100000),
            loop_type: LoopType::NoOp,
        }),
        (44., 42., EntryPoints::Loop {
            loop_count: Some(10000),
            loop_type: LoopType::Arithmetic,
        }),
        // This is a cheap bcs (serializing vec<u8>), so not representative of what BCS native call should cost.
        // (175., EntryPoints::Loop { loop_count: Some(1000), loop_type: LoopType::BCS { len: 1024 }}),
        (666., 1031., EntryPoints::CreateObjects {
            num_objects: 10,
            object_payload_size: 0,
        }),
        (103., 108., EntryPoints::CreateObjects {
            num_objects: 10,
            object_payload_size: 10 * 1024,
        }),
        (93., 148., EntryPoints::CreateObjects {
            num_objects: 100,
            object_payload_size: 0,
        }),
        (43., 50., EntryPoints::CreateObjects {
            num_objects: 100,
            object_payload_size: 10 * 1024,
        }),
        (1605., 2100., EntryPoints::InitializeVectorPicture {
            length: 40,
        }),
        (2850., 3400., EntryPoints::VectorPicture { length: 40 }),
        (2900., 3480., EntryPoints::VectorPictureRead { length: 40 }),
        (30., 31., EntryPoints::InitializeVectorPicture {
            length: 30 * 1024,
        }),
        (169., 180., EntryPoints::VectorPicture { length: 30 * 1024 }),
        (189., 200., EntryPoints::VectorPictureRead {
            length: 30 * 1024,
        }),
        (22., 17.8, EntryPoints::SmartTablePicture {
            length: 30 * 1024,
            num_points_per_txn: 200,
        }),
        (3., 2.75, EntryPoints::SmartTablePicture {
            length: 1024 * 1024,
            num_points_per_txn: 1024,
        }),
        (1351., 1719., EntryPoints::TokenV1MintAndTransferFT),
        (
            971.,
            1150.,
            EntryPoints::TokenV1MintAndTransferNFTSequential,
        ),
        (1077., 1274., EntryPoints::TokenV2AmbassadorMint {
            numbered: true,
        }),
    ];

    for (large_db_tps, small_db_tps, entry_point) in &entry_points {
        if let MultiSigConfig::None = entry_point.multi_sig_additional_num() {
            let publisher = runner.harness.new_account_with_key_pair();
            let user = runner.harness.new_account_with_key_pair();

            let mut package_handler = PackageHandler::new(entry_point.package_name());
            let mut rng = StdRng::seed_from_u64(14);
            let package = package_handler.pick_package(&mut rng, *publisher.address());
            runner
                .harness
                .run_transaction_payload(&publisher, package.publish_transaction_payload());
            if let Some(init_entry_point) = entry_point.initialize_entry_point() {
                runner.harness.run_transaction_payload(
                    &publisher,
                    init_entry_point.create_payload(
                        &package,
                        init_entry_point.module_name(),
                        Some(&mut rng),
                        Some(publisher.address()),
                    ),
                );
            }

            runner.run_with_tps_estimate(
                &format!("entry_point_{entry_point:?}"),
                &user,
                entry_point.create_payload(
                    &package,
                    entry_point.module_name(),
                    Some(&mut rng),
                    Some(publisher.address()),
                ),
                if use_large_db_numbers {
                    *large_db_tps
                } else {
                    *small_db_tps
                },
            );
        } else {
            println!("Skipping multisig {entry_point:?}");
        }
    }

    runner.run_with_tps_estimate(
        "Transfer",
        account_1,
        aptos_stdlib::aptos_coin_transfer(account_2_address, 1000),
        if use_large_db_numbers { 2032. } else { 2791. },
    );

    runner.run_with_tps_estimate(
        "CreateAccount",
        account_1,
        aptos_stdlib::aptos_account_create_account(
            AccountAddress::from_hex_literal("0xcafe1").unwrap(),
        ),
        if use_large_db_numbers { 1583.0 } else { 2215. },
    );

    let mut package_handler = PackageHandler::new("simple");
    let mut rng = StdRng::seed_from_u64(14);
    let package = package_handler.pick_package(&mut rng, *account_1.address());
    runner.run_with_tps_estimate(
        "PublishModule",
        account_1,
        package.publish_transaction_payload(),
        if use_large_db_numbers { 138.0 } else { 148. },
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

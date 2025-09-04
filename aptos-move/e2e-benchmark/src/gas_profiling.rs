// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_block_executor::types::{InputOutputKey, ReadWriteSummary};
use aptos_gas_algebra::GasQuantity;
use aptos_gas_profiling::TransactionGasLog;
use aptos_language_e2e_tests::account::Account;
use aptos_sdk::{
    transaction_builder::TransactionFactory,
    types::{AccountKey, LocalAccount},
};
use aptos_transaction_generator_lib::{
    call_custom_modules::CustomModulesDelegationGeneratorCreator,
    entry_point_trait::EntryPointTrait,
    entry_points::EntryPointTransactionGenerator,
    workflow_delegator::{WorkflowKind, WorkflowTxnGeneratorCreator},
    AlwaysApproveRootAccountHandle, CounterState, ReliableTransactionSubmitter,
    TransactionGenerator, TransactionGeneratorCreator, WorkflowProgress,
};
#[cfg(test)]
use aptos_types::transaction::TransactionPayload;
use aptos_types::{
    account_address::AccountAddress,
    chain_id::ChainId,
    fee_statement::FeeStatement,
    transaction::{signature_verified_transaction::SignatureVerifiedTransaction, SignedTransaction, TransactionExecutableRef},
};
use e2e_move_tests::MoveHarnessSend;
use std::{
    collections::HashMap,
    path::Path,
    sync::{atomic::AtomicUsize, Arc, Mutex},
};

#[derive(Clone, Debug)]
pub enum CalibrationWorkload {
    EntryPoint(Box<dyn EntryPointTrait>),
    Workflow(Box<dyn WorkflowKind>),
}

impl CalibrationWorkload {
    pub async fn initialize(
        self,
        harness: &mut MoveHarnessSend,
        cur_phase: Arc<AtomicUsize>,
    ) -> Box<dyn TransactionGenerator> {
        match self {
            CalibrationWorkload::EntryPoint(entry_point) => {
                initialize_entry_point_workload(entry_point, harness).await
            },
            CalibrationWorkload::Workflow(workflow_kind) => {
                initialize_workflow_workload(workflow_kind, cur_phase, harness).await
            },
        }
    }
}

pub struct CalibrationRunner {
    pub harness: MoveHarnessSend,
    profile_gas: bool,
}

impl CalibrationRunner {
    pub fn new(harness: MoveHarnessSend, profile_gas: bool) -> Self {
        Self {
            harness,
            profile_gas,
        }
    }

    pub async fn run_workload(
        &mut self,
        workload: CalibrationWorkload,
        name: String,
        to_skip: usize,
        to_evaluate: usize,
        tps: f64,
    ) {
        let cur_phase = Arc::new(AtomicUsize::new(0));
        let mut creator = workload
            .initialize(&mut self.harness, cur_phase.clone())
            .await;

        let user = into_local_account(self.harness.new_account_with_key_pair());

        let mut generate_next = move || {
            let mut in_a_row = 0;
            loop {
                let txns = creator.generate_transactions(&user, 1);
                if txns.is_empty() {
                    in_a_row += 1;
                    if in_a_row > 10 {
                        cur_phase.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        in_a_row = 0;
                    }
                } else {
                    assert_eq!(txns.len(), 1);
                    return txns.into_iter().next().unwrap();
                }
            }
        };
        let to_skip_txns = (0..to_skip).map(|_| generate_next()).collect::<Vec<_>>();

        execute_block_expect_success(to_skip_txns, &mut self.harness);

        let mut aggregate_gas_log: Option<TransactionGasLog> = None;

        let mut read_write_sets = vec![];

        for i in 0..to_evaluate {
            let txn = generate_next();
            let cur_name = if to_evaluate > 1 {
                if let TransactionExecutableRef::EntryFunction(entry_fun) =
                    txn.payload().executable_ref().unwrap()
                {
                    format!("{}_{}_{}", name, i, entry_fun.function().as_str())
                } else {
                    format!("{}_{}", name, i)
                }
            } else {
                name.clone()
            };
            let (log, fee_statement) = self.run_with_tps_estimate_signed(&cur_name, txn, tps);

            if let Some(mut log) = log {
                let reads = log.exec_io.call_graph.get_reads().into_iter().map(InputOutputKey::Resource).collect();
                let writes = log.exec_io.write_set_transient.iter().map(|w| InputOutputKey::Resource(w.key.clone())).collect();

                let exe_and_io_gas = fee_statement.map_or(0, |fee_statement| {
                    fee_statement.execution_gas_used() + fee_statement.io_gas_used()
                });

                read_write_sets.push((cur_name, exe_and_io_gas, ReadWriteSummary::<SignatureVerifiedTransaction>::new(reads, writes)));
                log.exec_io.call_graph = log.exec_io.call_graph.fold_unique_stack();
                aggregate_gas_log = Some(
                    if let Some(aggregate_gas_log) = aggregate_gas_log {
                        aggregate_gas_log.combine(&log)
                    } else {
                        log
                    },
                );
            }
        }

        if to_evaluate > 1 {
            if let Some(mut aggregate_gas_log) = aggregate_gas_log {
                aggregate_gas_log.exec_io.call_graph =
                    aggregate_gas_log.exec_io.call_graph.fold_unique_stack();
                save_profiling_results_with_path_name(
                    &format!("{} aggregated", name),
                    &format!("{} with folded unique stack", name),
                    &aggregate_gas_log,
                );
            }
        }


        if !read_write_sets.is_empty() {
            let mut gas = vec![];
            let mut end_gas = vec![];
            for cur in 0..to_evaluate {
                let cur_gas = read_write_sets[cur].1;
                gas.push(cur_gas);
                let mut start = 0;
                println!("== Conflicts for txn [{}] {} == ", cur, &read_write_sets[cur].0);
                for prev in 0..cur {
                    let cur_rw = &read_write_sets[cur].2;
                    let prev_rw = &read_write_sets[prev].2;
                    let conflicts = cur_rw.find_conflicts(prev_rw);
                    if !conflicts.is_empty() {
                        println!("[{}] {} {:?}", prev, conflicts.len(), conflicts);
                        start = start.max(end_gas[prev]);
                    }
                }
                end_gas.push(start + cur_gas);
                println!("Takes {} gas, finishes after {} gas", cur_gas, start + cur_gas);
            }
            println!("End gas: {:?}, total gas: {}", end_gas, gas.iter().sum::<u64>());
        }
    }

    #[cfg(test)]
    fn run(&mut self, function: &str, account: &Account, payload: TransactionPayload) {
        let (gas_used, fee_statement) = if !self.profile_gas {
            self.harness.evaluate_gas(account, payload)
        } else {
            let (log, gas_used, fee_statement) =
                self.harness.evaluate_gas_with_profiler(account, payload);
            save_profiling_results(function, &log);
            (gas_used, fee_statement)
        };
        print_gas_cost_with_statement(function, gas_used, fee_statement);
    }

    #[cfg(test)]
    fn run_with_tps_estimate(
        &mut self,
        function: &str,
        account: &Account,
        payload: TransactionPayload,
        tps: f64,
    ) {
        if !self.profile_gas {
            let (gas_used, fee_statement) = self.harness.evaluate_gas(account, payload);
            print_gas_cost_with_statement(function, gas_used, fee_statement);
        } else {
            let (log, gas_used, fee_statement) =
                self.harness.evaluate_gas_with_profiler(account, payload);
            save_profiling_results(function, &log);
            print_gas_cost_with_statement_and_tps(
                function,
                gas_used,
                fee_statement,
                summarize_exe_and_io(&log),
                tps,
            );
        }
    }

    pub fn run_with_tps_estimate_signed(
        &mut self,
        function: &str,
        txn: SignedTransaction,
        tps: f64,
    ) -> (Option<TransactionGasLog>, Option<FeeStatement>) {
        if !self.profile_gas {
            let (gas_used, fee_statement) = self.harness.evaluate_gas_signed(txn);
            print_gas_cost_with_statement(function, gas_used, fee_statement);
            (None, fee_statement)
        } else {
            let (log, gas_used, fee_statement) =
                self.harness.evaluate_gas_with_profiler_signed(txn);
            save_profiling_results(function, &log);
            print_gas_cost_with_statement_and_tps(
                function,
                gas_used,
                fee_statement,
                summarize_exe_and_io(&log),
                tps,
            );
            (Some(log), fee_statement)
        }
    }

    #[cfg(test)]
    fn publish(&mut self, name: &str, account: &Account, path: &Path) {
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

fn into_local_account(account: Account) -> LocalAccount {
    LocalAccount::new(
        *account.address(),
        AccountKey::from_private_key(account.privkey),
        0,
    )
}

fn create_transaction_factory() -> TransactionFactory {
    TransactionFactory::new(ChainId::test())
        .with_absolute_transaction_expiration_timestamp(30)
        .with_gas_unit_price(100)
        .with_max_gas_amount(2_000_000)
}

async fn initialize_entry_point_workload(
    entry_point: Box<dyn EntryPointTrait>,
    harness: &mut MoveHarnessSend,
) -> Box<dyn TransactionGenerator> {
    let txn_factory = create_transaction_factory();
    let source_account = AlwaysApproveRootAccountHandle {
        root_account: Arc::new(into_local_account(harness.store_and_fund_account(
            &Account::new(),
            u64::MAX / 4,
            0,
        ))),
    };
    let txn_executor = HarnessReliableTransactionSubmitter::new(harness);
    let num_modules = 1;

    let generator = CustomModulesDelegationGeneratorCreator::new(
        txn_factory.clone(),
        txn_factory.clone(),
        &source_account,
        &txn_executor,
        num_modules,
        entry_point.pre_built_packages(),
        entry_point.package_name(),
        &mut EntryPointTransactionGenerator::new_singleton(entry_point),
    )
    .await;

    generator.create_transaction_generator()
}

async fn initialize_workflow_workload(
    workflow_kind: Box<dyn WorkflowKind>,
    cur_phase: Arc<AtomicUsize>,
    harness: &mut MoveHarnessSend,
) -> Box<dyn TransactionGenerator> {
    let txn_factory = create_transaction_factory();
    let source_account = AlwaysApproveRootAccountHandle {
        root_account: Arc::new(into_local_account(harness.store_and_fund_account(
            &Account::new(),
            u64::MAX / 4,
            0,
        ))),
    };
    let txn_executor = HarnessReliableTransactionSubmitter::new(harness);
    let num_modules = 1;

    let generator = WorkflowTxnGeneratorCreator::create_workload(
        workflow_kind,
        txn_factory.clone(),
        txn_factory.clone(),
        &source_account,
        &txn_executor,
        num_modules,
        cur_phase.clone(),
        WorkflowProgress::MoveByPhases,
    )
    .await;

    generator.create_transaction_generator()
}

fn execute_block_expect_success(txns: Vec<SignedTransaction>, harness: &mut MoveHarnessSend) {
    let outputs = harness.run_block(txns.clone());

    for (idx, (status, txn)) in outputs.into_iter().zip(txns.into_iter()).enumerate() {
        assert!(
            status.is_kept(),
            "[{idx}] status {:?} for txn {:.2000?}",
            status,
            txn
        );
        assert!(
            status.status().unwrap().is_success(),
            "[{idx}] status {:?} for txn {:.2000?}",
            status,
            txn
        );
    }
}

pub struct HarnessReliableTransactionSubmitter<'a> {
    pub harness: Mutex<&'a mut MoveHarnessSend>,
}

impl<'a> HarnessReliableTransactionSubmitter<'a> {
    pub fn new(harness: &'a mut MoveHarnessSend) -> Self {
        Self {
            harness: Mutex::new(harness),
        }
    }

    // pub fn into_innner(self) -> MoveHarness {
    //     self.harness.into_inner().unwrap()
    // }
}

#[async_trait::async_trait]
impl<'a> ReliableTransactionSubmitter for HarnessReliableTransactionSubmitter<'a> {
    async fn get_account_balance(&self, account_address: AccountAddress) -> anyhow::Result<u64> {
        Ok(self
            .harness
            .lock()
            .unwrap()
            .read_aptos_balance(&account_address))
    }

    async fn query_sequence_number(&self, address: AccountAddress) -> anyhow::Result<u64> {
        Ok(self
            .harness
            .lock()
            .unwrap()
            .sequence_number_opt(&address)
            .unwrap_or(0))
    }

    async fn execute_transactions_with_counter(
        &self,
        txns: &[SignedTransaction],
        _state: &CounterState,
    ) -> anyhow::Result<()> {
        execute_block_expect_success(txns.to_vec(), self.harness.lock().as_mut().unwrap());
        Ok(())
    }

    fn create_counter_state(&self) -> CounterState {
        CounterState {
            submit_failures: vec![AtomicUsize::new(0)],
            wait_failures: vec![AtomicUsize::new(0)],
            successes: AtomicUsize::new(0),
            by_client: HashMap::new(),
        }
    }
}

fn save_profiling_results(name: &str, log: &TransactionGasLog) {
    save_profiling_results_with_path_name(name, name, log);
}

fn save_profiling_results_with_path_name(name: &str, path_name: &str, log: &TransactionGasLog) {
    let path = Path::new("gas-profiling").join(path_name);
    log.generate_html_report(path, format!("Gas Report - {}", name))
        .unwrap();
}

pub struct SummaryExeAndIO {
    pub intrinsic_cost: f64,
    pub execution_cost: f64,
    pub read_cost: f64,
    pub write_cost: f64,
}

fn summarize_exe_and_io(log: &TransactionGasLog) -> SummaryExeAndIO {
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

fn print_gas_cost_with_statement(
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

fn print_gas_cost_with_statement_and_tps(
    function: &str,
    gas_units: u64,
    fee_statement: Option<FeeStatement>,
    summary: SummaryExeAndIO,
    tps: f64,
) {
    let exe_and_io_gas = fee_statement.map_or(0, |fee_statement| {
        fee_statement.execution_gas_used() + fee_statement.io_gas_used()
    });
    println!(
        "{:9} | {:9.6} | {:9.6} | {:9.6} | {:8} | {:8.2} | {:8.2} | {:8.2} | {:8.2} | {:8.0} | {}",
        gas_units,
        dollar_cost(gas_units, 5),
        dollar_cost(gas_units, 15),
        dollar_cost(gas_units, 30),
        exe_and_io_gas,
        // fee_statement.unwrap().execution_gas_used(),
        // fee_statement.unwrap().io_gas_used(),
        summary.intrinsic_cost,
        summary.execution_cost,
        summary.read_cost,
        summary.write_cost,
        (exe_and_io_gas) as f64 * tps,
        function,
    );
}

#[cfg(test)]
mod tests {
    use crate::gas_profiling::{
        print_gas_cost_with_statement_and_tps_header, CalibrationRunner, CalibrationWorkload,
    };
    use aptos_cached_packages::{aptos_stdlib, aptos_token_sdk_builder};
    use aptos_crypto::{bls12381, PrivateKey, Uniform};
    use aptos_sdk::move_types::{identifier::Identifier, language_storage::ModuleId};
    use aptos_transaction_generator_lib::{
        entry_point_trait::EntryPointTrait, publishing::publish_util::PackageHandler,
        workflow_delegator::WorkflowKind,
    };
    use aptos_transaction_workloads_lib::{EntryPoints, LoopType, TokenWorkflowKind};
    use aptos_types::{
        account_address::{default_stake_pool_address, AccountAddress},
        chain_id::ChainId,
        transaction::{EntryFunction, TransactionPayload},
    };
    use aptos_vm_environment::prod_configs::set_paranoid_type_checks;
    use e2e_move_tests::MoveHarnessSend;
    use rand::{rngs::StdRng, SeedableRng};
    use std::path::PathBuf;

    pub fn test_dir_path(s: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("e2e-move-tests")
            .join("src")
            .join("tests")
            .join(s)
    }

    /// Run with `cargo test test_gas -- --nocapture` to see output.
    #[test]
    fn test_gas() {
        // Start with 100 validators.
        let mut harness = MoveHarnessSend::new_with_validators(100);
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

        let mut runner = CalibrationRunner::new(harness, profile_gas);

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

        // publish_object_token_example(&mut runner.harness, account_1_address, account_1);
        // runner.run(
        //     "MintTokenV2",
        //     account_1,
        //     create_mint_hero_payload(&account_1_address, SHORT_STR),
        // );
        // runner.run(
        //     "MutateTokenV2",
        //     account_1,
        //     create_set_hero_description_payload(&account_1_address, SHORT_STR),
        // );
        // publish_object_token_example(&mut runner.harness, account_2_address, account_2);
        // runner.run(
        //     "MintLargeTokenV2",
        //     account_2,
        //     create_mint_hero_payload(&account_2_address, LONG_STR),
        // );
        // runner.run(
        //     "MutateLargeTokenV2",
        //     account_2,
        //     create_set_hero_description_payload(&account_2_address, LONG_STR),
        // );

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

    #[tokio::test]
    #[ignore]
    async fn test_txn_generator_workloads_calibrate_gas() {
        // Start with 100 validators.
        let mut harness = MoveHarnessSend::new_with_validators(100);
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

        let workflows: Vec<(f64, f64, Box<dyn WorkflowKind>)> = vec![(
            1.0,
            1.0,
            Box::new(TokenWorkflowKind::CreateMintBurn {
                count: 1,
                creation_balance: 200000,
            }),
        )];

        let mut runner = CalibrationRunner::new(harness, profile_gas);

        for (large_db_tps, small_db_tps, entry_point) in entry_points {
            let tps = if use_large_db_numbers {
                large_db_tps
            } else {
                small_db_tps
            };
            let name = format!("entry_point_{entry_point:?}");
            runner
                .run_workload(
                    CalibrationWorkload::EntryPoint(Box::new(entry_point)),
                    name,
                    0,
                    1,
                    tps,
                )
                .await;
        }

        for (large_db_tps, small_db_tps, workflow) in workflows {
            let tps = if use_large_db_numbers {
                large_db_tps
            } else {
                small_db_tps
            };
            let name = format!("workflow_{workflow:?}");
            runner
                .run_workload(CalibrationWorkload::Workflow(workflow), name, 0, 3, tps)
                .await;
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

        let mut package_handler =
            PackageHandler::new(EntryPoints::Nop.pre_built_packages(), "simple");
        let mut rng = StdRng::seed_from_u64(14);
        let package = package_handler.pick_package(&mut rng, *account_1.address());
        let payloads = package.publish_transaction_payload(&ChainId::test());
        assert!(payloads.len() == 1);
        runner.run_with_tps_estimate(
            "PublishModule",
            account_1,
            payloads[0].clone(),
            if use_large_db_numbers { 138.0 } else { 148. },
        );
    }

    //     const SHORT_STR: &str = "A hero.";
    //     const LONG_STR: &str = "\
    //         0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    //         0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    //         0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    //         0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    //         0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    //         0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    //         0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    //         0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    //         0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    //         0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    //         0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    //         0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    //         0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    //         0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    //         0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    //         0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    //         0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    //         0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    //         0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    //         0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    //         0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    //         0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    //         0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    //         0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    //         0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    //         0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    //         0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    //         0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    //         0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    //         0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    //         0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    //         0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
    //         ";
}

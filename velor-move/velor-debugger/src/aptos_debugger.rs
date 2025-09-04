// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, format_err};
use velor_block_executor::txn_provider::{default::DefaultTxnProvider, TxnProvider};
use velor_gas_profiling::{GasProfiler, TransactionGasLog};
use velor_rest_client::Client;
use velor_types::{
    account_address::AccountAddress,
    block_executor::{
        config::{BlockExecutorConfig, BlockExecutorConfigFromOnchain, BlockExecutorLocalConfig},
        transaction_slice_metadata::TransactionSliceMetadata,
    },
    contract_event::ContractEvent,
    state_store::TStateView,
    transaction::{
        signature_verified_transaction::SignatureVerifiedTransaction, AuxiliaryInfo, BlockOutput,
        SignedTransaction, Transaction, TransactionExecutableRef, TransactionInfo,
        TransactionOutput, TransactionPayload, Version,
    },
    vm_status::VMStatus,
};
use velor_validator_interface::{
    VelorValidatorInterface, DBDebuggerInterface, DebuggerStateView, RestDebuggerInterface,
};
use velor_vm::{
    velor_vm::VelorVMBlockExecutor, data_cache::AsMoveResolver, VelorVM, VMBlockExecutor,
};
use velor_vm_environment::environment::VelorEnvironment;
use velor_vm_logging::log_schema::AdapterLogSchema;
use velor_vm_types::{module_and_script_storage::AsVelorCodeStorage, output::VMOutput};
use itertools::Itertools;
use std::{path::Path, sync::Arc, time::Instant};

pub struct VelorDebugger {
    debugger: Arc<dyn VelorValidatorInterface + Send>,
}

impl VelorDebugger {
    pub fn new(debugger: Arc<dyn VelorValidatorInterface + Send>) -> Self {
        Self { debugger }
    }

    pub fn rest_client(rest_client: Client) -> anyhow::Result<Self> {
        Ok(Self::new(Arc::new(RestDebuggerInterface::new(rest_client))))
    }

    pub fn db<P: AsRef<Path> + Clone>(db_root_path: P) -> anyhow::Result<Self> {
        Ok(Self::new(Arc::new(DBDebuggerInterface::open(
            db_root_path,
        )?)))
    }

    pub async fn get_committed_transactions(
        &self,
        begin: Version,
        limit: u64,
    ) -> anyhow::Result<(Vec<Transaction>, Vec<TransactionInfo>)> {
        self.debugger.get_committed_transactions(begin, limit).await
    }

    pub fn execute_transactions_at_version(
        &self,
        version: Version,
        txns: Vec<Transaction>,
        repeat_execution_times: u64,
        concurrency_levels: &[usize],
    ) -> anyhow::Result<Vec<TransactionOutput>> {
        let sig_verified_txns: Vec<SignatureVerifiedTransaction> =
            txns.into_iter().map(|x| x.into()).collect::<Vec<_>>();
        // TODO(grao): Pass in persisted info.
        let txn_provider = DefaultTxnProvider::new_without_info(sig_verified_txns);
        let state_view = DebuggerStateView::new(self.debugger.clone(), version);

        print_transaction_stats(txn_provider.get_txns(), version);

        let mut result = None;
        assert!(
            !concurrency_levels.is_empty(),
            "concurrency_levels cannot be empty"
        );
        for concurrency_level in concurrency_levels {
            for i in 0..repeat_execution_times {
                let start_time = Instant::now();
                let cur_result =
                    execute_block_no_limit(&txn_provider, &state_view, *concurrency_level)
                        .map_err(|err| format_err!("Unexpected VM Error: {:?}", err))?;

                println!(
                    "[{} txns from {}] Finished execution round {}/{} with concurrency_level={} in {}ms",
                    txn_provider.num_txns(),
                    version,
                    i + 1,
                    repeat_execution_times,
                    concurrency_level,
                    start_time.elapsed().as_millis(),
                );

                match &result {
                    None => result = Some(cur_result),
                    Some(prev_result) => {
                        if !Self::ensure_output_matches(&cur_result, prev_result, version) {
                            bail!(
                                "Execution result mismatched in round {}/{}",
                                i,
                                repeat_execution_times
                            );
                        }
                    },
                }
            }
        }

        let result = result.unwrap();
        assert_eq!(txn_provider.num_txns(), result.len());
        Ok(result)
    }

    pub fn execute_transaction_at_version_with_gas_profiler(
        &self,
        version: Version,
        txn: SignedTransaction,
        auxiliary_info: AuxiliaryInfo,
    ) -> anyhow::Result<(VMStatus, VMOutput, TransactionGasLog)> {
        let state_view = DebuggerStateView::new(self.debugger.clone(), version);
        let log_context = AdapterLogSchema::new(state_view.id(), 0);
        let txn = txn
            .check_signature()
            .map_err(|err| format_err!("Unexpected VM Error: {:?}", err))?;

        // Module bundle is deprecated!
        if let TransactionPayload::ModuleBundle(_) = txn.payload() {
            bail!("Module bundle payload has been removed")
        }

        let env = VelorEnvironment::new(&state_view);
        let vm = VelorVM::new(&env, &state_view);
        let resolver = state_view.as_move_resolver();
        let code_storage = state_view.as_velor_code_storage(&env);

        let (status, output, gas_profiler) = vm.execute_user_transaction_with_modified_gas_meter(
            &resolver,
            &code_storage,
            &txn,
            &log_context,
            |gas_meter| {
                let gas_profiler = match txn
                    .executable_ref()
                    .expect("Module bundle payload has been removed")
                {
                    TransactionExecutableRef::Script(_) => GasProfiler::new_script(gas_meter),
                    TransactionExecutableRef::EntryFunction(entry_func) => {
                        GasProfiler::new_function(
                            gas_meter,
                            entry_func.module().clone(),
                            entry_func.function().to_owned(),
                            entry_func.ty_args().to_vec(),
                        )
                    },
                    TransactionExecutableRef::Empty => {
                        // TODO[Orderless]: Implement this
                        unimplemented!("not supported yet")
                    },
                };
                gas_profiler
            },
            &auxiliary_info,
        )?;

        Ok((status, output, gas_profiler.finish()))
    }

    pub async fn execute_past_transactions(
        &self,
        begin: Version,
        limit: u64,
        use_same_block_boundaries: bool,
        repeat_execution_times: u64,
        concurrency_levels: &[usize],
    ) -> anyhow::Result<Vec<TransactionOutput>> {
        let (txns, txn_infos) = self.get_committed_transactions(begin, limit).await?;

        if use_same_block_boundaries {
            // when going block by block, no need to worry about epoch boundaries
            // as new epoch is always a new block.
            Ok(self
                .execute_transactions_by_block(
                    begin,
                    txns.clone(),
                    repeat_execution_times,
                    concurrency_levels,
                )
                .await?)
        } else {
            self.execute_transactions_by_epoch(
                limit,
                begin,
                txns,
                repeat_execution_times,
                concurrency_levels,
                txn_infos,
            )
            .await
        }
    }

    fn print_mismatches(
        txn_outputs: &[TransactionOutput],
        expected_txn_infos: &[TransactionInfo],
        first_version: Version,
    ) {
        for idx in 0..txn_outputs.len() {
            let txn_output = &txn_outputs[idx];
            let txn_info = &expected_txn_infos[idx];
            let version = first_version + idx as Version;
            txn_output
                .ensure_match_transaction_info(version, txn_info, None, None)
                .unwrap_or_else(|err| println!("{}", err))
        }
    }

    fn ensure_output_matches(
        txn_outputs: &[TransactionOutput],
        expected_txn_outputs: &[TransactionOutput],
        first_version: Version,
    ) -> bool {
        let mut all_match = true;
        for idx in 0..txn_outputs.len() {
            let txn_output = &txn_outputs[idx];
            let expected_output = &expected_txn_outputs[idx];
            let version = first_version + idx as Version;
            if txn_output != expected_output {
                println!(
                    "Mismatch at version {:?}:\nExpected: {:#?}\nActual: {:#?}",
                    version, expected_output, txn_output
                );
                all_match = false;
            }
        }
        all_match
    }

    async fn execute_transactions_until_epoch_end(
        &self,
        begin: Version,
        txns: Vec<Transaction>,
        repeat_execution_times: u64,
        concurrency_levels: &[usize],
    ) -> anyhow::Result<Vec<TransactionOutput>> {
        let results = self.execute_transactions_at_version(
            begin,
            txns,
            repeat_execution_times,
            concurrency_levels,
        )?;
        let mut ret = vec![];
        let mut is_reconfig = false;

        for result in results.into_iter() {
            if is_reconfig {
                continue;
            }
            if is_reconfiguration(&result) {
                is_reconfig = true;
            }
            ret.push(result)
        }
        Ok(ret)
    }

    async fn execute_transactions_by_epoch(
        &self,
        mut limit: u64,
        mut begin: u64,
        mut txns: Vec<Transaction>,
        repeat_execution_times: u64,
        concurrency_levels: &[usize],
        mut txn_infos: Vec<TransactionInfo>,
    ) -> anyhow::Result<Vec<TransactionOutput>> {
        let mut ret = vec![];
        while limit != 0 {
            println!(
                "Starting epoch execution at {:?}, {:?} transactions remaining",
                begin, limit
            );

            let mut epoch_result = self
                .execute_transactions_until_epoch_end(
                    begin,
                    txns.clone(),
                    repeat_execution_times,
                    concurrency_levels,
                )
                .await?;
            begin += epoch_result.len() as u64;
            limit -= epoch_result.len() as u64;
            txns = txns.split_off(epoch_result.len());
            let epoch_txn_infos = txn_infos.drain(0..epoch_result.len()).collect::<Vec<_>>();
            Self::print_mismatches(&epoch_result, &epoch_txn_infos, begin);

            ret.append(&mut epoch_result);
        }
        Ok(ret)
    }

    async fn execute_transactions_by_block(
        &self,
        begin: Version,
        txns: Vec<Transaction>,
        repeat_execution_times: u64,
        concurrency_levels: &[usize],
    ) -> anyhow::Result<Vec<TransactionOutput>> {
        let mut ret = vec![];
        let mut cur = vec![];
        let mut cur_version = begin;
        for txn in txns {
            if txn.is_block_start() && !cur.is_empty() {
                let to_execute = std::mem::take(&mut cur);
                let results = self.execute_transactions_at_version(
                    cur_version,
                    to_execute,
                    repeat_execution_times,
                    concurrency_levels,
                )?;
                cur_version += results.len() as u64;
                ret.extend(results);
            }
            cur.push(txn);
        }
        if !cur.is_empty() {
            let results = self.execute_transactions_at_version(
                cur_version,
                cur,
                repeat_execution_times,
                concurrency_levels,
            )?;
            ret.extend(results);
        }

        Ok(ret)
    }

    pub async fn get_version_by_account_sequence(
        &self,
        account: AccountAddress,
        seq: u64,
    ) -> anyhow::Result<Option<Version>> {
        self.debugger
            .get_version_by_account_sequence(account, seq)
            .await
    }

    pub async fn get_committed_transaction_at_version(
        &self,
        version: Version,
    ) -> anyhow::Result<(Transaction, TransactionInfo)> {
        let (mut txns, mut info) = self.debugger.get_committed_transactions(version, 1).await?;

        let txn = txns.pop().expect("there must be exactly 1 txn in the vec");
        let info = info
            .pop()
            .expect("there must be exactly 1 txn info in the vec");

        Ok((txn, info))
    }

    pub fn state_view_at_version(&self, version: Version) -> DebuggerStateView {
        DebuggerStateView::new(self.debugger.clone(), version)
    }
}

fn print_transaction_stats(sig_verified_txns: &[SignatureVerifiedTransaction], version: u64) {
    let transaction_types = sig_verified_txns
        .iter()
        .map(|txn| txn.expect_valid().type_name().to_string())
        // conflate same consecutive elements into one with count
        .chunk_by(|k| k.clone())
        .into_iter()
        .map(|(k, r)| {
            let num = r.count();
            if num > 1 {
                format!("{} {}s", num, k)
            } else {
                k
            }
        })
        .collect::<Vec<_>>();
    let entry_functions = sig_verified_txns
        .iter()
        .filter_map(|txn| {
            txn.expect_valid().try_as_signed_user_txn().map(|txn| {
                let mut executable_type = match &txn.payload().executable_ref() {
                    Ok(TransactionExecutableRef::EntryFunction(txn)) => format!(
                        "entry: {:?}::{:?}",
                        txn.module().name.as_str(),
                        txn.function().as_str()
                    ),
                    Ok(TransactionExecutableRef::Script(_)) => "script".to_string(),
                    Ok(TransactionExecutableRef::Empty) => "empty".to_string(),
                    Err(e) => {
                        panic!("deprecated transaction payload: {}", e)
                    },
                };
                if txn.payload().is_multisig() {
                    executable_type = format!("multisig: {}", executable_type);
                }
                executable_type
            })
        })
        // Count number of instances for each (irrsepsecitve of order)
        .sorted()
        .chunk_by(|k| k.clone())
        .into_iter()
        .map(|(k, r)| (r.count(), k))
        .sorted_by_key(|(num, _k)| *num)
        .rev()
        .map(|(num, k)| {
            if num > 1 {
                format!("{} {}s", num, k)
            } else {
                k
            }
        })
        .collect::<Vec<_>>();
    println!(
        "[{} txns from {}] Transaction types: {:?}",
        sig_verified_txns.len(),
        version,
        transaction_types
    );
    println!(
        "[{} txns from {}] Entry Functions {:?}",
        sig_verified_txns.len(),
        version,
        entry_functions
    );
}

fn is_reconfiguration(vm_output: &TransactionOutput) -> bool {
    vm_output
        .events()
        .iter()
        .any(ContractEvent::is_new_epoch_event)
}

fn execute_block_no_limit(
    txn_provider: &DefaultTxnProvider<SignatureVerifiedTransaction, AuxiliaryInfo>,
    state_view: &DebuggerStateView,
    concurrency_level: usize,
) -> Result<Vec<TransactionOutput>, VMStatus> {
    let executor = VelorVMBlockExecutor::new();
    executor
        .execute_block_with_config(
            txn_provider,
            state_view,
            BlockExecutorConfig {
                local: BlockExecutorLocalConfig::default_with_concurrency_level(concurrency_level),
                onchain: BlockExecutorConfigFromOnchain::new_no_block_limit(),
            },
            TransactionSliceMetadata::unknown(),
        )
        .map(BlockOutput::into_transaction_outputs_forced)
}

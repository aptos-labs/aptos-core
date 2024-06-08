// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, format_err, Result};
use aptos_gas_profiling::{GasProfiler, TransactionGasLog};
use aptos_rest_client::Client;
use aptos_types::{
    account_address::AccountAddress,
    state_store::TStateView,
    transaction::{
        signature_verified_transaction::SignatureVerifiedTransaction, SignedTransaction,
        Transaction, TransactionInfo, TransactionOutput, TransactionPayload, Version,
    },
    vm_status::VMStatus,
};
use aptos_validator_interface::{
    AptosValidatorInterface, DBDebuggerInterface, DebuggerStateView, RestDebuggerInterface,
};
use aptos_vm::{data_cache::AsMoveResolver, AptosVM, VMExecutor};
use aptos_vm_logging::log_schema::AdapterLogSchema;
use aptos_vm_types::output::VMOutput;
use std::{path::Path, sync::Arc};

pub struct AptosDebugger {
    debugger: Arc<dyn AptosValidatorInterface + Send>,
}

impl AptosDebugger {
    pub fn new(debugger: Arc<dyn AptosValidatorInterface + Send>) -> Self {
        Self { debugger }
    }

    pub fn rest_client(rest_client: Client) -> Result<Self> {
        Ok(Self::new(Arc::new(RestDebuggerInterface::new(rest_client))))
    }

    pub fn db<P: AsRef<Path> + Clone>(db_root_path: P) -> Result<Self> {
        Ok(Self::new(Arc::new(DBDebuggerInterface::open(
            db_root_path,
        )?)))
    }

    pub fn execute_transactions_at_version(
        &self,
        version: Version,
        txns: Vec<Transaction>,
        repeat_execution_times: u64,
    ) -> Result<Vec<TransactionOutput>> {
        let sig_verified_txns: Vec<SignatureVerifiedTransaction> =
            txns.into_iter().map(|x| x.into()).collect::<Vec<_>>();
        let state_view = DebuggerStateView::new(self.debugger.clone(), version);

        let result = AptosVM::execute_block_no_limit(&sig_verified_txns, &state_view)
            .map_err(|err| format_err!("Unexpected VM Error: {:?}", err))?;

        for i in 1..repeat_execution_times {
            let repeat_result = AptosVM::execute_block_no_limit(&sig_verified_txns, &state_view)
                .map_err(|err| format_err!("Unexpected VM Error: {:?}", err))?;
            println!(
                "Finished execution round {}/{} with {} transactions",
                i,
                repeat_execution_times,
                sig_verified_txns.len()
            );
            if !Self::ensure_output_matches(&repeat_result, &result, version) {
                bail!(
                    "Execution result mismatched in round {}/{}",
                    i,
                    repeat_execution_times
                );
            }
        }
        Ok(result)
    }

    pub fn execute_transaction_at_version_with_gas_profiler(
        &self,
        version: Version,
        txn: SignedTransaction,
    ) -> Result<(VMStatus, VMOutput, TransactionGasLog)> {
        let state_view = DebuggerStateView::new(self.debugger.clone(), version);
        let log_context = AdapterLogSchema::new(state_view.id(), 0);
        let txn = txn
            .check_signature()
            .map_err(|err| format_err!("Unexpected VM Error: {:?}", err))?;

        // TODO(Gas): revisit this.
        let resolver = state_view.as_move_resolver();
        let vm = AptosVM::new(
            &resolver,
            /*override_is_delayed_field_optimization_capable=*/ Some(false),
        );

        // Module bundle is deprecated!
        if let TransactionPayload::ModuleBundle(_) = txn.payload() {
            anyhow::bail!("Module bundle payload has been removed")
        }

        let (status, output, gas_profiler) = vm.execute_user_transaction_with_modified_gas_meter(
            &resolver,
            &txn,
            &log_context,
            |gas_meter| {
                let gas_profiler = match txn.payload() {
                    TransactionPayload::Script(_) => GasProfiler::new_script(gas_meter),
                    TransactionPayload::EntryFunction(entry_func) => GasProfiler::new_function(
                        gas_meter,
                        entry_func.module().clone(),
                        entry_func.function().to_owned(),
                        entry_func.ty_args().to_vec(),
                    ),
                    TransactionPayload::Multisig(..) => unimplemented!("not supported yet"),

                    // Deprecated.
                    TransactionPayload::ModuleBundle(..) => {
                        unreachable!("Module bundle payload has already been checked because before this function is called")
                    },
                };
                gas_profiler
            },
        )?;

        Ok((status, output, gas_profiler.finish()))
    }

    pub async fn execute_past_transactions(
        &self,
        mut begin: Version,
        mut limit: u64,
        repeat_execution_times: u64,
    ) -> Result<Vec<TransactionOutput>> {
        let (mut txns, mut txn_infos) = self
            .debugger
            .get_committed_transactions(begin, limit)
            .await?;

        let mut ret = vec![];
        while limit != 0 {
            println!(
                "Starting epoch execution at {:?}, {:?} transactions remaining",
                begin, limit
            );
            let mut epoch_result = self
                .execute_transactions_by_epoch(begin, txns.clone(), repeat_execution_times)
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

    pub async fn execute_transactions_by_epoch(
        &self,
        begin: Version,
        txns: Vec<Transaction>,
        repeat_execution_times: u64,
    ) -> Result<Vec<TransactionOutput>> {
        let results = self.execute_transactions_at_version(begin, txns, repeat_execution_times)?;
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

    pub async fn get_version_by_account_sequence(
        &self,
        account: AccountAddress,
        seq: u64,
    ) -> Result<Option<Version>> {
        self.debugger
            .get_version_by_account_sequence(account, seq)
            .await
    }

    pub async fn get_committed_transaction_at_version(
        &self,
        version: Version,
    ) -> Result<(Transaction, TransactionInfo)> {
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

fn is_reconfiguration(vm_output: &TransactionOutput) -> bool {
    let new_epoch_event_key = aptos_types::on_chain_config::new_epoch_event_key();
    vm_output
        .events()
        .iter()
        .any(|event| event.event_key() == Some(&new_epoch_event_key))
}

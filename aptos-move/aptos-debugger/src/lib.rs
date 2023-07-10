// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{format_err, Result};
use aptos_gas::{
    AbstractValueSizeGasParameters, ChangeSetConfigs, NativeGasParameters, StandardGasMeter,
    LATEST_GAS_FEATURE_VERSION,
};
use aptos_gas_profiling::{GasProfiler, TransactionGasLog};
use aptos_memory_usage_tracker::MemoryTrackedGasMeter;
use aptos_resource_viewer::{AnnotatedAccountStateBlob, AptosValueAnnotator};
use aptos_rest_client::Client;
use aptos_state_view::TStateView;
use aptos_types::{
    account_address::AccountAddress,
    chain_id::ChainId,
    on_chain_config::{Features, OnChainConfig, TimedFeatures},
    transaction::{
        SignedTransaction, Transaction, TransactionInfo, TransactionOutput, TransactionPayload,
        Version,
    },
    vm_status::VMStatus,
};
use aptos_validator_interface::{
    AptosValidatorInterface, DBDebuggerInterface, DebuggerStateView, RestDebuggerInterface,
};
use aptos_vm::{
    data_cache::StorageAdapter,
    move_vm_ext::{MoveVmExt, SessionExt, SessionId},
    AptosVM, VMExecutor,
};
use aptos_vm_logging::log_schema::AdapterLogSchema;
use aptos_vm_types::{change_set::VMChangeSet, output::VMOutput};
use move_binary_format::errors::VMResult;
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
    ) -> Result<Vec<TransactionOutput>> {
        let state_view = DebuggerStateView::new(self.debugger.clone(), version);
        AptosVM::execute_block(txns, &state_view, None)
            .map_err(|err| format_err!("Unexpected VM Error: {:?}", err))
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

        let (status, output, gas_profiler) =
            AptosVM::execute_user_transaction_with_custom_gas_meter(
                &state_view,
                &txn,
                &log_context,
                |gas_feature_version, gas_params, storage_gas_params, balance| {
                    let gas_meter = MemoryTrackedGasMeter::new(StandardGasMeter::new(
                        gas_feature_version,
                        gas_params,
                        storage_gas_params,
                        balance,
                    ));
                    let gas_profiler = match txn.payload() {
                        TransactionPayload::Script(_) => GasProfiler::new_script(gas_meter),
                        TransactionPayload::EntryFunction(entry_func) => GasProfiler::new_function(
                            gas_meter,
                            entry_func.module().clone(),
                            entry_func.function().to_owned(),
                            entry_func.ty_args().to_vec(),
                        ),
                        TransactionPayload::ModuleBundle(..) => unreachable!("not supported"),
                        TransactionPayload::Multisig(..) => unimplemented!("not supported yet"),
                    };
                    Ok(gas_profiler)
                },
            )?;

        Ok((status, output, gas_profiler.finish()))
    }

    pub async fn execute_past_transactions(
        &self,
        mut begin: Version,
        mut limit: u64,
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
                .execute_transactions_by_epoch(begin, txns.clone())
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

    pub async fn execute_transactions_by_epoch(
        &self,
        begin: Version,
        txns: Vec<Transaction>,
    ) -> Result<Vec<TransactionOutput>> {
        let results = self.execute_transactions_at_version(begin, txns)?;
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

    pub async fn annotate_account_state_at_version(
        &self,
        account: AccountAddress,
        version: Version,
    ) -> Result<Option<AnnotatedAccountStateBlob>> {
        let state_view = DebuggerStateView::new(self.debugger.clone(), version);
        let remote_storage = StorageAdapter::new(&state_view);
        let annotator = AptosValueAnnotator::new(&remote_storage);
        Ok(
            match self
                .debugger
                .get_account_state_by_version(account, version)
                .await?
            {
                Some(account_state) => Some(annotator.view_account_state(&account_state)?),
                None => None,
            },
        )
    }

    pub async fn annotate_key_accounts_at_version(
        &self,
        version: Version,
    ) -> Result<Vec<(AccountAddress, AnnotatedAccountStateBlob)>> {
        let accounts = self.debugger.get_admin_accounts(version).await?;
        let state_view = DebuggerStateView::new(self.debugger.clone(), version);
        let remote_storage = StorageAdapter::new(&state_view);
        let annotator = AptosValueAnnotator::new(&remote_storage);

        let mut result = vec![];
        for (addr, state) in accounts.into_iter() {
            result.push((addr, annotator.view_account_state(&state)?));
        }
        Ok(result)
    }

    pub async fn get_latest_version(&self) -> Result<Version> {
        self.debugger.get_latest_version().await
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

    pub fn run_session_at_version<F>(&self, version: Version, f: F) -> Result<VMChangeSet>
    where
        F: FnOnce(&mut SessionExt) -> VMResult<()>,
    {
        let state_view = DebuggerStateView::new(self.debugger.clone(), version);
        let state_view_storage = StorageAdapter::new(&state_view);
        let features = Features::fetch_config(&state_view_storage).unwrap_or_default();
        let move_vm = MoveVmExt::new(
            NativeGasParameters::zeros(),
            AbstractValueSizeGasParameters::zeros(),
            LATEST_GAS_FEATURE_VERSION,
            ChainId::test().id(),
            features,
            TimedFeatures::enable_all(),
        )
        .unwrap();
        let mut session = move_vm.new_session(&state_view_storage, SessionId::Void, true);
        f(&mut session).map_err(|err| format_err!("Unexpected VM Error: {:?}", err))?;
        let change_set = session
            .finish(
                &mut (),
                &ChangeSetConfigs::unlimited_at_gas_feature_version(LATEST_GAS_FEATURE_VERSION),
            )
            .map_err(|err| format_err!("Unexpected VM Error: {:?}", err))?;
        Ok(change_set)
    }
}

fn is_reconfiguration(vm_output: &TransactionOutput) -> bool {
    let new_epoch_event_key = aptos_types::on_chain_config::new_epoch_event_key();
    vm_output
        .events()
        .iter()
        .any(|event| *event.key() == new_epoch_event_key)
}

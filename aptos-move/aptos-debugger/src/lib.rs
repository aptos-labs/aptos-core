// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::{format_err, Result};
use aptos_crypto::hash::{CryptoHash, EventAccumulatorHasher};
use aptos_gas::{
    AbstractValueSizeGasParameters, ChangeSetConfigs, NativeGasParameters,
    LATEST_GAS_FEATURE_VERSION,
};
use aptos_resource_viewer::{AnnotatedAccountStateBlob, AptosValueAnnotator};
use aptos_rest_client::Client;
use aptos_types::{
    account_address::AccountAddress,
    chain_id::ChainId,
    on_chain_config::{Features, OnChainConfig},
    proof::accumulator::InMemoryAccumulator,
    transaction::{
        ChangeSet, Transaction, TransactionInfo, TransactionOutput, TransactionStatus, Version,
    },
};
use aptos_validator_interface::{
    AptosValidatorInterface, DBDebuggerInterface, DebuggerStateView, RestDebuggerInterface,
};
use aptos_vm::{
    data_cache::StorageAdapter,
    move_vm_ext::{MoveVmExt, SessionExt, SessionId},
    AptosVM, VMExecutor,
};
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
        AptosVM::execute_block(txns, &state_view)
            .map_err(|err| format_err!("Unexpected VM Error: {:?}", err))
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
            let expected_txn_status: TransactionStatus = txn_info.status().clone().into();
            if txn_output.status() != &expected_txn_status {
                println!(
                    "Mismatch: ver:{} status:{:?} on_chain:{:?}",
                    version,
                    txn_output.status(),
                    expected_txn_status,
                );
            }

            if txn_output.gas_used() != txn_info.gas_used() {
                println!(
                    "Mismatch: ver:{} gas_used:{} on_chain:{}",
                    version,
                    txn_output.gas_used(),
                    txn_info.gas_used(),
                );
            }

            let write_set_hash = txn_output.write_set().hash();
            if write_set_hash != txn_info.state_change_hash() {
                println!(
                    "Mismatch: ver:{} write_set_hash:{} on_chain:{}",
                    version,
                    write_set_hash,
                    txn_info.state_change_hash(),
                );
            }

            let event_hashes = txn_output
                .events()
                .iter()
                .map(CryptoHash::hash)
                .collect::<Vec<_>>();
            let event_root_hash =
                InMemoryAccumulator::<EventAccumulatorHasher>::from_leaves(&event_hashes).root_hash;
            if event_root_hash != txn_info.event_root_hash() {
                println!(
                    "Mismatch: ver:{} event_root_hash:{} on_chain:{}",
                    version,
                    event_root_hash,
                    txn_info.event_root_hash(),
                );
            }
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

    pub fn run_session_at_version<F>(&self, version: Version, f: F) -> Result<ChangeSet>
    where
        F: FnOnce(&mut SessionExt<StorageAdapter<DebuggerStateView>>) -> VMResult<()>,
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
        )
        .unwrap();
        let mut session = move_vm.new_session(&state_view_storage, SessionId::Void);
        f(&mut session).map_err(|err| format_err!("Unexpected VM Error: {:?}", err))?;
        session
            .finish()
            .map_err(|err| format_err!("Unexpected VM Error: {:?}", err))?
            .into_change_set(
                &mut (),
                &ChangeSetConfigs::unlimited_at_gas_feature_version(LATEST_GAS_FEATURE_VERSION),
            )
            .map_err(|err| format_err!("Unexpected VM Error: {:?}", err))
            .map(|res| res.into_inner().1)
    }
}

fn is_reconfiguration(vm_output: &TransactionOutput) -> bool {
    let new_epoch_event_key = aptos_types::on_chain_config::new_epoch_event_key();
    vm_output
        .events()
        .iter()
        .any(|event| *event.key() == new_epoch_event_key)
}

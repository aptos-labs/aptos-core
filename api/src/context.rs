// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_api_types::{Error, LedgerInfo, TransactionOnChainData};
use aptos_config::config::ApiConfig;
use aptos_crypto::HashValue;
use aptos_mempool::{MempoolClientRequest, MempoolClientSender, SubmissionStatus};
use aptos_types::{
    account_address::AccountAddress,
    account_state::AccountState,
    chain_id::ChainId,
    contract_event::ContractEvent,
    event::EventKey,
    ledger_info::LedgerInfoWithSignatures,
    transaction::{SignedTransaction, TransactionWithProof},
};
use storage_interface::{DbReader, Order};

use anyhow::{ensure, format_err, Result};
use aptos_state_view::StateView;
use aptos_types::{
    state_store::{state_key::StateKey, state_key_prefix::StateKeyPrefix},
    transaction::Version,
};
use aptos_vm::data_cache::{IntoMoveResolver, RemoteStorageOwned};
use futures::{channel::oneshot, SinkExt};
use std::{convert::Infallible, sync::Arc};
use storage_interface::state_view::{DbStateView, DbStateViewAtVersion, LatestDbStateView};
use warp::{filters::BoxedFilter, Filter, Reply};

// Context holds application scope context
#[derive(Clone)]
pub struct Context {
    chain_id: ChainId,
    db: Arc<dyn DbReader>,
    mp_sender: MempoolClientSender,
    api_config: ApiConfig,
}

impl Context {
    pub fn new(
        chain_id: ChainId,
        db: Arc<dyn DbReader>,
        mp_sender: MempoolClientSender,
        api_config: ApiConfig,
    ) -> Self {
        Self {
            chain_id,
            db,
            mp_sender,
            api_config,
        }
    }

    pub fn move_resolver(&self) -> Result<RemoteStorageOwned<DbStateView>> {
        self.db
            .latest_state_view()
            .map(|state_view| state_view.into_move_resolver())
    }

    pub fn state_view_at_version(&self, version: Version) -> Result<DbStateView> {
        self.db.state_view_at_version(Some(version))
    }

    pub fn chain_id(&self) -> ChainId {
        self.chain_id
    }

    pub fn content_length_limit(&self) -> u64 {
        self.api_config.content_length_limit()
    }

    pub fn filter(self) -> impl Filter<Extract = (Context,), Error = Infallible> + Clone {
        warp::any().map(move || self.clone())
    }

    pub async fn submit_transaction(&self, txn: SignedTransaction) -> Result<SubmissionStatus> {
        let (req_sender, callback) = oneshot::channel();
        self.mp_sender
            .clone()
            .send(MempoolClientRequest::SubmitTransaction(txn, req_sender))
            .await?;

        callback.await?
    }

    pub fn get_latest_ledger_info(&self) -> Result<LedgerInfo, Error> {
        Ok(LedgerInfo::new(
            &self.chain_id(),
            &self.get_latest_ledger_info_with_signatures()?,
        ))
    }

    pub fn get_latest_ledger_info_with_signatures(&self) -> Result<LedgerInfoWithSignatures> {
        self.db.get_latest_ledger_info()
    }

    pub fn get_state_value(&self, state_key: &StateKey, version: u64) -> Result<Option<Vec<u8>>> {
        self.db
            .state_view_at_version(Some(version))?
            .get_state_value(state_key)
    }

    pub fn get_account_state(
        &self,
        address: AccountAddress,
        version: u64,
    ) -> Result<Option<AccountState>> {
        AccountState::from_access_paths_and_values(
            &self
                .db
                .get_state_values_by_key_prefix(&StateKeyPrefix::from(address), version)?,
        )
    }

    pub fn get_block_timestamp(&self, version: u64) -> Result<u64> {
        self.db.get_block_timestamp(version)
    }

    pub fn get_transactions(
        &self,
        start_version: u64,
        limit: u16,
        ledger_version: u64,
    ) -> Result<Vec<TransactionOnChainData>> {
        let data = self
            .db
            .get_transaction_outputs(start_version, limit as u64, ledger_version)?;

        let txn_start_version = data
            .first_transaction_output_version
            .ok_or_else(|| format_err!("no start version from database"))?;
        ensure!(
            txn_start_version == start_version,
            "invalid start version from database: {} != {}",
            txn_start_version,
            start_version
        );

        let infos = data.proof.transaction_infos;
        let transactions_and_outputs = data.transactions_and_outputs;

        ensure!(
            transactions_and_outputs.len() == infos.len(),
            "invalid data size from database: {}, {}",
            transactions_and_outputs.len(),
            infos.len(),
        );

        transactions_and_outputs
            .into_iter()
            .zip(infos.into_iter())
            .enumerate()
            .map(|(i, ((txn, txn_output), info))| {
                let version = start_version + i as u64;
                let (write_set, events, _, _) = txn_output.unpack();
                self.get_accumulator_root_hash(version)
                    .map(|h| (version, txn, info, events, h, write_set).into())
            })
            .collect()
    }

    pub fn get_account_transactions(
        &self,
        address: AccountAddress,
        start_seq_number: u64,
        limit: u16,
        ledger_version: u64,
    ) -> Result<Vec<TransactionOnChainData>> {
        let txns = self.db.get_account_transactions(
            address,
            start_seq_number,
            limit as u64,
            true,
            ledger_version,
        )?;
        txns.into_inner()
            .into_iter()
            .map(|t| self.convert_into_transaction_on_chain_data(t))
            .collect::<Result<Vec<_>>>()
    }

    pub fn get_transaction_by_hash(
        &self,
        hash: HashValue,
        ledger_version: u64,
    ) -> Result<Option<TransactionOnChainData>> {
        self.db
            .get_transaction_by_hash(hash, ledger_version, true)?
            .map(|t| self.convert_into_transaction_on_chain_data(t))
            .transpose()
    }

    pub async fn get_pending_transaction_by_hash(
        &self,
        hash: HashValue,
    ) -> Result<Option<SignedTransaction>> {
        let (req_sender, callback) = oneshot::channel();

        self.mp_sender
            .clone()
            .send(MempoolClientRequest::GetTransactionByHash(hash, req_sender))
            .await
            .map_err(anyhow::Error::from)?;

        callback.await.map_err(anyhow::Error::from)
    }

    pub fn get_transaction_by_version(
        &self,
        version: u64,
        ledger_version: u64,
    ) -> Result<TransactionOnChainData> {
        self.convert_into_transaction_on_chain_data(self.db.get_transaction_by_version(
            version,
            ledger_version,
            true,
        )?)
    }

    pub fn get_accumulator_root_hash(&self, version: u64) -> Result<HashValue> {
        self.db.get_accumulator_root_hash(version)
    }

    fn convert_into_transaction_on_chain_data(
        &self,
        txn: TransactionWithProof,
    ) -> Result<TransactionOnChainData> {
        // the type is Vec<(Transaction, TransactionOutput)> - given we have one transaction here, there should only ever be one value in this array
        let (_, txn_output) = &self
            .db
            .get_transaction_outputs(txn.version, 1, txn.version)?
            .transactions_and_outputs[0];
        self.get_accumulator_root_hash(txn.version)
            .map(|h| (txn, h, txn_output).into())
    }

    pub fn get_events(
        &self,
        event_key: &EventKey,
        start: u64,
        limit: u16,
        ledger_version: u64,
    ) -> Result<Vec<ContractEvent>> {
        let events = self
            .db
            .get_events(event_key, start, Order::Ascending, limit as u64)?;
        Ok(events
            .into_iter()
            .filter(|(version, _event)| version <= &ledger_version)
            .map(|(_, event)| event)
            .collect::<Vec<_>>())
    }

    pub fn health_check_route(&self) -> BoxedFilter<(impl Reply,)> {
        super::health_check::health_check_route(self.db.clone())
    }
}

// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, bail, ensure, format_err, Context as AnyhowContext, Result};
use aptos_api_types::{BlockInfo, Error, LedgerInfo, TransactionOnChainData, U64};
use aptos_config::config::{NodeConfig, RoleType};
use aptos_crypto::HashValue;
use aptos_mempool::{MempoolClientRequest, MempoolClientSender, SubmissionStatus};
use aptos_state_view::StateView;
use aptos_types::transaction::Transaction;
use aptos_types::{
    account_address::AccountAddress,
    account_state::AccountState,
    chain_id::ChainId,
    contract_event::EventWithVersion,
    event::EventKey,
    ledger_info::LedgerInfoWithSignatures,
    state_store::{state_key::StateKey, state_key_prefix::StateKeyPrefix, state_value::StateValue},
    transaction::{SignedTransaction, TransactionWithProof, Version},
};
use aptos_vm::data_cache::{IntoMoveResolver, RemoteStorageOwned};
use futures::{channel::oneshot, SinkExt};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, convert::Infallible, sync::Arc};
use storage_interface::{
    state_view::{DbStateView, DbStateViewAtVersion, LatestDbStateCheckpointView},
    DbReader, Order,
};
use warp::{filters::BoxedFilter, Filter, Reply};

use crate::poem_backend::{AptosErrorCode, InternalError};

// Context holds application scope context
#[derive(Clone)]
pub struct Context {
    chain_id: ChainId,
    pub db: Arc<dyn DbReader>,
    mp_sender: MempoolClientSender,
    node_config: NodeConfig,
}

impl Context {
    pub fn new(
        chain_id: ChainId,
        db: Arc<dyn DbReader>,
        mp_sender: MempoolClientSender,
        node_config: NodeConfig,
    ) -> Self {
        Self {
            chain_id,
            db,
            mp_sender,
            node_config,
        }
    }

    pub fn move_resolver(&self) -> Result<RemoteStorageOwned<DbStateView>> {
        self.db
            .latest_state_checkpoint_view()
            .map(|state_view| state_view.into_move_resolver())
    }

    pub fn move_resolver_poem<E: InternalError>(
        &self,
    ) -> Result<RemoteStorageOwned<DbStateView>, E> {
        self.move_resolver()
            .context("Failed to read latest state checkpoint from DB")
            .map_err(|e| E::internal(e).error_code(AptosErrorCode::ReadFromStorageError))
    }

    pub fn state_view_at_version(&self, version: Version) -> Result<DbStateView> {
        self.db.state_view_at_version(Some(version))
    }

    pub fn chain_id(&self) -> ChainId {
        self.chain_id
    }

    pub fn node_role(&self) -> RoleType {
        self.node_config.base.role
    }

    pub fn content_length_limit(&self) -> u64 {
        self.node_config.api.content_length_limit()
    }

    pub fn failpoints_enabled(&self) -> bool {
        self.node_config.api.failpoints_enabled
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
        if let Some(oldest_version) = self.db.get_first_txn_version()? {
            Ok(LedgerInfo::new(
                &self.chain_id(),
                &self.get_latest_ledger_info_with_signatures()?,
                oldest_version,
            ))
        } else {
            return Err(anyhow! {"Failed to retrieve oldest version"}.into());
        }
    }

    // TODO: Add error codes to these errors.
    pub fn get_latest_ledger_info_poem<E: InternalError>(&self) -> Result<LedgerInfo, E> {
        if let Some(oldest_version) = self
            .db
            .get_first_txn_version()
            .map_err(|e| E::internal(e).error_code(AptosErrorCode::ReadFromStorageError))?
        {
            Ok(LedgerInfo::new(
                &self.chain_id(),
                &self
                    .get_latest_ledger_info_with_signatures()
                    .map_err(E::internal)?,
                oldest_version,
            ))
        } else {
            Err(E::internal(anyhow!(
                "Failed to retrieve latest ledger info"
            )))
        }
    }

    pub fn get_latest_ledger_info_with_signatures(&self) -> Result<LedgerInfoWithSignatures> {
        self.db.get_latest_ledger_info()
    }

    pub fn get_state_value(&self, state_key: &StateKey, version: u64) -> Result<Option<Vec<u8>>> {
        self.db
            .state_view_at_version(Some(version))?
            .get_state_value(state_key)
    }

    pub fn get_state_value_poem<E: InternalError>(
        &self,
        state_key: &StateKey,
        version: u64,
    ) -> Result<Option<Vec<u8>>, E> {
        self.get_state_value(state_key, version)
            .context("Failed to retrieve state value")
            .map_err(|e| E::internal(e).error_code(AptosErrorCode::ReadFromStorageError))
    }

    pub fn get_state_values(
        &self,
        address: AccountAddress,
        version: u64,
    ) -> Result<HashMap<StateKey, StateValue>> {
        self.db
            .get_state_values_by_key_prefix(&StateKeyPrefix::from(address), version)
    }

    pub fn get_account_state(
        &self,
        address: AccountAddress,
        version: u64,
    ) -> Result<Option<AccountState>> {
        AccountState::from_access_paths_and_values(
            address,
            &self.get_state_values(address, version)?,
        )
    }

    pub fn get_block_timestamp(&self, version: u64) -> Result<u64> {
        self.db.get_block_timestamp(version)
    }

    /// Retrieves information about a block
    pub fn get_block_info(&self, version: u64, ledger_version: u64) -> Result<BlockInfo> {
        let (first_version, last_version, new_block_event) = self.db.get_block_info(version)?;
        ensure!(
            last_version <= ledger_version,
            "Block last version {} for txn version {} < ledger version {}",
            last_version,
            version,
            ledger_version
        );

        let txn_with_proof =
            self.db
                .get_transaction_by_version(first_version, ledger_version, false)?;

        // TODO: embed block hash into the NewBlockEvent
        let block_hash = match &txn_with_proof.transaction {
            Transaction::GenesisTransaction(_) => HashValue::zero(),
            Transaction::BlockMetadata(inner) => inner.id(),
            _ => {
                bail!(
                    "Genesis or BlockMetadata transaction expected at block first version {}",
                    first_version,
                );
            }
        };

        Ok(BlockInfo {
            block_height: new_block_event.height(),
            start_version: first_version,
            end_version: last_version,
            block_hash: block_hash.into(),
            block_timestamp: new_block_event.proposed_time(),
            num_transactions: (last_version + 1 - first_version) as u16,
        })
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
    ) -> Result<Vec<EventWithVersion>> {
        self.db.get_events(
            event_key,
            start,
            Order::Ascending,
            limit as u64,
            ledger_version,
        )
    }

    pub fn health_check_route(&self) -> BoxedFilter<(impl Reply,)> {
        super::health_check::health_check_route(self.db.clone())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockMetadataState {
    epoch_interval: U64,
    height: U64,
}

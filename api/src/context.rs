// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::poem_backend::{AptosErrorResponse, BasicErrorWith404, InternalError, NotFoundError};
use anyhow::{anyhow, ensure, format_err, Context as AnyhowContext, Result};
use aptos_api_types::{
    AptosErrorCode, AsConverter, Block, BlockInfo, Error, LedgerInfo, TransactionOnChainData,
};
use aptos_config::config::{NodeConfig, RoleType};
use aptos_crypto::HashValue;
use aptos_mempool::{MempoolClientRequest, MempoolClientSender, SubmissionStatus};
use aptos_state_view::StateView;
use aptos_types::account_config::NewBlockEvent;
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
use std::{collections::HashMap, convert::Infallible, sync::Arc};
use storage_interface::{
    state_view::{DbStateView, DbStateViewAtVersion, LatestDbStateCheckpointView},
    DbReader, Order,
};
use warp::{filters::BoxedFilter, Filter, Reply};

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
        let maybe_oldest_version = self.db.get_first_viable_txn_version()?;
        let ledger_info = self.get_latest_ledger_info_with_signatures()?;
        let (oldest_version, oldest_block_event) =
            self.db.get_next_block_event(maybe_oldest_version)?;
        let (_, _, newest_block_event) = self
            .db
            .get_block_info_by_version(ledger_info.ledger_info().version())?;
        Ok(LedgerInfo::new(
            &self.chain_id(),
            &ledger_info,
            oldest_version,
            oldest_block_event.height(),
            newest_block_event.height(),
        ))
    }

    // TODO: Add error codes to these errors.
    pub fn get_latest_ledger_info_poem<E: InternalError>(&self) -> Result<LedgerInfo, E> {
        let maybe_oldest_version = self
            .db
            .get_first_viable_txn_version()
            .map_err(|e| E::internal(e).error_code(AptosErrorCode::ReadFromStorageError))?;
        let ledger_info = self
            .get_latest_ledger_info_with_signatures()
            .map_err(E::internal)?;
        let (oldest_version, oldest_block_event) = self
            .db
            .get_next_block_event(maybe_oldest_version)
            .map_err(|e| E::internal(e).error_code(AptosErrorCode::ReadFromStorageError))?;
        let (_, _, newest_block_event) = self
            .db
            .get_block_info_by_version(ledger_info.ledger_info().version())
            .map_err(|e| E::internal(e).error_code(AptosErrorCode::ReadFromStorageError))?;

        Ok(LedgerInfo::new(
            &self.chain_id(),
            &ledger_info,
            oldest_version,
            oldest_block_event.height(),
            newest_block_event.height(),
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
        let (first_version, last_version, new_block_event) =
            self.db.get_block_info_by_version(version)?;
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
        let (block_hash, timestamp) =
            get_block_hash_and_timestamp(&txn_with_proof.transaction, first_version)?;

        Ok(BlockInfo {
            block_height: new_block_event.height(),
            start_version: first_version,
            end_version: last_version,
            block_hash: block_hash.into(),
            block_timestamp: timestamp,
            num_transactions: (last_version + 1 - first_version) as u16,
        })
    }

    pub fn get_block_by_height(
        &self,
        height: u64,
        ledger_version: u64,
        with_transactions: bool,
    ) -> Result<Block, BasicErrorWith404> {
        let (first_version, last_version, new_block_event) = self
            .db
            .get_block_info_by_height(height)
            .context("Failed to find block")
            .map_err(BasicErrorWith404::not_found)?;

        self.get_block(
            ledger_version,
            with_transactions,
            first_version,
            last_version,
            new_block_event,
        )
    }

    pub fn get_block_by_version(
        &self,
        version: u64,
        ledger_version: u64,
        with_transactions: bool,
    ) -> Result<Block, BasicErrorWith404> {
        let (first_version, last_version, new_block_event) = self
            .db
            .get_block_info_by_version(version)
            .context("Failed to find block")
            .map_err(BasicErrorWith404::not_found)?;

        self.get_block(
            ledger_version,
            with_transactions,
            first_version,
            last_version,
            new_block_event,
        )
    }

    fn get_block(
        &self,
        ledger_version: Version,
        with_transactions: bool,
        first_version: Version,
        last_version: Version,
        new_block_event: NewBlockEvent,
    ) -> Result<Block, BasicErrorWith404> {
        if last_version > ledger_version {
            return Err(BasicErrorWith404::not_found(anyhow!("Block not found")));
        }

        let (block_hash, timestamp, txns) = if with_transactions {
            let txns = self
                .get_transactions(
                    first_version,
                    (last_version - first_version + 1) as u16,
                    ledger_version,
                )
                .context("Failed to read raw transactions from storage")
                .map_err(BasicErrorWith404::internal)
                .map_err(|e| e.error_code(AptosErrorCode::InvalidBcsInStorageError))?;

            // TODO: embed block hash into the NewBlockEvent
            let (block_hash, timestamp) = if let Some(txn) = txns.first() {
                get_block_hash_and_timestamp(&txn.transaction, first_version)
                    .map_err(BasicErrorWith404::internal)?
            } else {
                return Err(BasicErrorWith404::internal(anyhow!(
                    "No transactions found for block"
                )));
            };
            (block_hash, timestamp, Some(txns))
        } else {
            let txn = self
                .get_transaction_by_version(first_version, ledger_version)
                .context("Failed to read raw transactions from storage")
                .map_err(BasicErrorWith404::internal)
                .map_err(|e| e.error_code(AptosErrorCode::InvalidBcsInStorageError))?;
            let (block_hash, timestamp) =
                get_block_hash_and_timestamp(&txn.transaction, first_version)
                    .map_err(BasicErrorWith404::internal)?;
            (block_hash, timestamp, None)
        };

        let transactions = if let Some(inner) = txns {
            Some(self.render_transactions(inner, timestamp)?)
        } else {
            None
        };

        Ok(Block {
            block_height: new_block_event.height().into(),
            block_hash: block_hash.into(),
            block_timestamp: new_block_event.proposed_time().into(),
            first_version: first_version.into(),
            last_version: last_version.into(),
            transactions,
        })
    }

    pub fn render_transactions<E: InternalError>(
        &self,
        data: Vec<TransactionOnChainData>,
        timestamp: u64,
    ) -> Result<Vec<aptos_api_types::Transaction>, E> {
        if data.is_empty() {
            return Ok(vec![]);
        }

        let resolver = self.move_resolver_poem()?;
        let converter = resolver.as_converter(self.db.clone());
        let txns: Vec<aptos_api_types::Transaction> = data
            .into_iter()
            .map(|t| {
                let txn = converter.try_into_onchain_transaction(timestamp, t)?;
                Ok(txn)
            })
            .collect::<Result<_, anyhow::Error>>()
            .context("Failed to convert transaction data from storage")
            .map_err(E::internal)?;

        Ok(txns)
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
        start: Option<u64>,
        limit: u16,
        ledger_version: u64,
    ) -> Result<Vec<EventWithVersion>> {
        if let Some(start) = start {
            self.db.get_events(
                event_key,
                start,
                Order::Ascending,
                limit as u64,
                ledger_version,
            )
        } else {
            self.db
                .get_events(
                    event_key,
                    u64::MAX,
                    Order::Descending,
                    limit as u64,
                    ledger_version,
                )
                .map(|mut result| {
                    result.reverse();
                    result
                })
        }
    }

    pub fn health_check_route(&self) -> BoxedFilter<(impl Reply,)> {
        super::health_check::health_check_route(self.db.clone())
    }
}

pub fn get_block_hash_and_timestamp(txn: &Transaction, version: u64) -> Result<(HashValue, u64)> {
    match txn {
        Transaction::GenesisTransaction(_) => Ok((HashValue::zero(), 0)),
        Transaction::BlockMetadata(ref inner) => Ok((inner.id(), inner.timestamp_usecs())),
        _ => {
            return Err(anyhow!(
                "Genesis or BlockMetadata transaction expected at block first version {}",
                version,
            ))
        }
    }
}

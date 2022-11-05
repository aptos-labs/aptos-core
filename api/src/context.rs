// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::accept_type::AcceptType;
use crate::response::{
    bcs_api_disabled, block_not_found_by_height, block_not_found_by_version,
    block_pruned_by_height, json_api_disabled, version_not_found, version_pruned, ForbiddenError,
    InternalError, NotFoundError, ServiceUnavailableError, StdApiError,
};
use anyhow::{bail, ensure, format_err, Context as AnyhowContext, Result};
use aptos_api_types::{
    AptosErrorCode, AsConverter, BcsBlock, GasEstimation, LedgerInfo, TransactionOnChainData,
};
use aptos_config::config::{NodeConfig, RoleType};
use aptos_crypto::HashValue;
use aptos_gas::{AptosGasParameters, FromOnChainGasSchedule};
use aptos_logger::error;
use aptos_mempool::{MempoolClientRequest, MempoolClientSender, SubmissionStatus};
use aptos_state_view::StateView;
use aptos_types::access_path::{AccessPath, Path};
use aptos_types::account_config::NewBlockEvent;
use aptos_types::account_view::AccountView;
use aptos_types::on_chain_config::{GasSchedule, GasScheduleV2, OnChainConfig};
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
use aptos_vm::data_cache::{IntoMoveResolver, StorageAdapter, StorageAdapterOwned};
use futures::{channel::oneshot, SinkExt};
use itertools::Itertools;
use move_core_types::language_storage::{ModuleId, StructTag};
use std::sync::RwLock;
use std::{collections::HashMap, sync::Arc};
use storage_interface::{
    state_view::{DbStateView, DbStateViewAtVersion, LatestDbStateCheckpointView},
    DbReader, Order, MAX_REQUEST_LIMIT,
};

// Context holds application scope context
#[derive(Clone)]
pub struct Context {
    chain_id: ChainId,
    pub db: Arc<dyn DbReader>,
    mp_sender: MempoolClientSender,
    pub node_config: NodeConfig,
    gas_estimation: Arc<RwLock<GasEstimationCache>>,
    gas_schedule_cache: Arc<RwLock<GasScheduleCache>>,
}

impl std::fmt::Debug for Context {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Context<chain_id: {}>", self.chain_id)
    }
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
            gas_estimation: Arc::new(RwLock::new(GasEstimationCache {
                last_updated_version: None,
                last_updated_epoch: None,
                deprioritized_gas_price: 0,
                median_gas_price: 0,
                prioritized_gas_price: 0,
            })),
            gas_schedule_cache: Arc::new(RwLock::new(GasScheduleCache {
                last_updated_epoch: None,
                gas_schedule_params: None,
            })),
        }
    }

    pub fn max_transactions_page_size(&self) -> u16 {
        self.node_config.api.max_transactions_page_size
    }

    pub fn max_events_page_size(&self) -> u16 {
        self.node_config.api.max_events_page_size
    }

    pub fn max_account_resources_page_size(&self) -> u16 {
        self.node_config.api.max_account_resources_page_size
    }

    pub fn max_account_modules_page_size(&self) -> u16 {
        self.node_config.api.max_account_modules_page_size
    }

    pub fn move_resolver(&self) -> Result<StorageAdapterOwned<DbStateView>> {
        self.db
            .latest_state_checkpoint_view()
            .map(|state_view| state_view.into_move_resolver())
    }

    pub fn move_resolver_poem<E: InternalError>(
        &self,
        ledger_info: &LedgerInfo,
    ) -> Result<StorageAdapterOwned<DbStateView>, E> {
        self.move_resolver()
            .context("Failed to read latest state checkpoint from DB")
            .map_err(|e| E::internal_with_code(e, AptosErrorCode::InternalError, ledger_info))
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

    pub fn max_submit_transaction_batch_size(&self) -> usize {
        self.node_config.api.max_submit_transaction_batch_size
    }

    pub async fn submit_transaction(&self, txn: SignedTransaction) -> Result<SubmissionStatus> {
        let (req_sender, callback) = oneshot::channel();
        self.mp_sender
            .clone()
            .send(MempoolClientRequest::SubmitTransaction(txn, req_sender))
            .await?;

        callback.await?
    }

    // For use from external crates where they don't want to handle
    // the API response error types.
    pub fn get_latest_ledger_info_wrapped(&self) -> anyhow::Result<LedgerInfo> {
        self.get_latest_ledger_info::<crate::response::BasicError>()
            .map_err(|e| e.into())
    }

    pub fn get_latest_ledger_info<E: ServiceUnavailableError>(&self) -> Result<LedgerInfo, E> {
        let maybe_oldest_version = self
            .db
            .get_first_viable_txn_version()
            .context("Failed to retrieve oldest version in DB")
            .map_err(|e| {
                E::service_unavailable_with_code_no_info(e, AptosErrorCode::InternalError)
            })?;
        let ledger_info = self
            .get_latest_ledger_info_with_signatures()
            .context("Failed to retrieve latest ledger info")
            .map_err(|e| {
                E::service_unavailable_with_code_no_info(e, AptosErrorCode::InternalError)
            })?;
        let (oldest_version, oldest_block_event) = self
            .db
            .get_next_block_event(maybe_oldest_version)
            .context("Failed to retrieve oldest block information")
            .map_err(|e| {
                E::service_unavailable_with_code_no_info(e, AptosErrorCode::InternalError)
            })?;
        let (_, _, newest_block_event) = self
            .db
            .get_block_info_by_version(ledger_info.ledger_info().version())
            .context("Failed to retrieve latest block information")
            .map_err(|e| {
                E::service_unavailable_with_code_no_info(e, AptosErrorCode::InternalError)
            })?;

        Ok(LedgerInfo::new(
            &self.chain_id(),
            &ledger_info,
            oldest_version,
            oldest_block_event.height(),
            newest_block_event.height(),
        ))
    }

    pub fn get_latest_ledger_info_and_verify_lookup_version<E: StdApiError>(
        &self,
        requested_ledger_version: Option<Version>,
    ) -> Result<(LedgerInfo, Version), E> {
        let latest_ledger_info = self.get_latest_ledger_info()?;

        let requested_ledger_version =
            requested_ledger_version.unwrap_or_else(|| latest_ledger_info.version());

        // This is too far in the future, a retriable case
        if requested_ledger_version > latest_ledger_info.version() {
            return Err(version_not_found(
                requested_ledger_version,
                &latest_ledger_info,
            ));
        } else if requested_ledger_version < latest_ledger_info.oldest_ledger_version.0 {
            return Err(version_pruned(
                requested_ledger_version,
                &latest_ledger_info,
            ));
        }

        Ok((latest_ledger_info, requested_ledger_version))
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
        ledger_info: &LedgerInfo,
    ) -> Result<Option<Vec<u8>>, E> {
        self.get_state_value(state_key, version)
            .context("Failed to retrieve state value")
            .map_err(|e| E::internal_with_code(e, AptosErrorCode::InternalError, ledger_info))
    }

    pub fn get_state_values(
        &self,
        address: AccountAddress,
        version: u64,
    ) -> Result<HashMap<StateKey, StateValue>> {
        let mut iter = self.db.get_prefixed_state_value_iterator(
            &StateKeyPrefix::from(address),
            None,
            version,
        )?;
        let kvs = iter
            .by_ref()
            .take(MAX_REQUEST_LIMIT as usize)
            .collect::<Result<_>>()?;
        if iter.next().transpose()?.is_some() {
            bail!("Too many state items under account ({:?}).", address);
        }
        Ok(kvs)
    }

    pub fn get_resources_by_pagination(
        &self,
        address: AccountAddress,
        prev_state_key: Option<&StateKey>,
        version: u64,
        limit: u64,
    ) -> Result<(Vec<(StructTag, Vec<u8>)>, Option<StateKey>)> {
        let account_iter = self.db.get_prefixed_state_value_iterator(
            &StateKeyPrefix::from(address),
            prev_state_key,
            version,
        )?;
        let mut resource_iter = account_iter
            .filter_map(|res| match res {
                Ok((k, v)) => match k {
                    StateKey::AccessPath(AccessPath { address: _, path }) => {
                        match Path::try_from(path.as_slice()) {
                            Ok(Path::Resource(struct_tag)) => {
                                Some(Ok((struct_tag, v.into_bytes())))
                            }
                            Ok(Path::Code(_)) => None,
                            Err(e) => Some(Err(anyhow::Error::from(e))),
                        }
                    }
                    _ => {
                        error!("storage prefix scan return inconsistent key ({:?}) with expected key prefix ({:?}).", k, StateKeyPrefix::from(address));
                        Some(Err(format_err!( "storage prefix scan return inconsistent key ({:?})", k )))
                    }
                },
                Err(e) => Some(Err(e)),
            })
            .take(limit as usize + 1);
        let kvs = resource_iter
            .by_ref()
            .take(limit as usize)
            .collect::<Result<_>>()?;
        let next_key = resource_iter.next().transpose()?.map(|(struct_tag, _v)| {
            StateKey::AccessPath(AccessPath::new(
                address,
                AccessPath::resource_path_vec(struct_tag),
            ))
        });
        Ok((kvs, next_key))
    }

    pub fn get_modules_by_pagination(
        &self,
        address: AccountAddress,
        prev_state_key: Option<&StateKey>,
        version: u64,
        limit: u64,
    ) -> Result<(Vec<(ModuleId, Vec<u8>)>, Option<StateKey>)> {
        let account_iter = self.db.get_prefixed_state_value_iterator(
            &StateKeyPrefix::from(address),
            prev_state_key,
            version,
        )?;
        let mut module_iter = account_iter
            .filter_map(|res| match res {
                Ok((k, v)) => match k {
                    StateKey::AccessPath(AccessPath { address: _, path }) => {
                        match Path::try_from(path.as_slice()) {
                            Ok(Path::Code(module_id)) => Some(Ok((module_id, v.into_bytes()))),
                            Ok(Path::Resource(_)) => None,
                            Err(e) => Some(Err(anyhow::Error::from(e))),
                        }
                    }
                    _ => {
                        error!("storage prefix scan return inconsistent key ({:?}) with expected key prefix ({:?}).", k, StateKeyPrefix::from(address));
                        Some(Err(format_err!( "storage prefix scan return inconsistent key ({:?})", k )))
                    }
                },
                Err(e) => Some(Err(e)),
            })
            .take(limit as usize + 1);
        let kvs = module_iter
            .by_ref()
            .take(limit as usize)
            .collect::<Result<_>>()?;
        let next_key = module_iter.next().transpose()?.map(|(module_id, _v)| {
            StateKey::AccessPath(AccessPath::new(
                address,
                AccessPath::code_path_vec(module_id),
            ))
        });
        Ok((kvs, next_key))
    }

    // This function should be deprecated. DO NOT USE it.
    // Instead, call either `get_modules_by_pagination` or `get_modules_by_pagination`.
    pub fn get_account_state<E: InternalError>(
        &self,
        address: AccountAddress,
        version: u64,
        latest_ledger_info: &LedgerInfo,
    ) -> Result<Option<AccountState>, E> {
        AccountState::from_access_paths_and_values(
            address,
            &self.get_state_values(address, version).map_err(|err| {
                E::internal_with_code(err, AptosErrorCode::InternalError, latest_ledger_info)
            })?,
        )
        .context("Failed to read account state at requested version")
        .map_err(|err| {
            E::internal_with_code(err, AptosErrorCode::InternalError, latest_ledger_info)
        })
    }

    pub fn get_block_timestamp<E: InternalError>(
        &self,
        ledger_info: &LedgerInfo,
        version: u64,
    ) -> Result<u64, E> {
        self.db
            .get_block_timestamp(version)
            .context("Failed to retrieve block timestamp")
            .map_err(|err| E::internal_with_code(err, AptosErrorCode::InternalError, ledger_info))
    }

    pub fn get_block_by_height<E: StdApiError>(
        &self,
        height: u64,
        latest_ledger_info: &LedgerInfo,
        with_transactions: bool,
    ) -> Result<BcsBlock, E> {
        if height < latest_ledger_info.oldest_block_height.0 {
            return Err(block_pruned_by_height(height, latest_ledger_info));
        } else if height > latest_ledger_info.block_height.0 {
            return Err(block_not_found_by_height(height, latest_ledger_info));
        }

        let (first_version, last_version, new_block_event) = self
            .db
            .get_block_info_by_height(height)
            .map_err(|_| block_not_found_by_height(height, latest_ledger_info))?;

        self.get_block(
            latest_ledger_info,
            with_transactions,
            first_version,
            last_version,
            new_block_event,
        )
    }

    pub fn get_block_by_version<E: StdApiError>(
        &self,
        version: u64,
        latest_ledger_info: &LedgerInfo,
        with_transactions: bool,
    ) -> Result<BcsBlock, E> {
        if version < latest_ledger_info.oldest_ledger_version.0 {
            return Err(version_pruned(version, latest_ledger_info));
        } else if version > latest_ledger_info.version() {
            return Err(version_not_found(version, latest_ledger_info));
        }

        let (first_version, last_version, new_block_event) = self
            .db
            .get_block_info_by_version(version)
            .map_err(|_| block_not_found_by_version(version, latest_ledger_info))?;

        self.get_block(
            latest_ledger_info,
            with_transactions,
            first_version,
            last_version,
            new_block_event,
        )
    }

    fn get_block<E: StdApiError>(
        &self,
        latest_ledger_info: &LedgerInfo,
        with_transactions: bool,
        first_version: Version,
        last_version: Version,
        new_block_event: NewBlockEvent,
    ) -> Result<BcsBlock, E> {
        let ledger_version = latest_ledger_info.ledger_version.0;

        // We can't pull a block in the future, but this shouldn't happen
        if last_version > ledger_version {
            return Err(block_not_found_by_height(
                new_block_event.height(),
                latest_ledger_info,
            ));
        }

        let block_hash = new_block_event
            .hash()
            .context("Failed to parse block hash")
            .map_err(|err| {
                E::internal_with_code(err, AptosErrorCode::InternalError, latest_ledger_info)
            })?;
        let block_timestamp = new_block_event.proposed_time();

        // We can only get the max_transactions page size
        let max_txns = std::cmp::min(
            self.node_config.api.max_transactions_page_size,
            (last_version - first_version + 1) as u16,
        );
        let txns = if with_transactions {
            Some(
                self.get_transactions(first_version, max_txns, ledger_version)
                    .context("Failed to read raw transactions from storage")
                    .map_err(|err| {
                        E::internal_with_code(
                            err,
                            AptosErrorCode::InternalError,
                            latest_ledger_info,
                        )
                    })?,
            )
        } else {
            None
        };

        Ok(BcsBlock {
            block_height: new_block_event.height(),
            block_hash,
            block_timestamp,
            first_version,
            last_version,
            transactions: txns,
        })
    }

    pub fn render_transactions_sequential<E: InternalError>(
        &self,
        ledger_info: &LedgerInfo,
        data: Vec<TransactionOnChainData>,
        mut timestamp: u64,
    ) -> Result<Vec<aptos_api_types::Transaction>, E> {
        if data.is_empty() {
            return Ok(vec![]);
        }

        let resolver = self.move_resolver_poem(ledger_info)?;
        let converter = resolver.as_converter(self.db.clone());
        let txns: Vec<aptos_api_types::Transaction> = data
            .into_iter()
            .map(|t| {
                // Update the timestamp if the next block occurs
                if let Transaction::BlockMetadata(ref txn) = t.transaction {
                    timestamp = txn.timestamp_usecs();
                }
                let txn = converter.try_into_onchain_transaction(timestamp, t)?;
                Ok(txn)
            })
            .collect::<Result<_, anyhow::Error>>()
            .context("Failed to convert transaction data from storage")
            .map_err(|err| {
                E::internal_with_code(err, AptosErrorCode::InternalError, ledger_info)
            })?;

        Ok(txns)
    }

    pub fn render_transactions_non_sequential<E: InternalError>(
        &self,
        ledger_info: &LedgerInfo,
        data: Vec<TransactionOnChainData>,
    ) -> Result<Vec<aptos_api_types::Transaction>, E> {
        if data.is_empty() {
            return Ok(vec![]);
        }

        let resolver = self.move_resolver_poem(ledger_info)?;
        let converter = resolver.as_converter(self.db.clone());
        let txns: Vec<aptos_api_types::Transaction> = data
            .into_iter()
            .map(|t| {
                let timestamp = self.db.get_block_timestamp(t.version)?;
                let txn = converter.try_into_onchain_transaction(timestamp, t)?;
                Ok(txn)
            })
            .collect::<Result<_, anyhow::Error>>()
            .context("Failed to convert transaction data from storage")
            .map_err(|err| {
                E::internal_with_code(err, AptosErrorCode::InternalError, ledger_info)
            })?;

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

    pub fn get_account_transactions<E: NotFoundError + InternalError>(
        &self,
        address: AccountAddress,
        start_seq_number: Option<u64>,
        limit: u16,
        ledger_version: u64,
        ledger_info: &LedgerInfo,
    ) -> Result<Vec<TransactionOnChainData>, E> {
        let start_seq_number = if let Some(start_seq_number) = start_seq_number {
            start_seq_number
        } else {
            // Get the current account state, and get the sequence number to get the limit most
            // recent transactions
            let account_state = self
                .get_account_state(address, ledger_info.version(), ledger_info)?
                .ok_or_else(|| {
                    E::not_found_with_code(
                        "Account not found",
                        AptosErrorCode::AccountNotFound,
                        ledger_info,
                    )
                })?;
            let resource = account_state
                .get_account_resource()
                .map_err(|err| {
                    E::internal_with_code(
                        format!("Failed to get account resource {}", err),
                        AptosErrorCode::InternalError,
                        ledger_info,
                    )
                })?
                .ok_or_else(|| {
                    E::not_found_with_code(
                        "Account not found",
                        AptosErrorCode::AccountNotFound,
                        ledger_info,
                    )
                })?;

            resource.sequence_number().saturating_sub(limit as u64)
        };

        let txns = self
            .db
            .get_account_transactions(
                address,
                start_seq_number,
                limit as u64,
                true,
                ledger_version,
            )
            .context("Failed to retrieve account transactions")
            .map_err(|err| {
                E::internal_with_code(err, AptosErrorCode::InternalError, ledger_info)
            })?;
        txns.into_inner()
            .into_iter()
            .map(|t| self.convert_into_transaction_on_chain_data(t))
            .collect::<Result<Vec<_>>>()
            .context("Failed to parse account transactions")
            .map_err(|err| E::internal_with_code(err, AptosErrorCode::InternalError, ledger_info))
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

    pub fn estimate_gas_price<E: InternalError>(
        &self,
        ledger_info: &LedgerInfo,
    ) -> Result<GasEstimation, E> {
        // The search size
        const SEARCH_SIZE: u64 = 100_000;
        let oldest_search_version = std::cmp::max(
            ledger_info.ledger_version.0.saturating_sub(SEARCH_SIZE),
            ledger_info.oldest_ledger_version.0,
        );

        // If it's cached, let's use that
        {
            let gas_estimation = self.gas_estimation.read().unwrap();
            let (last_updated_epoch, _) = self.get_gas_schedule(ledger_info)?;

            // Cache includes if there was an update on the gas schedule due to epoch changes
            if gas_estimation.last_updated_version.is_some()
                && gas_estimation.last_updated_version.unwrap() > oldest_search_version
                && last_updated_epoch == gas_estimation.last_updated_epoch.unwrap_or_default()
            {
                return Ok(gas_estimation.gas_estimate());
            }
        }

        // Otherwise, get the estimated amount from storage
        {
            let mut gas_estimation = self.gas_estimation.write().unwrap();
            let (last_updated_epoch, gas_schedule) = self.get_gas_schedule(ledger_info)?;

            // If this has been updated by a different thread, use that instead
            if gas_estimation.last_updated_version.is_some()
                && gas_estimation.last_updated_version.unwrap() > oldest_search_version
                && last_updated_epoch == gas_estimation.last_updated_epoch.unwrap_or_default()
            {
                return Ok(gas_estimation.gas_estimate());
            }
            let mut gas_prices: Vec<u64> = self
                .db
                .get_gas_prices(
                    oldest_search_version,
                    SEARCH_SIZE,
                    ledger_info.ledger_version.0,
                )
                .map_err(|err| {
                    E::internal_with_code(err, AptosErrorCode::InternalError, ledger_info)
                })?;

            // When there's no gas prices in the last 100k transactions, we're going to set it to
            // the lowest gas price because the transaction should get through with any amount
            gas_estimation.last_updated_version = Some(ledger_info.ledger_version.0);
            gas_estimation.last_updated_epoch = Some(last_updated_epoch);
            let min_gas_price = gas_schedule.txn.min_price_per_gas_unit.into();
            let max_gas_price = gas_schedule.txn.max_price_per_gas_unit.into();

            let observed_gas_median = if gas_prices.is_empty() {
                // This will make it always pick the minimum gas price
                0
            } else {
                // Sort and take the median of the gas prices
                gas_prices.sort();

                // Median is the center if it's odd, average of the two middle if even
                let mid = gas_prices.len() / 2;
                if gas_prices.len() % 2 == 0 {
                    let lower = gas_prices.get(mid.saturating_sub(1)).unwrap();
                    let upper = gas_prices.get(mid).unwrap();

                    upper.saturating_add(*lower).saturating_div(2)
                } else {
                    *gas_prices.get(mid).unwrap()
                }
            };

            // Ensure the price is within the min/max bounds
            gas_estimation.median_gas_price = std::cmp::min(
                max_gas_price,
                std::cmp::max(min_gas_price, observed_gas_median),
            );

            // Handle prices for "prioritized" and "deprioritized"
            // There must be at least 1 buckets
            assert!(!self.node_config.mempool.broadcast_buckets.is_empty());
            let median_gas_price = gas_estimation.median_gas_price;

            // TODO: Buckets are assumed to be ordered
            let bucket_index = self
                .node_config
                .mempool
                .broadcast_buckets
                .iter()
                .sorted()
                .enumerate()
                .find_map(|(index, current_bucket)| {
                    if median_gas_price >= *current_bucket {
                        Some(index)
                    } else {
                        None
                    }
                })
                .unwrap_or_default();

            // Previous bucket could be the same as the minimum price (if too low)
            let previous_bucket_price = self
                .node_config
                .mempool
                .broadcast_buckets
                .get(bucket_index.saturating_sub(1))
                .copied()
                .unwrap_or(min_gas_price);
            // Next bucket could be the same as the current gas price, but we increase by 1 for the estimation
            let next_bucket_price = self
                .node_config
                .mempool
                .broadcast_buckets
                .get(bucket_index.saturating_add(1))
                .copied()
                .unwrap_or_else(|| median_gas_price.saturating_add(1));
            // Ensure that bucket prices are within the bounds
            gas_estimation.deprioritized_gas_price =
                std::cmp::max(min_gas_price, previous_bucket_price);
            gas_estimation.prioritized_gas_price = std::cmp::min(max_gas_price, next_bucket_price);

            Ok(gas_estimation.gas_estimate())
        }
    }

    pub fn get_gas_schedule<E: InternalError>(
        &self,
        ledger_info: &LedgerInfo,
    ) -> Result<(u64, AptosGasParameters), E> {
        // If it's the same epoch, used the cached results
        {
            let cache = self.gas_schedule_cache.read().unwrap();
            if let (Some(ref last_updated_epoch), Some(gas_params)) =
                (cache.last_updated_epoch, &cache.gas_schedule_params)
            {
                if *last_updated_epoch == ledger_info.epoch.0 {
                    return Ok((
                        cache.last_updated_epoch.unwrap_or_default(),
                        gas_params.clone(),
                    ));
                }
            }
        }

        // Otherwise refresh the cache
        {
            let mut cache = self.gas_schedule_cache.write().unwrap();
            // If a different thread updated the cache, we can exit early
            if let (Some(ref last_updated_epoch), Some(gas_params)) =
                (cache.last_updated_epoch, &cache.gas_schedule_params)
            {
                if *last_updated_epoch == ledger_info.epoch.0 {
                    return Ok((
                        cache.last_updated_epoch.unwrap_or_default(),
                        gas_params.clone(),
                    ));
                }
            }

            // Retrieve the gas schedule from storage and parse it accordingly
            let state_view = self
                .db
                .state_view_at_version(Some(ledger_info.version()))
                .map_err(|e| {
                    E::internal_with_code(e, AptosErrorCode::InternalError, ledger_info)
                })?;
            let storage_adapter = StorageAdapter::new(&state_view);

            let gas_schedule_params =
                match GasScheduleV2::fetch_config(&storage_adapter).and_then(|gas_schedule| {
                    let gas_schedule = gas_schedule.to_btree_map();
                    AptosGasParameters::from_on_chain_gas_schedule(&gas_schedule)
                }) {
                    Some(gas_schedule) => Ok(gas_schedule),
                    None => GasSchedule::fetch_config(&storage_adapter)
                        .and_then(|gas_schedule| {
                            let gas_schedule = gas_schedule.to_btree_map();
                            AptosGasParameters::from_on_chain_gas_schedule(&gas_schedule)
                        })
                        .ok_or_else(|| {
                            E::internal_with_code(
                                "Failed to retrieve gas schedule",
                                AptosErrorCode::InternalError,
                                ledger_info,
                            )
                        }),
                }?;

            // Update the cache
            cache.gas_schedule_params = Some(gas_schedule_params.clone());
            cache.last_updated_epoch = Some(ledger_info.epoch.0);
            Ok((
                cache.last_updated_epoch.unwrap_or_default(),
                gas_schedule_params,
            ))
        }
    }

    pub fn check_api_output_enabled<E: ForbiddenError>(
        &self,
        api_name: &'static str,
        accept_type: &AcceptType,
    ) -> Result<(), E> {
        match accept_type {
            AcceptType::Json => {
                if !self.node_config.api.json_output_enabled {
                    return Err(json_api_disabled(api_name));
                }
            }
            AcceptType::Bcs => {
                if !self.node_config.api.bcs_output_enabled {
                    return Err(bcs_api_disabled(api_name));
                }
            }
        }
        Ok(())
    }

    pub fn last_updated_gas_schedule(&self) -> Option<u64> {
        self.gas_schedule_cache.read().unwrap().last_updated_epoch
    }

    pub fn last_updated_gas_estimation(&self) -> Option<u64> {
        self.gas_estimation.read().unwrap().last_updated_version
    }
}

pub struct GasEstimationCache {
    last_updated_version: Option<u64>,
    last_updated_epoch: Option<u64>,
    deprioritized_gas_price: u64,
    median_gas_price: u64,
    prioritized_gas_price: u64,
}

impl GasEstimationCache {
    pub fn gas_estimate(&self) -> GasEstimation {
        GasEstimation {
            deprioritized_gas_estimate: Some(self.deprioritized_gas_price),
            gas_estimate: self.median_gas_price,
            prioritized_gas_estimate: Some(self.prioritized_gas_price),
        }
    }
}

pub struct GasScheduleCache {
    last_updated_epoch: Option<u64>,
    gas_schedule_params: Option<AptosGasParameters>,
}

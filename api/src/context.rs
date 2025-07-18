// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    accept_type::AcceptType,
    metrics,
    response::{
        bcs_api_disabled, block_not_found_by_height, block_not_found_by_version,
        block_pruned_by_height, json_api_disabled, version_not_found, version_pruned,
        ForbiddenError, InternalError, NotFoundError, ServiceUnavailableError, StdApiError,
    },
};
use anyhow::{anyhow, bail, ensure, format_err, Context as AnyhowContext, Result};
use aptos_api_types::{
    transaction::ReplayProtector, AptosErrorCode, AsConverter, BcsBlock, GasEstimation, LedgerInfo,
    ResourceGroup, TransactionOnChainData, TransactionSummary,
};
use aptos_config::config::{GasEstimationConfig, NodeConfig, RoleType};
use aptos_crypto::HashValue;
use aptos_gas_schedule::{AptosGasParameters, FromOnChainGasSchedule};
use aptos_logger::{error, info, Schema};
use aptos_mempool::{MempoolClientRequest, MempoolClientSender, SubmissionStatus};
use aptos_storage_interface::{
    state_store::state_view::db_state_view::{
        DbStateView, DbStateViewAtVersion, LatestDbStateCheckpointView,
    },
    AptosDbError, DbReader, Order, MAX_REQUEST_LIMIT,
};
use aptos_types::{
    access_path::{AccessPath, Path},
    account_address::AccountAddress,
    account_config::{AccountResource, NewBlockEvent},
    chain_id::ChainId,
    contract_event::{ContractEvent, ContractEventV1, EventWithVersion},
    event::EventKey,
    indexer::indexer_db_reader::IndexerReader,
    ledger_info::LedgerInfoWithSignatures,
    on_chain_config::{
        FeatureFlag, Features, GasSchedule, GasScheduleV2, OnChainConfig, OnChainExecutionConfig,
    },
    state_store::{
        state_key::{inner::StateKeyInner, prefix::StateKeyPrefix, StateKey},
        state_value::StateValue,
        TStateView,
    },
    transaction::{
        block_epilogue::BlockEndInfo,
        use_case::{UseCaseAwareTransaction, UseCaseKey},
        IndexedTransactionSummary, SignedTransaction, Transaction, TransactionWithProof, Version,
    },
};
use futures::{channel::oneshot, SinkExt};
use mini_moka::sync::Cache;
use move_core_types::{
    identifier::Identifier,
    language_storage::{ModuleId, StructTag},
    move_resource::MoveResource,
};
use serde::Serialize;
use std::{
    cmp::Reverse,
    collections::{BTreeMap, HashMap},
    ops::{Bound::Included, Deref},
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc, RwLock, RwLockWriteGuard,
    },
    time::Instant,
};

// Context holds application scope context
#[derive(Clone)]
pub struct Context {
    chain_id: ChainId,
    pub db: Arc<dyn DbReader>,
    mp_sender: MempoolClientSender,
    pub node_config: Arc<NodeConfig>,
    gas_schedule_cache: Arc<RwLock<GasScheduleCache>>,
    gas_estimation_cache: Arc<RwLock<GasEstimationCache>>,
    gas_limit_cache: Arc<RwLock<GasLimitCache>>,
    view_function_stats: Arc<FunctionStats>,
    simulate_txn_stats: Arc<FunctionStats>,
    pub indexer_reader: Option<Arc<dyn IndexerReader>>,
    pub wait_for_hash_active_connections: Arc<AtomicUsize>,
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
        indexer_reader: Option<Arc<dyn IndexerReader>>,
    ) -> Self {
        let (view_function_stats, simulate_txn_stats) = {
            let log_per_call_stats = node_config.api.periodic_function_stats_sec.is_some();
            (
                Arc::new(FunctionStats::new(
                    FunctionType::ViewFunction,
                    log_per_call_stats,
                )),
                Arc::new(FunctionStats::new(
                    FunctionType::TxnSimulation,
                    log_per_call_stats,
                )),
            )
        };
        Self {
            chain_id,
            db,
            mp_sender,
            node_config: Arc::new(node_config),
            gas_schedule_cache: Arc::new(RwLock::new(GasScheduleCache {
                last_updated_epoch: None,
                gas_schedule_params: None,
            })),
            gas_estimation_cache: Arc::new(RwLock::new(GasEstimationCache {
                last_updated_epoch: None,
                last_updated_time: None,
                estimation: None,
                min_inclusion_prices: BTreeMap::new(),
            })),
            gas_limit_cache: Arc::new(RwLock::new(GasLimitCache {
                last_updated_epoch: None,
                execution_onchain_config: OnChainExecutionConfig::default_if_missing(),
            })),
            view_function_stats,
            simulate_txn_stats,
            indexer_reader,
            wait_for_hash_active_connections: Arc::new(AtomicUsize::new(0)),
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

    pub fn latest_state_view(&self) -> Result<DbStateView> {
        Ok(self.db.latest_state_checkpoint_view()?)
    }

    pub fn latest_state_view_poem<E: InternalError>(
        &self,
        ledger_info: &LedgerInfo,
    ) -> Result<DbStateView, E> {
        self.db
            .latest_state_checkpoint_view()
            .context("Failed to read latest state checkpoint from DB")
            .map_err(|e| E::internal_with_code(e, AptosErrorCode::InternalError, ledger_info))
    }

    pub fn feature_enabled(&self, feature: FeatureFlag) -> Result<bool> {
        let state_view = self.latest_state_view()?;
        let features = Features::fetch_config(&state_view)
            .ok_or_else(|| anyhow::anyhow!("Failed to fetch features from state view"))?;
        Ok(features.is_enabled(feature))
    }

    pub fn state_view<E: StdApiError>(
        &self,
        requested_ledger_version: Option<u64>,
    ) -> Result<(LedgerInfo, u64, DbStateView), E> {
        let (latest_ledger_info, requested_ledger_version) =
            self.get_latest_ledger_info_and_verify_lookup_version(requested_ledger_version)?;

        let state_view = self
            .state_view_at_version(requested_ledger_version)
            .map_err(|err| {
                E::internal_with_code(err, AptosErrorCode::InternalError, &latest_ledger_info)
            })?;

        Ok((latest_ledger_info, requested_ledger_version, state_view))
    }

    pub fn state_view_at_version(&self, version: Version) -> Result<DbStateView> {
        Ok(self.db.state_view_at_version(Some(version))?)
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

    pub fn get_oldest_version_and_block_height<E: ServiceUnavailableError>(
        &self,
    ) -> Result<(Version, u64), E> {
        self.db
            .get_first_viable_block()
            .context("Failed to retrieve oldest block information")
            .map_err(|e| E::service_unavailable_with_code_no_info(e, AptosErrorCode::InternalError))
    }

    pub fn get_latest_storage_ledger_info<E: ServiceUnavailableError>(
        &self,
    ) -> Result<LedgerInfo, E> {
        let ledger_info = self
            .get_latest_ledger_info_with_signatures()
            .context("Failed to retrieve latest ledger info")
            .map_err(|e| {
                E::service_unavailable_with_code_no_info(e, AptosErrorCode::InternalError)
            })?;

        let (oldest_version, oldest_block_height) = self.get_oldest_version_and_block_height()?;
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
            oldest_block_height,
            newest_block_event.height(),
        ))
    }

    pub fn get_latest_ledger_info<E: ServiceUnavailableError>(&self) -> Result<LedgerInfo, E> {
        if let Some(indexer_reader) = self.indexer_reader.as_ref() {
            if indexer_reader.is_internal_indexer_enabled() {
                return self.get_latest_internal_indexer_ledger_info();
            }
        }
        self.get_latest_storage_ledger_info()
    }

    pub fn get_latest_internal_and_storage_ledger_info<E: ServiceUnavailableError>(
        &self,
    ) -> Result<(Option<LedgerInfo>, LedgerInfo), E> {
        if let Some(indexer_reader) = self.indexer_reader.as_ref() {
            if indexer_reader.is_internal_indexer_enabled() {
                return Ok((
                    Some(self.get_latest_internal_indexer_ledger_info()?),
                    self.get_latest_storage_ledger_info()?,
                ));
            }
        }
        Ok((None, self.get_latest_storage_ledger_info()?))
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

    pub fn get_latest_internal_indexer_ledger_info<E: ServiceUnavailableError>(
        &self,
    ) -> Result<LedgerInfo, E> {
        if let Some(indexer_reader) = self.indexer_reader.as_ref() {
            if indexer_reader.is_internal_indexer_enabled() {
                if let Some(mut latest_version) = indexer_reader
                    .get_latest_internal_indexer_ledger_version()
                    .map_err(|err| {
                        E::service_unavailable_with_code_no_info(err, AptosErrorCode::InternalError)
                    })?
                {
                    // The internal indexer version can be ahead of the storage committed version since it syncs to db's latest synced version
                    let last_storage_version =
                        self.get_latest_storage_ledger_info()?.ledger_version.0;
                    latest_version = std::cmp::min(latest_version, last_storage_version);
                    let (_, block_end_version, new_block_event) = self
                        .db
                        .get_block_info_by_version(latest_version)
                        .map_err(|_| {
                            E::service_unavailable_with_code_no_info(
                                "Failed to get block",
                                AptosErrorCode::InternalError,
                            )
                        })?;
                    let (oldest_version, oldest_block_height) =
                        self.get_oldest_version_and_block_height()?;
                    return Ok(LedgerInfo::new_ledger_info(
                        &self.chain_id(),
                        new_block_event.epoch(),
                        block_end_version,
                        oldest_version,
                        oldest_block_height,
                        new_block_event.height(),
                        new_block_event.proposed_time(),
                    ));
                } else {
                    // Indexer doesn't have data yet as DB is boostrapping.
                    return Err(E::service_unavailable_with_code_no_info(
                        "DB is bootstrapping",
                        AptosErrorCode::InternalError,
                    ));
                }
            }
        }

        Err(E::service_unavailable_with_code_no_info(
            "Indexer reader doesn't exist",
            AptosErrorCode::InternalError,
        ))
    }

    pub fn get_latest_ledger_info_with_signatures(&self) -> Result<LedgerInfoWithSignatures> {
        Ok(self.db.get_latest_ledger_info()?)
    }

    pub fn get_state_value(&self, state_key: &StateKey, version: u64) -> Result<Option<Vec<u8>>> {
        Ok(self
            .db
            .state_view_at_version(Some(version))?
            .get_state_value_bytes(state_key)?
            .map(|val| val.to_vec()))
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

    pub fn get_resource<T: MoveResource>(
        &self,
        address: AccountAddress,
        version: Version,
    ) -> Result<Option<T>> {
        let bytes_opt = self.get_state_value(&StateKey::resource_typed::<T>(&address)?, version)?;
        bytes_opt
            .map(|bytes| bcs::from_bytes(&bytes))
            .transpose()
            .map_err(|err| anyhow!(format!("Failed to deserialize resource: {}", err)))
    }

    pub fn get_resource_poem<T: MoveResource, E: InternalError>(
        &self,
        address: AccountAddress,
        version: Version,
        latest_ledger_info: &LedgerInfo,
    ) -> Result<Option<T>, E> {
        self.get_resource(address, version)
            .context("Failed to read account resource.")
            .map_err(|err| {
                E::internal_with_code(err, AptosErrorCode::InternalError, latest_ledger_info)
            })
    }

    pub fn expect_resource_poem<T: MoveResource, E: InternalError + NotFoundError>(
        &self,
        address: AccountAddress,
        version: Version,
        latest_ledger_info: &LedgerInfo,
    ) -> Result<T, E> {
        self.get_resource_poem(address, version, latest_ledger_info)?
            .ok_or_else(|| {
                E::not_found_with_code(
                    format!(
                        "{} not found under address {}",
                        T::struct_identifier(),
                        address,
                    ),
                    AptosErrorCode::ResourceNotFound,
                    latest_ledger_info,
                )
            })
    }

    pub fn get_state_values(
        &self,
        address: AccountAddress,
        version: u64,
    ) -> Result<HashMap<StateKey, StateValue>> {
        let mut iter = if !db_sharding_enabled(&self.node_config) {
            Box::new(
                self.db
                    .get_prefixed_state_value_iterator(
                        &StateKeyPrefix::from(address),
                        None,
                        version,
                    )?
                    .map(|item| item.map_err(|err| anyhow!(err.to_string()))),
            )
        } else {
            self.indexer_reader
                .as_ref()
                .ok_or_else(|| format_err!("Indexer reader doesn't exist"))?
                .get_prefixed_state_value_iterator(&StateKeyPrefix::from(address), None, version)?
        };

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
        let account_iter = if !db_sharding_enabled(&self.node_config) {
            Box::new(
                self.db
                    .get_prefixed_state_value_iterator(
                        &StateKeyPrefix::from(address),
                        prev_state_key,
                        version,
                    )?
                    .map(|item| item.map_err(|err| anyhow!(err.to_string()))),
            )
        } else {
            self.indexer_reader
                .as_ref()
                .ok_or_else(|| format_err!("Indexer reader doesn't exist"))?
                .get_prefixed_state_value_iterator(
                    &StateKeyPrefix::from(address),
                    prev_state_key,
                    version,
                )?
        };
        // TODO: Consider rewriting this to consider resource groups:
        // * If a resource group is found, expand
        // * Return Option<Result<(PathType, StructTag, Vec<u8>)>>
        // * Count resources and only include a resource group if it can completely fit
        // * Get next_key as the first struct_tag not included
        let mut resource_iter = account_iter
            .filter_map(|res| match res {
                Ok((k, v)) => match k.inner() {
                    StateKeyInner::AccessPath(AccessPath { address: _, path }) => {
                        match Path::try_from(path.as_slice()) {
                            Ok(Path::Resource(struct_tag)) => {
                                Some(Ok((struct_tag, v.bytes().to_vec())))
                            }
                            // TODO: Consider expanding to Path::Resource
                            Ok(Path::ResourceGroup(struct_tag)) => {
                                Some(Ok((struct_tag, v.bytes().to_vec())))
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
            .collect::<Result<Vec<(StructTag, Vec<u8>)>>>()?;

        // We should be able to do an unwrap here, otherwise the above db read would fail.
        let state_view = self.state_view_at_version(version)?;
        let converter = state_view.as_converter(self.db.clone(), self.indexer_reader.clone());

        // Extract resources from resource groups and flatten into all resources
        let kvs = kvs
            .into_iter()
            .map(|(tag, value)| {
                if converter.is_resource_group(&tag) {
                    // An error here means a storage invariant has been violated
                    bcs::from_bytes::<ResourceGroup>(&value)
                        .map(|map| map.into_iter().map(|(t, v)| (t, v)).collect::<Vec<_>>())
                        .map_err(|e| e.into())
                } else {
                    Ok(vec![(tag, value)])
                }
            })
            .collect::<Result<Vec<Vec<(StructTag, Vec<u8>)>>>>()?
            .into_iter()
            .flatten()
            .collect();

        let next_key = if let Some((struct_tag, _v)) = resource_iter.next().transpose()? {
            Some(StateKey::resource(&address, &struct_tag)?)
        } else {
            None
        };
        Ok((kvs, next_key))
    }

    pub fn get_modules_by_pagination(
        &self,
        address: AccountAddress,
        prev_state_key: Option<&StateKey>,
        version: u64,
        limit: u64,
    ) -> Result<(Vec<(ModuleId, Vec<u8>)>, Option<StateKey>)> {
        let account_iter = if !db_sharding_enabled(&self.node_config) {
            Box::new(
                self.db
                    .get_prefixed_state_value_iterator(
                        &StateKeyPrefix::from(address),
                        prev_state_key,
                        version,
                    )?
                    .map(|item| item.map_err(|err| anyhow!(err.to_string()))),
            )
        } else {
            self.indexer_reader
                .as_ref()
                .ok_or_else(|| format_err!("Indexer reader doesn't exist"))?
                .get_prefixed_state_value_iterator(
                    &StateKeyPrefix::from(address),
                    prev_state_key,
                    version,
                )?
        };
        let mut module_iter = account_iter
            .filter_map(|res| match res {
                Ok((k, v)) => match k.inner() {
                    StateKeyInner::AccessPath(AccessPath { address: _, path }) => {
                        match Path::try_from(path.as_slice()) {
                            Ok(Path::Code(module_id)) => Some(Ok((module_id, v.bytes().to_vec()))),
                            Ok(Path::Resource(_)) | Ok(Path::ResourceGroup(_)) => None,
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
        let next_key = module_iter
            .next()
            .transpose()?
            .map(|(module_id, _v)| StateKey::module_id(&module_id));
        Ok((kvs, next_key))
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
            self.node_config.api.max_block_transactions_page_size,
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

        let state_view = self.latest_state_view_poem(ledger_info)?;
        let converter = state_view.as_converter(self.db.clone(), self.indexer_reader.clone());
        let txns: Vec<aptos_api_types::Transaction> = data
            .into_iter()
            .map(|t| {
                // Update the timestamp if the next block occurs
                if let Some(txn) = t.transaction.try_as_block_metadata_ext() {
                    timestamp = txn.timestamp_usecs();
                } else if let Some(txn) = t.transaction.try_as_block_metadata() {
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

        let state_view = self.latest_state_view_poem(ledger_info)?;
        let converter = state_view.as_converter(self.db.clone(), self.indexer_reader.clone());
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

    pub fn render_transaction_summaries<E: InternalError>(
        &self,
        ledger_info: &LedgerInfo,
        data: Vec<IndexedTransactionSummary>,
    ) -> Result<Vec<aptos_api_types::TransactionSummary>, E> {
        if data.is_empty() {
            return Ok(vec![]);
        }

        let txn_summaries: Vec<aptos_api_types::TransactionSummary> = data
            .into_iter()
            .map(|t| {
                Ok(TransactionSummary {
                    sender: t.sender().into(),
                    version: t.version().into(),
                    transaction_hash: t.transaction_hash().into(),
                    replay_protector: match t.replay_protector() {
                        aptos_types::transaction::ReplayProtector::Nonce(nonce) => {
                            ReplayProtector::Nonce(nonce.into())
                        },
                        aptos_types::transaction::ReplayProtector::SequenceNumber(seq_num) => {
                            ReplayProtector::SequenceNumber(seq_num.into())
                        },
                    },
                })
            })
            .collect::<Result<_, anyhow::Error>>()
            .context("Failed to convert transaction summary data from storage")
            .map_err(|err| {
                E::internal_with_code(err, AptosErrorCode::InternalError, ledger_info)
            })?;
        Ok(txn_summaries)
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
            .get_first_output_version()
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
            .zip(infos)
            .enumerate()
            .map(
                |(i, ((txn, txn_output), info))| -> Result<TransactionOnChainData> {
                    let version = start_version + i as u64;
                    let (write_set, events, _, _, _) = txn_output.unpack();
                    let h = self.get_accumulator_root_hash(version)?;
                    let txn: TransactionOnChainData =
                        (version, txn, info, events, h, write_set).into();
                    Ok(self.maybe_translate_v2_to_v1_events(txn))
                },
            )
            .collect()
    }

    pub fn get_account_ordered_transactions<E: NotFoundError + InternalError>(
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
            self.get_resource_poem::<AccountResource, E>(
                address,
                ledger_info.version(),
                ledger_info,
            )?
            .map(|r| r.sequence_number())
            .unwrap_or(0)
            .saturating_sub(limit as u64)
        };

        let txns_res = if !db_sharding_enabled(&self.node_config) {
            self.db.get_account_ordered_transactions(
                address,
                start_seq_number,
                limit as u64,
                true,
                ledger_version,
            )
        } else {
            self.indexer_reader
                .as_ref()
                .ok_or_else(|| anyhow!("Indexer reader is None"))
                .map_err(|err| {
                    E::internal_with_code(err, AptosErrorCode::InternalError, ledger_info)
                })?
                .get_account_ordered_transactions(
                    address,
                    start_seq_number,
                    limit as u64,
                    true,
                    ledger_version,
                )
                .map_err(|e| AptosDbError::Other(e.to_string()))
        };
        let txns = txns_res
            .context("Failed to retrieve account transactions")
            .map_err(|err| {
                E::internal_with_code(err, AptosErrorCode::InternalError, ledger_info)
            })?;
        txns.into_inner()
            .into_iter()
            .map(|t| -> Result<TransactionOnChainData> {
                let txn = self.convert_into_transaction_on_chain_data(t)?;
                Ok(self.maybe_translate_v2_to_v1_events(txn))
            })
            .collect::<Result<Vec<_>>>()
            .context("Failed to parse account transactions")
            .map_err(|err| E::internal_with_code(err, AptosErrorCode::InternalError, ledger_info))
    }

    pub fn get_account_transaction_summaries<E: NotFoundError + InternalError>(
        &self,
        address: AccountAddress,
        start_version: Option<u64>,
        end_version: Option<u64>,
        limit: u16,
        ledger_version: u64,
        ledger_info: &LedgerInfo,
    ) -> Result<Vec<IndexedTransactionSummary>, E> {
        self.db
            .get_account_transaction_summaries(
                address,
                start_version,
                end_version,
                limit as u64,
                ledger_version,
            )
            .context("Failed to retrieve account transaction summaries")
            .map_err(|err| E::internal_with_code(err, AptosErrorCode::InternalError, ledger_info))
    }

    pub fn get_transaction_by_hash(
        &self,
        hash: HashValue,
        ledger_version: u64,
    ) -> Result<Option<TransactionOnChainData>> {
        if let Some(t) = self
            .db
            .get_transaction_by_hash(hash, ledger_version, true)?
        {
            let txn: TransactionOnChainData = self.convert_into_transaction_on_chain_data(t)?;
            Ok(Some(self.maybe_translate_v2_to_v1_events(txn)))
        } else {
            Ok(None)
        }
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
        let txn = self.convert_into_transaction_on_chain_data(
            self.db
                .get_transaction_by_version(version, ledger_version, true)?,
        )?;
        Ok(self.maybe_translate_v2_to_v1_events(txn))
    }

    fn maybe_translate_v2_to_v1_events(
        &self,
        mut txn: TransactionOnChainData,
    ) -> TransactionOnChainData {
        if self.indexer_reader.is_some()
            && self
                .node_config
                .indexer_db_config
                .enable_event_v2_translation
        {
            self.translate_v2_to_v1_events_for_version(txn.version, &mut txn.events)
                .ok();
        }
        txn
    }

    fn translate_v2_to_v1_events_for_version(
        &self,
        version: u64,
        events: &mut [ContractEvent],
    ) -> Result<()> {
        for (idx, event) in events.iter_mut().enumerate() {
            let translated_event = self
                .indexer_reader
                .as_ref()
                .ok_or(anyhow!("Internal indexer reader doesn't exist"))?
                .get_translated_v1_event_by_version_and_index(version, idx as u64);
            if let Ok(translated_event) = translated_event {
                *event = ContractEvent::V1(translated_event);
            }
        }
        Ok(())
    }

    pub fn translate_v2_to_v1_events_for_simulation(
        &self,
        events: &mut [ContractEvent],
    ) -> Result<()> {
        let mut count_map: HashMap<EventKey, u64> = HashMap::new();
        for event in events.iter_mut() {
            if let ContractEvent::V2(v2) = event {
                let translated_event = self
                    .indexer_reader
                    .as_ref()
                    .ok_or(anyhow!("Internal indexer reader doesn't exist"))?
                    .translate_event_v2_to_v1(v2)?;
                if let Some(v1) = translated_event {
                    let count = count_map.get(v1.key()).unwrap_or(&0);
                    let v1_adjusted = ContractEventV1::new(
                        *v1.key(),
                        v1.sequence_number() + count,
                        v1.type_tag().clone(),
                        v1.event_data().to_vec(),
                    )?;
                    *event = ContractEvent::V1(v1_adjusted);
                    count_map.insert(*v1.key(), count + 1);
                }
            }
        }
        Ok(())
    }

    pub fn get_accumulator_root_hash(&self, version: u64) -> Result<HashValue> {
        Ok(self.db.get_accumulator_root_hash(version)?)
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
        let (start, order) = if let Some(start) = start {
            (start, Order::Ascending)
        } else {
            (u64::MAX, Order::Descending)
        };
        let mut res = if !db_sharding_enabled(&self.node_config) {
            self.db
                .get_events(event_key, start, order, limit as u64, ledger_version)?
        } else {
            self.indexer_reader
                .as_ref()
                .ok_or_else(|| anyhow!("Internal indexer reader doesn't exist"))?
                .get_events(event_key, start, order, limit as u64, ledger_version)?
        };
        if order == Order::Descending {
            res.reverse();
            Ok(res)
        } else {
            Ok(res)
        }
    }

    pub fn get_indexer_reader(&self) -> Option<&Arc<dyn IndexerReader>> {
        self.indexer_reader.as_ref()
    }

    fn next_bucket(&self, gas_unit_price: u64) -> u64 {
        match self
            .node_config
            .mempool
            .broadcast_buckets
            .iter()
            .find(|bucket| **bucket > gas_unit_price)
        {
            None => gas_unit_price,
            Some(bucket) => *bucket,
        }
    }

    fn default_gas_estimation(&self, min_gas_unit_price: u64) -> GasEstimation {
        GasEstimation {
            deprioritized_gas_estimate: Some(min_gas_unit_price),
            gas_estimate: min_gas_unit_price,
            prioritized_gas_estimate: Some(self.next_bucket(min_gas_unit_price)),
        }
    }

    fn cached_gas_estimation<T>(&self, cache: &T, current_epoch: u64) -> Option<GasEstimation>
    where
        T: Deref<Target = GasEstimationCache>,
    {
        if let Some(epoch) = cache.last_updated_epoch {
            if let Some(time) = cache.last_updated_time {
                if let Some(estimation) = cache.estimation {
                    if epoch == current_epoch
                        && (time.elapsed().as_millis() as u64)
                            < self.node_config.api.gas_estimation.cache_expiration_ms
                    {
                        return Some(estimation);
                    }
                }
            }
        }
        None
    }

    fn update_cached_gas_estimation(
        &self,
        cache: &mut RwLockWriteGuard<GasEstimationCache>,
        epoch: u64,
        estimation: GasEstimation,
    ) {
        cache.last_updated_epoch = Some(epoch);
        cache.estimation = Some(estimation);
        cache.last_updated_time = Some(Instant::now());
    }

    fn get_gas_prices_and_used(
        &self,
        start_version: Version,
        limit: u64,
        ledger_version: Version,
        count_majority_use_case: bool,
    ) -> Result<(Vec<(u64, u64)>, Vec<BlockEndInfo>, Option<f32>)> {
        if start_version > ledger_version || limit == 0 {
            return Ok((vec![], vec![], None));
        }

        // This is just an estimation, so we can just skip over errors
        let limit = std::cmp::min(limit, ledger_version - start_version + 1);
        let txns = self.db.get_transaction_iterator(start_version, limit)?;
        let infos = self
            .db
            .get_transaction_info_iterator(start_version, limit)?;

        let mut gas_prices = Vec::new();
        let mut block_end_infos = Vec::new();
        let mut count_by_use_case = HashMap::new();
        for (txn, info) in txns.zip(infos) {
            match txn.as_ref() {
                Ok(Transaction::UserTransaction(txn)) => {
                    if let Ok(info) = info.as_ref() {
                        gas_prices.push((txn.gas_unit_price(), info.gas_used()));
                        if count_majority_use_case {
                            let use_case_key = txn.parse_use_case();
                            *count_by_use_case.entry(use_case_key).or_insert(0) += 1;
                        }
                    }
                },
                Ok(Transaction::BlockEpilogue(txn)) => {
                    if let Some(block_end_info) = txn.try_as_block_end_info() {
                        block_end_infos.push(block_end_info.clone());
                    }
                },
                _ => {},
            }
        }

        let majority_use_case_fraction = if count_majority_use_case {
            count_by_use_case.iter().max_by_key(|(_, v)| *v).and_then(
                |(max_use_case, max_value)| {
                    if let UseCaseKey::ContractAddress(_) = max_use_case {
                        Some(*max_value as f32 / count_by_use_case.values().sum::<u64>() as f32)
                    } else {
                        None
                    }
                },
            )
        } else {
            None
        };
        Ok((gas_prices, block_end_infos, majority_use_case_fraction))
    }

    fn block_min_inclusion_price(
        &self,
        ledger_info: &LedgerInfo,
        first: Version,
        last: Version,
        gas_estimation_config: &GasEstimationConfig,
        execution_config: &OnChainExecutionConfig,
    ) -> Option<u64> {
        let user_use_case_spread_factor = if gas_estimation_config.incorporate_reordering_effects {
            execution_config
                .transaction_shuffler_type()
                .user_use_case_spread_factor()
        } else {
            None
        };

        match self.get_gas_prices_and_used(
            first,
            last - first,
            ledger_info.ledger_version.0,
            user_use_case_spread_factor.is_some(),
        ) {
            Ok((prices_and_used, block_end_infos, majority_use_case_fraction)) => {
                let is_full_block =
                    if majority_use_case_fraction.is_some_and(|fraction| fraction > 0.5) {
                        // If majority use case is above half of transactions, UseCaseAware block reordering
                        // will allow other transactions to get in the block (AIP-68)
                        false
                    } else if prices_and_used.len() >= gas_estimation_config.full_block_txns {
                        true
                    } else if !block_end_infos.is_empty() {
                        assert_eq!(1, block_end_infos.len());
                        block_end_infos.first().unwrap().limit_reached()
                    } else if let Some(block_gas_limit) =
                        execution_config.block_gas_limit_type().block_gas_limit()
                    {
                        let gas_used = prices_and_used.iter().map(|(_, used)| *used).sum::<u64>();
                        gas_used >= block_gas_limit
                    } else {
                        false
                    };

                if is_full_block {
                    Some(
                        self.next_bucket(
                            prices_and_used
                                .iter()
                                .map(|(price, _)| *price)
                                .min()
                                .unwrap(),
                        ),
                    )
                } else {
                    None
                }
            },
            Err(_) => None,
        }
    }

    pub fn estimate_gas_price<E: InternalError>(
        &self,
        ledger_info: &LedgerInfo,
    ) -> Result<GasEstimation, E> {
        let config = &self.node_config.api.gas_estimation;
        let min_gas_unit_price = self.min_gas_unit_price(ledger_info)?;
        let execution_config = self.execution_onchain_config(ledger_info)?;
        if !config.enabled {
            return Ok(self.default_gas_estimation(min_gas_unit_price));
        }
        if let Some(static_override) = &config.static_override {
            return Ok(GasEstimation {
                deprioritized_gas_estimate: Some(static_override.low),
                gas_estimate: static_override.market,
                prioritized_gas_estimate: Some(static_override.aggressive),
            });
        }

        let epoch = ledger_info.epoch.0;

        // 0. (0) Return cached result if it exists
        let cache = self.gas_estimation_cache.read().unwrap();
        if let Some(cached_gas_estimation) = self.cached_gas_estimation(&cache, epoch) {
            return Ok(cached_gas_estimation);
        }
        drop(cache);

        // 0. (1) Write lock and prepare cache
        let mut cache = self.gas_estimation_cache.write().unwrap();
        // Retry cached result after acquiring write lock
        if let Some(cached_gas_estimation) = self.cached_gas_estimation(&cache, epoch) {
            return Ok(cached_gas_estimation);
        }
        // Clear the cache if the epoch has changed
        if let Some(cached_epoch) = cache.last_updated_epoch {
            if cached_epoch != epoch {
                cache.min_inclusion_prices.clear();
            }
        }

        let max_block_history = config.aggressive_block_history;
        // 1. Get the block metadata txns
        let mut lookup_version = ledger_info.ledger_version.0;
        let mut blocks = vec![];
        // Skip the first block, which may be partial
        if let Ok((first, _, block)) = self.db.get_block_info_by_version(lookup_version) {
            if block.epoch() == epoch {
                lookup_version = first.saturating_sub(1);
            }
        }
        let mut cached_blocks_hit = false;
        for _i in 0..max_block_history {
            if cache
                .min_inclusion_prices
                .contains_key(&(epoch, lookup_version))
            {
                cached_blocks_hit = true;
                break;
            }
            match self.db.get_block_info_by_version(lookup_version) {
                Ok((first, last, block)) => {
                    if block.epoch() != epoch {
                        break;
                    }
                    lookup_version = first.saturating_sub(1);
                    blocks.push((first, last));
                    if lookup_version == 0 {
                        break;
                    }
                },
                Err(_) => {
                    break;
                },
            }
        }
        if blocks.is_empty() && !cached_blocks_hit {
            let estimation = self.default_gas_estimation(min_gas_unit_price);
            self.update_cached_gas_estimation(&mut cache, epoch, estimation);
            return Ok(estimation);
        }
        let blocks_len = blocks.len();
        let remaining = max_block_history - blocks_len;

        // 2. Get gas prices per block
        let mut min_inclusion_prices = vec![];
        // TODO: if multiple calls to db is a perf issue, combine into a single call and then split
        for (first, last) in blocks {
            let min_inclusion_price = self
                .block_min_inclusion_price(ledger_info, first, last, config, &execution_config)
                .unwrap_or(min_gas_unit_price);
            min_inclusion_prices.push(min_inclusion_price);
            cache
                .min_inclusion_prices
                .insert((epoch, last), min_inclusion_price);
        }
        if cached_blocks_hit {
            for (_, v) in cache
                .min_inclusion_prices
                .range((Included(&(epoch, 0)), Included(&(epoch, lookup_version))))
                .rev()
                .take(remaining)
            {
                min_inclusion_prices.push(*v);
            }
        }

        // 3. Get values
        // (1) low
        let low_price = match min_inclusion_prices
            .iter()
            .take(config.low_block_history)
            .min()
        {
            Some(price) => *price,
            None => min_gas_unit_price,
        };

        // (2) market
        let mut latest_prices: Vec<_> = min_inclusion_prices
            .iter()
            .take(config.market_block_history)
            .cloned()
            .collect();
        latest_prices.sort();
        let market_price = match latest_prices.get(latest_prices.len() / 2) {
            None => {
                error!(
                    "prices empty, blocks.len={}, cached_blocks_hit={}, epoch={}, version={}",
                    blocks_len,
                    cached_blocks_hit,
                    ledger_info.epoch.0,
                    ledger_info.ledger_version.0
                );
                return Ok(self.default_gas_estimation(min_gas_unit_price));
            },
            Some(price) => low_price.max(*price),
        };

        // (3) aggressive
        min_inclusion_prices.sort();
        let p90_price = match min_inclusion_prices.get(min_inclusion_prices.len() * 9 / 10) {
            None => {
                error!(
                    "prices empty, blocks.len={}, cached_blocks_hit={}, epoch={}, version={}",
                    blocks_len,
                    cached_blocks_hit,
                    ledger_info.epoch.0,
                    ledger_info.ledger_version.0
                );
                return Ok(self.default_gas_estimation(min_gas_unit_price));
            },
            Some(price) => market_price.max(*price),
        };
        // round up to next bucket
        let aggressive_price = self.next_bucket(p90_price);

        let estimation = GasEstimation {
            deprioritized_gas_estimate: Some(low_price),
            gas_estimate: market_price,
            prioritized_gas_estimate: Some(aggressive_price),
        };
        // 4. Update cache
        // GC old entries
        if cache.min_inclusion_prices.len() > max_block_history {
            for _i in max_block_history..cache.min_inclusion_prices.len() {
                cache.min_inclusion_prices.pop_first();
            }
        }
        self.update_cached_gas_estimation(&mut cache, epoch, estimation);
        Ok(estimation)
    }

    fn min_gas_unit_price<E: InternalError>(&self, ledger_info: &LedgerInfo) -> Result<u64, E> {
        let (_, gas_schedule) = self.get_gas_schedule(ledger_info)?;
        Ok(gas_schedule.vm.txn.min_price_per_gas_unit.into())
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

            let gas_schedule_params = {
                let may_be_params =
                    GasScheduleV2::fetch_config(&state_view).and_then(|gas_schedule| {
                        let feature_version = gas_schedule.feature_version;
                        let gas_schedule = gas_schedule.into_btree_map();
                        AptosGasParameters::from_on_chain_gas_schedule(
                            &gas_schedule,
                            feature_version,
                        )
                        .ok()
                    });
                match may_be_params {
                    Some(gas_schedule) => Ok(gas_schedule),
                    None => GasSchedule::fetch_config(&state_view)
                        .and_then(|gas_schedule| {
                            let gas_schedule = gas_schedule.into_btree_map();
                            AptosGasParameters::from_on_chain_gas_schedule(&gas_schedule, 0).ok()
                        })
                        .ok_or_else(|| {
                            E::internal_with_code(
                                "Failed to retrieve gas schedule",
                                AptosErrorCode::InternalError,
                                ledger_info,
                            )
                        }),
                }?
            };

            // Update the cache
            cache.gas_schedule_params = Some(gas_schedule_params.clone());
            cache.last_updated_epoch = Some(ledger_info.epoch.0);
            Ok((
                cache.last_updated_epoch.unwrap_or_default(),
                gas_schedule_params,
            ))
        }
    }

    pub fn execution_onchain_config<E: InternalError>(
        &self,
        ledger_info: &LedgerInfo,
    ) -> Result<OnChainExecutionConfig, E> {
        // If it's the same epoch, use the cached results
        {
            let cache = self.gas_limit_cache.read().unwrap();
            if let Some(ref last_updated_epoch) = cache.last_updated_epoch {
                if *last_updated_epoch == ledger_info.epoch.0 {
                    return Ok(cache.execution_onchain_config.clone());
                }
            }
        }

        // Otherwise refresh the cache
        {
            let mut cache = self.gas_limit_cache.write().unwrap();
            // If a different thread updated the cache, we can exit early
            if let Some(ref last_updated_epoch) = cache.last_updated_epoch {
                if *last_updated_epoch == ledger_info.epoch.0 {
                    return Ok(cache.execution_onchain_config.clone());
                }
            }

            // Retrieve the execution config from storage and parse it accordingly
            let state_view = self
                .db
                .state_view_at_version(Some(ledger_info.version()))
                .map_err(|e| {
                    E::internal_with_code(e, AptosErrorCode::InternalError, ledger_info)
                })?;

            let execution_onchain_config = OnChainExecutionConfig::fetch_config(&state_view)
                .unwrap_or_else(OnChainExecutionConfig::default_if_missing);

            // Update the cache
            cache.execution_onchain_config = execution_onchain_config.clone();
            cache.last_updated_epoch = Some(ledger_info.epoch.0);
            Ok(execution_onchain_config)
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
            },
            AcceptType::Bcs => {
                if !self.node_config.api.bcs_output_enabled {
                    return Err(bcs_api_disabled(api_name));
                }
            },
        }
        Ok(())
    }

    pub fn last_updated_gas_schedule(&self) -> Option<u64> {
        self.gas_schedule_cache.read().unwrap().last_updated_epoch
    }

    pub fn last_updated_gas_estimation_cache_size(&self) -> usize {
        self.gas_estimation_cache
            .read()
            .unwrap()
            .min_inclusion_prices
            .len()
    }

    pub fn view_function_stats(&self) -> &FunctionStats {
        &self.view_function_stats
    }

    pub fn simulate_txn_stats(&self) -> &FunctionStats {
        &self.simulate_txn_stats
    }
}

pub struct GasScheduleCache {
    last_updated_epoch: Option<u64>,
    gas_schedule_params: Option<AptosGasParameters>,
}

pub struct GasEstimationCache {
    last_updated_epoch: Option<u64>,
    last_updated_time: Option<Instant>,
    estimation: Option<GasEstimation>,
    /// (epoch, lookup_version) -> min_inclusion_price
    min_inclusion_prices: BTreeMap<(u64, u64), u64>,
}

pub struct GasLimitCache {
    last_updated_epoch: Option<u64>,
    execution_onchain_config: OnChainExecutionConfig,
}

/// This function just calls tokio::task::spawn_blocking with the given closure and in
/// the case of an error when joining the task converts it into a 500.
pub async fn api_spawn_blocking<F, T, E>(func: F) -> Result<T, E>
where
    F: FnOnce() -> Result<T, E> + Send + 'static,
    T: Send + 'static,
    E: InternalError + Send + 'static,
{
    tokio::task::spawn_blocking(func)
        .await
        .map_err(|err| E::internal_with_code_no_info(err, AptosErrorCode::InternalError))?
}

#[derive(Schema)]
pub struct LogSchema {
    event: LogEvent,
}

impl LogSchema {
    pub fn new(event: LogEvent) -> Self {
        Self { event }
    }
}

#[derive(Serialize, Copy, Clone)]
pub enum LogEvent {
    ViewFunction,
    TxnSimulation,
}

pub enum FunctionType {
    ViewFunction,
    TxnSimulation,
}

impl FunctionType {
    fn log_event(&self) -> LogEvent {
        match self {
            FunctionType::ViewFunction => LogEvent::ViewFunction,
            FunctionType::TxnSimulation => LogEvent::TxnSimulation,
        }
    }

    fn operation_id(&self) -> &'static str {
        match self {
            FunctionType::ViewFunction => "view_function",
            FunctionType::TxnSimulation => "txn_simulation",
        }
    }
}

pub struct FunctionStats {
    stats: Option<Cache<String, (Arc<AtomicU64>, Arc<AtomicU64>)>>,
    log_event: LogEvent,
    operation_id: String,
}

impl FunctionStats {
    fn new(function_type: FunctionType, log_per_call_stats: bool) -> Self {
        let stats = if log_per_call_stats {
            Some(Cache::new(100))
        } else {
            None
        };
        FunctionStats {
            stats,
            log_event: function_type.log_event(),
            operation_id: function_type.operation_id().to_string(),
        }
    }

    pub fn function_to_key(module: &ModuleId, function: &Identifier) -> String {
        format!("{}::{}", module, function)
    }

    pub fn increment(&self, key: String, gas: u64) {
        metrics::GAS_USED
            .with_label_values(&[&self.operation_id])
            .observe(gas as f64);
        if let Some(stats) = &self.stats {
            let (prev_gas, prev_count) = stats.get(&key).unwrap_or_else(|| {
                // Note, race can occur on inserting new entry, resulting in some lost data, but it should be fine
                let new_gas = Arc::new(AtomicU64::new(0));
                let new_count = Arc::new(AtomicU64::new(0));
                stats.insert(key.clone(), (new_gas.clone(), new_count.clone()));
                (new_gas, new_count)
            });
            prev_gas.fetch_add(gas, Ordering::Relaxed);
            prev_count.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn log_and_clear(&self) {
        if let Some(stats) = &self.stats {
            if stats.iter().next().is_none() {
                return;
            }

            let mut sorted: Vec<_> = stats
                .iter()
                .map(|entry| {
                    let (gas_used, count) = entry.value();
                    (
                        gas_used.load(Ordering::Relaxed),
                        count.load(Ordering::Relaxed),
                        entry.key().clone(),
                    )
                })
                .collect();
            sorted.sort_by_key(|(gas_used, ..)| Reverse(*gas_used));

            info!(
                LogSchema::new(self.log_event),
                top_1 = sorted.first(),
                top_2 = sorted.get(1),
                top_3 = sorted.get(2),
                top_4 = sorted.get(3),
                top_5 = sorted.get(4),
                top_6 = sorted.get(5),
                top_7 = sorted.get(6),
                top_8 = sorted.get(7),
            );

            stats.invalidate_all();
        }
    }
}

fn db_sharding_enabled(node_config: &NodeConfig) -> bool {
    node_config.storage.rocksdb_configs.enable_storage_sharding
}

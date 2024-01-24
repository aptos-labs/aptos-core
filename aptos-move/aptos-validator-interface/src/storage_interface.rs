// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{AptosValidatorInterface, FilterCondition};
use anyhow::{bail, ensure, Result};
use aptos_config::config::{
    RocksdbConfigs, StorageDirPaths, BUFFERED_STATE_TARGET_ITEMS,
    DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD, NO_OP_STORAGE_PRUNER_CONFIG,
};
use aptos_db::AptosDB;
use aptos_framework::natives::code::PackageMetadata;
use aptos_storage_interface::{AptosDbError, DbReader, MAX_REQUEST_LIMIT};
use aptos_types::{
    account_address::AccountAddress,
    account_state::AccountState,
    state_store::{state_key::StateKey, state_key_prefix::StateKeyPrefix, state_value::StateValue},
    transaction::{Transaction, TransactionInfo, Version},
};
use std::{collections::HashMap, path::Path, sync::Arc};

pub struct DBDebuggerInterface(Arc<dyn DbReader>);

impl DBDebuggerInterface {
    pub fn open<P: AsRef<Path> + Clone>(db_root_path: P) -> Result<Self> {
        Ok(Self(Arc::new(
            AptosDB::open(
                StorageDirPaths::from_path(db_root_path),
                true,
                NO_OP_STORAGE_PRUNER_CONFIG,
                RocksdbConfigs::default(),
                false, /* indexer */
                BUFFERED_STATE_TARGET_ITEMS,
                DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
                false, /* indexer async v2 */
            )
            .map_err(anyhow::Error::from)?,
        )))
    }
}

#[async_trait::async_trait]
impl AptosValidatorInterface for DBDebuggerInterface {
    async fn get_account_state_by_version(
        &self,
        account: AccountAddress,
        version: Version,
    ) -> Result<Option<AccountState>> {
        let key_prefix = StateKeyPrefix::from(account);
        let mut iter = self
            .0
            .get_prefixed_state_value_iterator(&key_prefix, None, version)?;
        let kvs = iter
            .by_ref()
            .take(MAX_REQUEST_LIMIT as usize)
            .collect::<Result<_, AptosDbError>>()
            .map_err(Into::<anyhow::Error>::into)?;
        if iter.next().is_some() {
            bail!(
                "Too many state items under state key prefix {:?}.",
                key_prefix
            );
        }
        AccountState::from_access_paths_and_values(account, &kvs)
    }

    async fn get_state_value_by_version(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> Result<Option<StateValue>> {
        self.0
            .get_state_value_by_version(state_key, version)
            .map_err(Into::into)
    }

    async fn get_committed_transactions(
        &self,
        start: Version,
        limit: u64,
    ) -> Result<(Vec<Transaction>, Vec<TransactionInfo>)> {
        let txn_iter = self.0.get_transaction_iterator(start, limit)?;
        let txn_info_iter = self.0.get_transaction_info_iterator(start, limit)?;
        let txns = txn_iter
            .map(|res| res.map_err(Into::into))
            .collect::<Result<Vec<_>>>()?;
        let txn_infos = txn_info_iter
            .map(|res| res.map_err(Into::into))
            .collect::<Result<Vec<_>>>()?;
        ensure!(txns.len() == txn_infos.len());
        Ok((txns, txn_infos))
    }

    async fn get_and_filter_committed_transactions(
        &self,
        _start: Version,
        _limit: u64,
        _filter_condition: FilterCondition,
    ) -> Result<
        Vec<(
            u64,
            Transaction,
            Option<(
                AccountAddress,
                String,
                HashMap<(AccountAddress, String), PackageMetadata>,
            )>,
        )>,
    > {
        unimplemented!();
    }

    async fn get_latest_version(&self) -> Result<Version> {
        self.0.get_latest_version().map_err(Into::into)
    }

    async fn get_version_by_account_sequence(
        &self,
        account: AccountAddress,
        seq: u64,
    ) -> Result<Option<Version>> {
        let ledger_version = self.get_latest_version().await?;
        self.0
            .get_account_transaction(account, seq, false, ledger_version)
            .map_or_else(
                |e| Err(anyhow::Error::from(e)),
                |tp| Ok(tp.map(|e| e.version)),
            )
    }
}

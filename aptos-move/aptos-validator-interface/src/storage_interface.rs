// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::AptosValidatorInterface;
use anyhow::{anyhow, Result};
use aptos_config::config::{
    RocksdbConfigs, DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD, NO_OP_STORAGE_PRUNER_CONFIG,
    TARGET_SNAPSHOT_SIZE,
};
use aptos_types::{
    account_address::AccountAddress,
    account_state::AccountState,
    contract_event::EventWithVersion,
    event::EventKey,
    state_store::{state_key::StateKey, state_key_prefix::StateKeyPrefix, state_value::StateValue},
    transaction::{Transaction, Version},
};
use aptosdb::AptosDB;
use std::{path::Path, sync::Arc};
use storage_interface::{DbReader, Order};

pub struct DBDebuggerInterface(Arc<dyn DbReader>);

impl DBDebuggerInterface {
    pub fn open<P: AsRef<Path> + Clone>(db_root_path: P) -> Result<Self> {
        Ok(Self(Arc::new(AptosDB::open(
            db_root_path,
            true,
            NO_OP_STORAGE_PRUNER_CONFIG,
            RocksdbConfigs::default(),
            false,
            TARGET_SNAPSHOT_SIZE,
            DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
        )?)))
    }
}

impl AptosValidatorInterface for DBDebuggerInterface {
    fn get_account_state_by_version(
        &self,
        account: AccountAddress,
        version: Version,
    ) -> Result<Option<AccountState>> {
        AccountState::from_access_paths_and_values(
            account,
            &self
                .0
                .get_state_values_by_key_prefix(&StateKeyPrefix::from(account), version)?,
        )
    }

    fn get_state_value_by_version(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> Result<Option<StateValue>> {
        Ok(self
            .0
            .get_state_value_with_proof_by_version(state_key, version)?
            .0)
    }

    fn get_events(
        &self,
        key: &EventKey,
        start_seq: u64,
        limit: u64,
        ledger_version: Version,
    ) -> Result<Vec<EventWithVersion>> {
        self.0
            .get_events(key, start_seq, Order::Ascending, limit, ledger_version)
    }

    fn get_committed_transactions(&self, start: Version, limit: u64) -> Result<Vec<Transaction>> {
        Ok(self
            .0
            .get_transactions(start, limit, self.get_latest_version()?, false)?
            .transactions)
    }

    fn get_latest_version(&self) -> Result<Version> {
        let (version, _) = self
            .0
            .get_latest_transaction_info_option()?
            .ok_or_else(|| anyhow!("DB doesn't have any transaction."))?;
        Ok(version)
    }

    fn get_version_by_account_sequence(
        &self,
        account: AccountAddress,
        seq: u64,
    ) -> Result<Option<Version>> {
        let ledger_version = self.get_latest_version()?;
        Ok(self
            .0
            .get_account_transaction(account, seq, false, ledger_version)?
            .map(|info| info.version))
    }
}

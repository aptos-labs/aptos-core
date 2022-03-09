// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::AptosValidatorInterface;
use anyhow::{anyhow, Result};
use aptos_config::config::{RocksdbConfig, NO_OP_STORAGE_PRUNER_CONFIG};
use aptos_types::{
    account_address::AccountAddress,
    account_state::AccountState,
    contract_event::EventWithProof,
    event::EventKey,
    transaction::{Transaction, Version},
};
use aptosdb::AptosDB;
use std::{convert::TryFrom, path::Path, sync::Arc};
use storage_interface::{DbReader, Order};

pub struct DBDebuggerInterface(Arc<dyn DbReader>);

impl DBDebuggerInterface {
    pub fn open<P: AsRef<Path> + Clone>(db_root_path: P) -> Result<Self> {
        Ok(Self(Arc::new(AptosDB::open(
            db_root_path,
            true,
            NO_OP_STORAGE_PRUNER_CONFIG,
            RocksdbConfig::default(),
            true, /* account_count_migration, ignored anyway */
        )?)))
    }
}

impl AptosValidatorInterface for DBDebuggerInterface {
    fn get_account_state_by_version(
        &self,
        account: AccountAddress,
        version: Version,
    ) -> Result<Option<AccountState>> {
        self.0
            .get_account_state_with_proof_by_version(account, version)?
            .0
            .map(|s| AccountState::try_from(&s))
            .transpose()
    }

    fn get_events(
        &self,
        key: &EventKey,
        start_seq: u64,
        limit: u64,
    ) -> Result<Vec<EventWithProof>> {
        self.0
            .get_events_with_proofs(key, start_seq, Order::Ascending, limit, None)
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

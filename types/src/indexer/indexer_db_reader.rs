// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    account_address::AccountAddress,
    contract_event::EventWithVersion,
    event::EventKey,
    state_store::{
        state_key::{prefix::StateKeyPrefix, StateKey},
        state_value::StateValue,
        table::{TableHandle, TableInfo},
    },
    transaction::{AccountTransactionsWithProof, Version},
};
use anyhow::Result;

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum Order {
    Ascending,
    Descending,
}

pub trait IndexerReader: Send + Sync {
    fn is_internal_indexer_enabled(&self) -> bool;

    fn get_table_info(&self, handle: TableHandle) -> Result<Option<TableInfo>>;

    fn get_events(
        &self,
        event_key: &EventKey,
        start: u64,
        order: Order,
        limit: u64,
        ledger_version: Version,
    ) -> Result<Vec<EventWithVersion>>;

    fn get_events_by_event_key(
        &self,
        event_key: &EventKey,
        start_seq_num: u64,
        order: Order,
        limit: u64,
        ledger_version: Version,
    ) -> Result<Vec<EventWithVersion>>;

    fn get_account_transactions(
        &self,
        address: AccountAddress,
        start_seq_num: u64,
        limit: u64,
        include_events: bool,
        ledger_version: Version,
    ) -> Result<AccountTransactionsWithProof>;

    fn get_prefixed_state_value_iterator(
        &self,
        key_prefix: &StateKeyPrefix,
        cursor: Option<&StateKey>,
        version: Version,
    ) -> Result<Box<dyn Iterator<Item = Result<(StateKey, StateValue)>> + '_>>;

    fn get_latest_internal_indexer_ledger_version(&self) -> Result<Option<Version>>;

    #[cfg(any(test, feature = "fuzzing"))]
    fn wait_for_internal_indexer(&self, version: Version) -> Result<()> {
        while self
            .get_latest_internal_indexer_ledger_version()?
            .map_or(true, |v| v < version)
        {
            std::thread::sleep(std::time::Duration::from_millis(200));
        }

        Ok(())
    }
}

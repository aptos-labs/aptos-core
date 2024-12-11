// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{db_indexer::DBIndexer, db_v2::IndexerAsyncV2};
use anyhow::anyhow;
use aptos_types::{
    account_address::AccountAddress,
    contract_event::{ContractEventV1, ContractEventV2, EventWithVersion},
    event::EventKey,
    indexer::indexer_db_reader::{IndexerReader, Order},
    state_store::{
        state_key::{prefix::StateKeyPrefix, StateKey},
        state_value::StateValue,
        table::{TableHandle, TableInfo},
    },
    transaction::{AccountTransactionsWithProof, Version},
};
use std::sync::Arc;

#[derive(Clone)]
pub struct IndexerReaders {
    table_info_reader: Option<Arc<IndexerAsyncV2>>,
    db_indexer_reader: Option<Arc<DBIndexer>>,
}

impl IndexerReaders {
    pub fn new(
        table_info_reader: Option<Arc<IndexerAsyncV2>>,
        db_indexer_reader: Option<Arc<DBIndexer>>,
    ) -> Option<Self> {
        if table_info_reader.is_none() && db_indexer_reader.is_none() {
            None
        } else {
            Some(Self {
                table_info_reader,
                db_indexer_reader,
            })
        }
    }
}

impl IndexerReader for IndexerReaders {
    fn is_internal_indexer_enabled(&self) -> bool {
        self.db_indexer_reader.is_some()
    }

    fn get_table_info(&self, handle: TableHandle) -> anyhow::Result<Option<TableInfo>> {
        if let Some(table_info_reader) = &self.table_info_reader {
            return Ok(table_info_reader.get_table_info_with_retry(handle)?);
        }
        anyhow::bail!("Table info reader is not available")
    }

    fn get_latest_internal_indexer_ledger_version(&self) -> anyhow::Result<Option<Version>> {
        if let Some(db_indexer) = &self.db_indexer_reader {
            return Ok(db_indexer.indexer_db.get_persisted_version()?);
        }
        anyhow::bail!("DB indexer reader is not available")
    }

    fn get_latest_table_info_ledger_version(&self) -> anyhow::Result<Option<Version>> {
        if let Some(table_info_reader) = &self.table_info_reader {
            return Ok(Some(table_info_reader.next_version()));
        }
        anyhow::bail!("Table info reader is not available")
    }

    fn get_events(
        &self,
        event_key: &EventKey,
        start: u64,
        order: Order,
        limit: u64,
        ledger_version: Version,
    ) -> anyhow::Result<Vec<EventWithVersion>> {
        if let Some(db_indexer_reader) = &self.db_indexer_reader {
            if db_indexer_reader.indexer_db.event_enabled() {
                return Ok(db_indexer_reader.get_events(
                    event_key,
                    start,
                    order,
                    limit,
                    ledger_version,
                )?);
            } else {
                anyhow::bail!("Internal event index is not enabled")
            }
        }
        anyhow::bail!("DB Indexer reader is not available")
    }

    fn get_events_by_event_key(
        &self,
        event_key: &EventKey,
        start_seq_num: u64,
        order: Order,
        limit: u64,
        ledger_version: Version,
    ) -> anyhow::Result<Vec<EventWithVersion>> {
        if let Some(db_indexer_reader) = &self.db_indexer_reader {
            if db_indexer_reader.indexer_db.event_enabled() {
                return Ok(db_indexer_reader.get_events_by_event_key(
                    event_key,
                    start_seq_num,
                    order,
                    limit,
                    ledger_version,
                )?);
            } else {
                anyhow::bail!("Internal event index is not enabled")
            }
        }
        anyhow::bail!("DB indexer reader is not available")
    }

    fn get_account_transactions(
        &self,
        address: AccountAddress,
        start_seq_num: u64,
        limit: u64,
        include_events: bool,
        ledger_version: Version,
    ) -> anyhow::Result<AccountTransactionsWithProof> {
        if let Some(db_indexer_reader) = &self.db_indexer_reader {
            if db_indexer_reader.indexer_db.transaction_enabled() {
                return Ok(db_indexer_reader.get_account_transactions(
                    address,
                    start_seq_num,
                    limit,
                    include_events,
                    ledger_version,
                )?);
            } else {
                anyhow::bail!("Interal transaction by account index is not enabled")
            }
        }
        anyhow::bail!("DB indexer reader is not available")
    }

    fn get_prefixed_state_value_iterator(
        &self,
        key_prefix: &StateKeyPrefix,
        cursor: Option<&StateKey>,
        ledger_version: Version,
    ) -> anyhow::Result<Box<dyn Iterator<Item = anyhow::Result<(StateKey, StateValue)>> + '_>> {
        if let Some(db_indexer_reader) = &self.db_indexer_reader {
            if db_indexer_reader.indexer_db.statekeys_enabled() {
                return Ok(Box::new(
                    db_indexer_reader
                        .get_prefixed_state_value_iterator(key_prefix, cursor, ledger_version)
                        .map_err(|err| {
                            anyhow!(format!(
                                "failed to get prefixed state value iterator {}",
                                err
                            ))
                        })?,
                )
                    as Box<
                        dyn Iterator<Item = anyhow::Result<(StateKey, StateValue)>>,
                    >);
            } else {
                anyhow::bail!("Internal statekeys index is not enabled")
            }
        }
        anyhow::bail!("DB indexer reader is not available")
    }

    fn get_translated_v1_event_by_version_and_index(
        &self,
        version: Version,
        index: u64,
    ) -> anyhow::Result<ContractEventV1> {
        if let Some(db_indexer_reader) = &self.db_indexer_reader {
            if db_indexer_reader.indexer_db.event_v2_translation_enabled() {
                return Ok(db_indexer_reader
                    .indexer_db
                    .get_translated_v1_event_by_version_and_index(version, index)?);
            } else {
                anyhow::bail!("Event translation is not enabled")
            }
        }
        anyhow::bail!("DB indexer reader is not available")
    }

    fn translate_event_v2_to_v1(
        &self,
        v2: &ContractEventV2,
    ) -> anyhow::Result<Option<ContractEventV1>> {
        if let Some(db_indexer_reader) = &self.db_indexer_reader {
            if db_indexer_reader.indexer_db.event_v2_translation_enabled() {
                return Ok(db_indexer_reader.translate_event_v2_to_v1(v2)?);
            } else {
                anyhow::bail!("Event translation is not enabled")
            }
        }
        anyhow::bail!("DB indexer reader is not available")
    }
}

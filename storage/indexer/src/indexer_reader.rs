// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{db_indexer::DBIndexer, db_v2::IndexerAsyncV2};
use anyhow::{bail, Result};
use aptos_types::{
    account_address::AccountAddress,
    contract_event::EventWithVersion,
    event::EventKey,
    indexer::indexer_db_reader::{IndexerReader, Order},
    state_store::table::{TableHandle, TableInfo},
    transaction::{AccountTransactionsWithProof, Version},
};
use std::sync::Arc;

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
    fn get_table_info(&self, handle: TableHandle) -> Result<Option<TableInfo>> {
        if let Some(table_info_reader) = &self.table_info_reader {
            return Ok(table_info_reader.get_table_info_with_retry(handle)?);
        }
        bail!("Table info reader is not available")
    }

    fn get_events(
        &self,
        event_key: &EventKey,
        start: u64,
        order: Order,
        limit: u64,
        ledger_version: Version,
    ) -> Result<Vec<EventWithVersion>> {
        if let Some(db_indexer_reader) = &self.db_indexer_reader {
            if db_indexer_reader.event_enabled() {
                return db_indexer_reader.get_events(
                    event_key,
                    start,
                    order,
                    limit,
                    ledger_version,
                );
            } else {
                bail!("Event index is not enabled")
            }
        }
        bail!("DB Indexer reader is not available")
    }

    fn get_events_by_event_key(
        &self,
        event_key: &EventKey,
        start_seq_num: u64,
        order: Order,
        limit: u64,
        ledger_version: Version,
    ) -> Result<Vec<EventWithVersion>> {
        if let Some(db_indexer_reader) = &self.db_indexer_reader {
            if db_indexer_reader.event_enabled() {
                return db_indexer_reader.get_events_by_event_key(
                    event_key,
                    start_seq_num,
                    order,
                    limit,
                    ledger_version,
                );
            } else {
                bail!("Event index is not enabled")
            }
        }
        bail!("DB indexer reader is not available")
    }

    fn get_account_transactions(
        &self,
        address: AccountAddress,
        start_seq_num: u64,
        limit: u64,
        include_events: bool,
        ledger_version: Version,
    ) -> Result<AccountTransactionsWithProof> {
        if let Some(db_indexer_reader) = &self.db_indexer_reader {
            if db_indexer_reader.transaction_enabled() {
                return db_indexer_reader.get_account_transactions(
                    address,
                    start_seq_num,
                    limit,
                    include_events,
                    ledger_version,
                );
            } else {
                bail!("Transaction by account index is not enabled")
            }
        }
        bail!("DB indexer reader is not available")
    }
}

// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    database::{execute_with_better_error, get_chunks, PgDbPool, PgPoolConnection},
    indexer::{
        errors::TransactionProcessingError, processing_result::ProcessingResult,
        transaction_processor::TransactionProcessor,
    },
    models::{
        events::EventModel,
        transactions::{BlockMetadataTransactionModel, TransactionModel, UserTransactionModel},
        write_set_changes::WriteSetChangeModel,
    },
    schema,
};
use aptos_rest_client::Transaction;
use async_trait::async_trait;
use diesel::result::Error;
use field_count::FieldCount;
use std::fmt::Debug;

pub struct DefaultTransactionProcessor {
    connection_pool: PgDbPool,
}

impl DefaultTransactionProcessor {
    pub fn new(connection_pool: PgDbPool) -> Self {
        Self { connection_pool }
    }
}

impl Debug for DefaultTransactionProcessor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state = &self.connection_pool.state();
        write!(
            f,
            "DefaultTransactionProcessor {{ connections: {:?}  idle_connections: {:?} }}",
            state.connections, state.idle_connections
        )
    }
}

fn insert_events(conn: &PgPoolConnection, events: &[EventModel]) {
    let chunks = get_chunks(events.len(), EventModel::field_count());
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::events::table)
                .values(&events[start_ind..end_ind])
                .on_conflict_do_nothing(),
        )
        .expect("Error inserting row into database");
    }
}

fn insert_write_set_changes(conn: &PgPoolConnection, write_set_changes: &[WriteSetChangeModel]) {
    let chunks = get_chunks(write_set_changes.len(), WriteSetChangeModel::field_count());
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::write_set_changes::table)
                .values(&write_set_changes[start_ind..end_ind])
                .on_conflict_do_nothing(),
        )
        .expect("Error inserting row into database");
    }
}

fn insert_transactions(conn: &PgPoolConnection, txns: &[TransactionModel]) {
    let chunks = get_chunks(txns.len(), TransactionModel::field_count());
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::transactions::table)
                .values(&txns[start_ind..end_ind])
                .on_conflict_do_nothing(),
        )
        .expect("Error inserting row into database");
    }
}

fn insert_user_transactions(conn: &PgPoolConnection, user_txns: &[UserTransactionModel]) {
    let chunks = get_chunks(user_txns.len(), UserTransactionModel::field_count());
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::user_transactions::table)
                .values(&user_txns[start_ind..end_ind])
                .on_conflict_do_nothing(),
        )
        .expect("Error inserting row into database");
    }
}

fn insert_block_metadata_transactions(
    conn: &PgPoolConnection,
    bm_txns: &[BlockMetadataTransactionModel],
) {
    let chunks = get_chunks(bm_txns.len(), UserTransactionModel::field_count());
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::block_metadata_transactions::table)
                .values(&bm_txns[start_ind..end_ind])
                .on_conflict_do_nothing(),
        )
        .expect("Error inserting row into database");
    }
}

fn insert_block(
    conn: &PgPoolConnection,
    name: &'static str,
    block_height: u64,
    txns: Vec<TransactionModel>,
    user_txns: Vec<UserTransactionModel>,
    bm_txns: Vec<BlockMetadataTransactionModel>,
    events: Vec<EventModel>,
    wscs: Vec<WriteSetChangeModel>,
) -> Result<(), Error> {
    aptos_logger::trace!("[{}] inserting block {}", name, block_height);
    conn.build_transaction()
        .read_write()
        .run::<_, Error, _>(|| {
            insert_transactions(conn, &txns);
            insert_user_transactions(conn, &user_txns);
            insert_block_metadata_transactions(conn, &bm_txns);
            insert_events(conn, &events);
            insert_write_set_changes(conn, &wscs);
            Ok(())
        })
}

#[async_trait]
impl TransactionProcessor for DefaultTransactionProcessor {
    fn name(&self) -> &'static str {
        "default_processor"
    }

    async fn process_transactions(
        &self,
        transactions: Vec<Transaction>,
        block_height: u64,
    ) -> Result<ProcessingResult, TransactionProcessingError> {
        let (txns, user_txns, bm_txns, events, write_set_changes) =
            TransactionModel::from_transactions(&transactions);

        let conn = self.get_conn();
        let tx_result = insert_block(
            &conn,
            self.name(),
            block_height,
            txns,
            user_txns,
            bm_txns,
            events,
            write_set_changes,
        );
        match tx_result {
            Ok(_) => Ok(ProcessingResult::new(self.name(), block_height)),
            Err(err) => Err(TransactionProcessingError::TransactionCommitError((
                anyhow::Error::from(err),
                block_height,
                self.name(),
            ))),
        }
    }

    fn connection_pool(&self) -> &PgDbPool {
        &self.connection_pool
    }
}

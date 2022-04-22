// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    database::{execute_with_better_error, PgDbPool, PgPoolConnection},
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
use diesel::Connection;
use futures::future::Either;
use std::{fmt::Debug, sync::Arc};

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

fn insert_events(conn: &PgPoolConnection, events: &Vec<EventModel>) {
    execute_with_better_error(
        conn,
        diesel::insert_into(schema::events::table)
            .values(events)
            .on_conflict_do_nothing(),
    )
    .expect("Error inserting row into database");
}

fn insert_write_set_changes(conn: &PgPoolConnection, write_set_changes: &Vec<WriteSetChangeModel>) {
    execute_with_better_error(
        conn,
        diesel::insert_into(schema::write_set_changes::table)
            .values(write_set_changes)
            .on_conflict_do_nothing(),
    )
    .expect("Error inserting row into database");
}

fn insert_transaction(conn: &PgPoolConnection, version: u64, transaction_model: &TransactionModel) {
    aptos_logger::trace!(
        "[default_processor] inserting 'transaction' version {} with hash {}",
        version,
        transaction_model.hash
    );
    execute_with_better_error(
        conn,
        diesel::insert_into(schema::transactions::table)
            .values(transaction_model)
            .on_conflict(schema::transactions::dsl::hash)
            .do_update()
            .set(transaction_model),
    )
    .expect("Error inserting row into database");
}

fn insert_user_transaction(
    conn: &PgPoolConnection,
    version: u64,
    transaction_model: &TransactionModel,
    user_transaction_model: &UserTransactionModel,
) {
    aptos_logger::trace!(
        "[default_processor] inserting 'user_transaction' version {} with hash {}",
        version,
        &transaction_model.hash
    );
    execute_with_better_error(
        conn,
        diesel::insert_into(schema::user_transactions::table)
            .values(user_transaction_model)
            .on_conflict(schema::user_transactions::dsl::hash)
            .do_update()
            .set(user_transaction_model),
    )
    .expect("Error inserting row into database");
}

fn insert_block_metadata_transaction(
    conn: &PgPoolConnection,
    version: u64,
    transaction_model: &TransactionModel,
    block_metadata_transaction_model: &BlockMetadataTransactionModel,
) {
    aptos_logger::trace!(
        "[default_processor] inserting 'block_metadata_transaction' version {} with hash {}",
        version,
        &transaction_model.hash
    );
    execute_with_better_error(
        conn,
        diesel::insert_into(schema::block_metadata_transactions::table)
            .values(block_metadata_transaction_model)
            .on_conflict(schema::block_metadata_transactions::dsl::hash)
            .do_update()
            .set(block_metadata_transaction_model),
    )
    .expect("Error inserting row into database");
}

#[async_trait]
impl TransactionProcessor for DefaultTransactionProcessor {
    fn name(&self) -> &'static str {
        "default_processor"
    }

    async fn process_transaction(
        &self,
        transaction: Arc<Transaction>,
    ) -> Result<ProcessingResult, TransactionProcessingError> {
        let version = transaction.version().unwrap_or(0);

        let (transaction_model, maybe_details_model, maybe_events, maybe_write_set_changes) =
            TransactionModel::from_transaction(&transaction);

        let conn = self.get_conn();

        let tx_result = conn.transaction::<(), diesel::result::Error, _>(|| {
            insert_transaction(&conn, version, &transaction_model);
            if let Some(tx_details_model) = maybe_details_model {
                match tx_details_model {
                    Either::Left(user_transaction_model) => {
                        insert_user_transaction(
                            &conn,
                            version,
                            &transaction_model,
                            &user_transaction_model,
                        );
                    }
                    Either::Right(block_metadata_transaction_model) => {
                        insert_block_metadata_transaction(
                            &conn,
                            version,
                            &transaction_model,
                            &block_metadata_transaction_model,
                        );
                    }
                };
            };

            if let Some(events) = maybe_events {
                insert_events(&conn, &events);
            };
            if let Some(write_set_changes) = maybe_write_set_changes {
                insert_write_set_changes(&conn, &write_set_changes);
            };
            Ok(())
        });

        match tx_result {
            Ok(_) => Ok(ProcessingResult::new(self.name(), version)),
            Err(err) => Err(TransactionProcessingError::TransactionCommitError((
                anyhow::Error::from(err),
                version,
                self.name(),
            ))),
        }
    }

    fn connection_pool(&self) -> &PgDbPool {
        &self.connection_pool
    }
}

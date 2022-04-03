// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    database::{execute_with_better_error, PgDbPool},
    indexer::{
        errors::TransactionProcessingError, processing_result::ProcessingResult,
        transaction_processor::TransactionProcessor,
    },
    models::transactions::TransactionModel,
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

        let (transaction_model, maybe_details_model, maybe_events) =
            TransactionModel::from_transaction(&transaction);

        let conn = self.connection_pool.get().map_err(|e| {
            TransactionProcessingError::ConnectionPoolError((
                anyhow::Error::from(e),
                version,
                self.name(),
            ))
        })?;

        let tx_result = conn.transaction::<(), diesel::result::Error, _>(||{
        aptos_logger::debug!(
            "[default_processor] inserting 'transaction' version {} with hash {}",
            version,
            &transaction_model.hash
        );
        execute_with_better_error(
            &conn,
            diesel::insert_into(schema::transactions::table)
                .values(&transaction_model)
                .on_conflict(schema::transactions::dsl::hash)
                .do_update()
                .set(&transaction_model),
        )
        .expect("Error inserting row into database");

        if let Some(tx_details_model) = maybe_details_model {
            match tx_details_model {
                Either::Left(ut) => {
                    aptos_logger::debug!(
                        "[default_processor] inserting 'user_transaction' version {} with hash {}",
                        version,
                        &transaction_model.hash
                    );
                    execute_with_better_error(
                        &conn,
                        diesel::insert_into(schema::user_transactions::table)
                            .values(&ut)
                            .on_conflict(schema::user_transactions::dsl::hash)
                            .do_update()
                            .set(&ut),
                    )
                    .expect("Error inserting row into database");
                }
                Either::Right(bmt) => {
                    aptos_logger::debug!(
                        "[default_processor] inserting 'block_metadata_transaction' version {} with hash {}",
                        version,
                        &transaction_model.hash
                    );
                    execute_with_better_error(
                        &conn,
                        diesel::insert_into(schema::block_metadata_transactions::table)
                            .values(&bmt)
                            .on_conflict(schema::block_metadata_transactions::dsl::hash)
                            .do_update()
                            .set(&bmt),
                    )
                    .expect("Error inserting row into database");
                }
            };
        };

        if let Some(events) = maybe_events {
            execute_with_better_error(
                &conn,
                diesel::insert_into(schema::events::table)
                    .values(events)
                    .on_conflict_do_nothing(),
            )
            .expect("Error inserting row into database");
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

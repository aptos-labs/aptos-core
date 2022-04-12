// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    counters::{
        GOT_CONNECTION, PROCESSOR_ERRORS, PROCESSOR_INVOCATIONS, PROCESSOR_SUCCESSES,
        UNABLE_TO_GET_CONNECTION,
    },
    database::{execute_with_better_error, PgDbPool, PgPoolConnection},
    indexer::{errors::TransactionProcessingError, processing_result::ProcessingResult},
    models::processor_statuses::ProcessorStatusModel,
    schema,
};
use aptos_rest_client::Transaction;
use async_trait::async_trait;
use diesel::{prelude::*, RunQueryDsl};
use schema::processor_statuses::{self, dsl};
use std::{fmt::Debug, sync::Arc};

/// The `TransactionProcessor` is used by an instance of a `Tailer` to process transactions
#[async_trait]
pub trait TransactionProcessor: Send + Sync + Debug {
    /// name of the processor, for status logging
    /// This will get stored in the database for each (`TransactionProcessor`, transaction_version) pair
    fn name(&self) -> &'static str;

    /// Accepts a transaction, and processes it. This method will be called from `process_transaction_with_status`
    /// In case a transaction cannot be processed, returns an error: the `Tailer` will mark it as failed in the database,
    /// and it will be retried next time the indexer is started.
    async fn process_transaction(
        &self,
        transaction: Arc<Transaction>,
    ) -> Result<ProcessingResult, TransactionProcessingError>;

    /// Gets a reference to the connection pool
    /// This is used by the `get_conn()` helper below
    fn connection_pool(&self) -> &PgDbPool;

    //* Below are helper methods that don't need to be implemented *//

    /// Gets the connection.
    /// If it was unable to do so (default timeout: 30s), it will keep retrying until it can.
    fn get_conn(&self) -> PgPoolConnection {
        let pool = self.connection_pool();
        loop {
            match pool.get() {
                Ok(conn) => {
                    GOT_CONNECTION.inc();
                    return conn;
                }
                Err(err) => {
                    UNABLE_TO_GET_CONNECTION.inc();
                    aptos_logger::error!(
                        "Could not get DB connection from pool, will retry in {:?}. Err: {:?}",
                        pool.connection_timeout(),
                        err
                    );
                }
            };
        }
    }

    /// This is a helper method, tying together the other helper methods to allow tracking status in the DB
    async fn process_transaction_with_status(
        &self,
        transaction: Arc<Transaction>,
    ) -> Result<ProcessingResult, TransactionProcessingError> {
        PROCESSOR_INVOCATIONS
            .with_label_values(&[self.name()])
            .inc();

        self.mark_version_started(transaction.version().unwrap());
        let res = self.process_transaction(transaction).await;
        // Handle version success/failure
        match res.as_ref() {
            Ok(processing_result) => self.update_status_success(processing_result),
            Err(tpe) => self.update_status_err(tpe),
        };
        res
    }

    /// Writes that a version has been started for this `TransactionProcessor` to the DB
    fn mark_version_started(&self, version: u64) {
        aptos_logger::debug!(
            "[{}] Marking processing version started: {}",
            self.name(),
            version
        );
        let psm = ProcessorStatusModel::for_mark_started(self.name(), version as i64);
        self.apply_processor_status(&psm);
    }

    /// Writes that a version has been completed successfully for this `TransactionProcessor` to the DB
    fn update_status_success(&self, processing_result: &ProcessingResult) {
        aptos_logger::debug!(
            "[{}] Marking processing version OK: {}",
            self.name(),
            processing_result.version
        );
        PROCESSOR_SUCCESSES.with_label_values(&[self.name()]).inc();
        let psm = ProcessorStatusModel::from_processing_result_ok(processing_result);
        self.apply_processor_status(&psm);
    }

    /// Writes that a version has errored for this `TransactionProcessor` to the DB
    fn update_status_err(&self, tpe: &TransactionProcessingError) {
        aptos_logger::debug!(
            "[{}] Marking processing version Err: {:?}",
            self.name(),
            tpe
        );
        PROCESSOR_ERRORS.with_label_values(&[self.name()]).inc();
        let psm = ProcessorStatusModel::from_transaction_processing_err(tpe);
        self.apply_processor_status(&psm);
    }

    /// Actually performs the write for a `ProcessorStatusModel` changeset
    fn apply_processor_status(&self, psm: &ProcessorStatusModel) {
        let conn = self.get_conn();
        execute_with_better_error(
            &conn,
            diesel::insert_into(processor_statuses::table)
                .values(psm)
                .on_conflict((dsl::name, dsl::version))
                .do_update()
                .set(psm),
        )
        .expect("Error updating Processor Status!");
    }

    /// Gets all versions which were not successfully processed for this `TransactionProcessor` from the DB
    /// This is so the `Tailer` can know which versions to retry
    fn get_error_versions(&self) -> Vec<u64> {
        let conn = self.get_conn();

        dsl::processor_statuses
            .select(dsl::version)
            .filter(
                dsl::success
                    .eq(false)
                    .and(dsl::name.eq(self.name().to_string())),
            )
            .load::<i64>(&conn)
            .expect("Error loading the error versions only query")
            .iter()
            .map(|v| *v as u64)
            .collect()
    }

    /// Gets the highest version for this `TransactionProcessor` from the DB
    /// This is so we know where to resume from on restarts
    fn get_max_version(&self) -> Option<u64> {
        let conn = self.get_conn();

        dsl::processor_statuses
            .select(diesel::dsl::max(dsl::version))
            .filter(dsl::name.eq(self.name().to_string()))
            .first::<Option<i64>>(&conn)
            .expect("Error loading the max version query")
            .map(|v| v as u64)
    }
}

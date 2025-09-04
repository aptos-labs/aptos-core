// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    counters::{
        GOT_CONNECTION, LATEST_PROCESSED_VERSION, PROCESSOR_ERRORS, PROCESSOR_INVOCATIONS,
        PROCESSOR_SUCCESSES, UNABLE_TO_GET_CONNECTION,
    },
    database::{execute_with_better_error, get_chunks, PgDbPool, PgPoolConnection},
    indexer::{errors::TransactionProcessingError, processing_result::ProcessingResult},
    models::processor_statuses::ProcessorStatusModel,
    schema,
};
use velor_api_types::Transaction;
use async_trait::async_trait;
use diesel::{pg::upsert::excluded, prelude::*};
use field_count::FieldCount;
use schema::processor_statuses::{self, dsl};
use std::fmt::Debug;

/// The `TransactionProcessor` is used by an instance of a `Tailer` to process transactions
#[async_trait]
pub trait TransactionProcessor: Send + Sync + Debug {
    /// name of the processor, for status logging
    /// This will get stored in the database for each (`TransactionProcessor`, transaction_version) pair
    fn name(&self) -> &'static str;

    /// Process all transactions within a block and processes it. This method will be called from `process_transaction_with_status`
    /// In case a transaction cannot be processed, we will fail the entire block.
    async fn process_transactions(
        &self,
        transactions: Vec<Transaction>,
        start_version: u64,
        end_version: u64,
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
                },
                Err(err) => {
                    UNABLE_TO_GET_CONNECTION.inc();
                    velor_logger::error!(
                        "Could not get DB connection from pool, will retry in {:?}. Err: {:?}",
                        pool.connection_timeout(),
                        err
                    );
                },
            };
        }
    }

    /// This is a helper method, tying together the other helper methods to allow tracking status in the DB
    async fn process_transactions_with_status(
        &self,
        txns: Vec<Transaction>,
    ) -> Result<ProcessingResult, TransactionProcessingError> {
        assert!(
            !txns.is_empty(),
            "Must provide at least one transaction to this function"
        );
        PROCESSOR_INVOCATIONS
            .with_label_values(&[self.name()])
            .inc();

        let start_version = txns.first().unwrap().version().unwrap();
        let end_version = txns.last().unwrap().version().unwrap();

        self.mark_versions_started(start_version, end_version);
        let res = self
            .process_transactions(txns, start_version, end_version)
            .await;
        // Handle block success/failure
        match res.as_ref() {
            Ok(processing_result) => self.update_status_success(processing_result),
            Err(tpe) => self.update_status_err(tpe),
        };
        res
    }

    /// Writes that a version has been started for this `TransactionProcessor` to the DB
    fn mark_versions_started(&self, start_version: u64, end_version: u64) {
        velor_logger::debug!(
            "[{}] Marking processing versions started from versions {} to {}",
            self.name(),
            start_version,
            end_version
        );
        let psms = ProcessorStatusModel::from_versions(
            self.name(),
            start_version,
            end_version,
            false,
            None,
        );
        self.apply_processor_status(&psms);
    }

    /// Writes that a version has been completed successfully for this `TransactionProcessor` to the DB
    fn update_status_success(&self, processing_result: &ProcessingResult) {
        velor_logger::debug!(
            "[{}] Marking processing version OK from versions {} to {}",
            self.name(),
            processing_result.start_version,
            processing_result.end_version
        );
        PROCESSOR_SUCCESSES.with_label_values(&[self.name()]).inc();
        LATEST_PROCESSED_VERSION
            .with_label_values(&[self.name()])
            .set(processing_result.end_version as i64);
        let psms = ProcessorStatusModel::from_versions(
            self.name(),
            processing_result.start_version,
            processing_result.end_version,
            true,
            None,
        );
        self.apply_processor_status(&psms);
    }

    /// Writes that a version has errored for this `TransactionProcessor` to the DB
    fn update_status_err(&self, tpe: &TransactionProcessingError) {
        velor_logger::debug!(
            "[{}] Marking processing version Err: {:?}",
            self.name(),
            tpe
        );
        PROCESSOR_ERRORS.with_label_values(&[self.name()]).inc();
        let psm = ProcessorStatusModel::from_transaction_processing_err(tpe);
        self.apply_processor_status(&psm);
    }

    /// Actually performs the write for a `ProcessorStatusModel` changeset
    fn apply_processor_status(&self, psms: &[ProcessorStatusModel]) {
        let mut conn = self.get_conn();
        let chunks = get_chunks(psms.len(), ProcessorStatusModel::field_count());
        for (start_ind, end_ind) in chunks {
            execute_with_better_error(
                &mut conn,
                diesel::insert_into(processor_statuses::table)
                    .values(&psms[start_ind..end_ind])
                    .on_conflict((dsl::name, dsl::version))
                    .do_update()
                    .set((
                        dsl::success.eq(excluded(dsl::success)),
                        dsl::details.eq(excluded(dsl::details)),
                        dsl::last_updated.eq(excluded(dsl::last_updated)),
                    )),
                None,
            )
            .expect("Error updating Processor Status!");
        }
    }
}

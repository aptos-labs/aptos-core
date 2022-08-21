// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    counters::{
        GOT_CONNECTION, PROCESSOR_ERRORS, PROCESSOR_INVOCATIONS, PROCESSOR_SUCCESSES,
        UNABLE_TO_GET_CONNECTION,
    },
    database::{execute_with_better_error, PgDbPool, PgPoolConnection},
    indexer::{errors::TransactionProcessingError, processing_result::ProcessingResult},
    models::v2_processor_statuses::ProcessorStatusModel,
    schema,
};
use aptos_rest_client::Transaction;
use async_trait::async_trait;
use diesel::sql_types::{BigInt, Text};
use diesel::{sql_query, RunQueryDsl};
use schema::v2_processor_statuses::{self, dsl};
use std::fmt::Debug;

/// The `TransactionProcessor` is used by an instance of a `Tailer` to process transactions
#[async_trait]
pub trait TransactionProcessor: Send + Sync + Debug {
    /// name of the processor, for status logging
    /// This will get stored in the database for each (`TransactionProcessor`, transaction_version) pair
    fn name(&self) -> &'static str;

    /// Accepts transactions within a block and processes it. This method will be called from `process_transaction_with_status`
    /// In case a transaction cannot be processed, we will fail the entire block.
    async fn process_transactions(
        &self,
        transactions: Vec<Transaction>,
        block_height: u64,
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
    async fn process_transactions_with_status(
        &self,
        txns: Vec<Transaction>,
        block_height: u64,
    ) -> Result<ProcessingResult, TransactionProcessingError> {
        PROCESSOR_INVOCATIONS
            .with_label_values(&[self.name()])
            .inc();

        self.mark_block_started(block_height);
        let res = self.process_transactions(txns, block_height).await;
        // Handle block success/failure
        match res.as_ref() {
            Ok(processing_result) => self.update_status_success(processing_result),
            Err(tpe) => self.update_status_err(tpe),
        };
        res
    }

    /// Writes that a block has errored for this `TransactionProcessor` to the DB
    fn update_status_err(&self, tpe: &TransactionProcessingError) {
        aptos_logger::debug!("[{}] Marking processing block Err: {:?}", self.name(), tpe);
        PROCESSOR_ERRORS.with_label_values(&[self.name()]).inc();
        let psm = ProcessorStatusModel::from_transaction_processing_err(tpe);
        self.apply_processor_status(&psm);
    }
    /// Writes that a block has been started for this `TransactionProcessor` to the DB
    fn mark_block_started(&self, block_height: u64) {
        aptos_logger::debug!(
            "[{}] Marking processing block started: {}",
            self.name(),
            block_height
        );
        let psm = ProcessorStatusModel::for_mark_started(self.name(), block_height);
        self.apply_processor_status(&psm);
    }

    /// Writes that a block_height has been completed successfully for this `TransactionProcessor` to the DB
    fn update_status_success(&self, processing_result: &ProcessingResult) {
        aptos_logger::debug!(
            "[{}] Marking processing block OK: {}",
            self.name(),
            processing_result.block_height
        );
        PROCESSOR_SUCCESSES.with_label_values(&[self.name()]).inc();
        let psm = ProcessorStatusModel::from_processing_result_ok(processing_result);
        self.apply_processor_status(&psm);
    }

    /// Actually performs the write for a `ProcessorStatusModel` changeset
    fn apply_processor_status(&self, psm: &ProcessorStatusModel) {
        let conn = self.get_conn();
        execute_with_better_error(
            &conn,
            diesel::insert_into(v2_processor_statuses::table)
                .values(psm)
                .on_conflict((dsl::name, dsl::block_height))
                .do_update()
                .set(psm),
        )
        .expect("Error updating Processor Status!");
    }

    /// Gets the highest block for this `SubstreamProcessor` from the DB
    /// This is so we know where to resume from on restarts.
    /// If a block has any unprocessed transactions, we will restart processing the entire block.
    fn get_start_block(&self) -> Option<i64> {
        let conn = self.get_conn();
        let sql = "
        WITH boundaries AS
        (
            SELECT
                MAX(block_height) AS MAX_V,
                MIN(block_height) AS MIN_V
            FROM
                v2_processor_statuses
            WHERE
                name = $1
                AND success = TRUE
        ),
        gap AS
        (
            SELECT
                MIN(block_height) + 1 AS maybe_gap
            FROM
                (
                    SELECT
                        block_height,
                        LEAD(block_height) OVER (
                    ORDER BY
                        block_height ASC) AS next_block_height
                    FROM
                        v2_processor_statuses,
                        boundaries
                    WHERE
                        name = $1
                        AND success = TRUE
                        AND block_height >= MAX_V - 1000000
                ) a
            WHERE
                block_height + 1 <> next_block_height
        )
        SELECT
            CASE
                WHEN
                    MIN_V <> 0
                THEN
                    0
                ELSE
                    COALESCE(maybe_gap, MAX_V + 1)
            END
            AS block_height
        FROM
            gap, boundaries
        ";
        #[derive(Debug, QueryableByName)]
        pub struct Gap {
            #[sql_type = "BigInt"]
            pub block_height: i64,
        }
        let mut res: Vec<Option<Gap>> = sql_query(sql)
            .bind::<Text, _>(self.name())
            .get_results(&conn)
            .unwrap();
        res.pop().unwrap().map(|g| g.block_height)
    }
}

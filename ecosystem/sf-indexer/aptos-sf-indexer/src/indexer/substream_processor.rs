// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    counters::{
        GOT_CONNECTION, PROCESSOR_ERRORS, PROCESSOR_INVOCATIONS, PROCESSOR_SUCCESSES,
        UNABLE_TO_GET_CONNECTION,
    },
    database::{execute_with_better_error, PgDbPool, PgPoolConnection},
    indexer::{errors::BlockProcessingError, processing_result::ProcessingResult},
    models::indexer_states::IndexerState,
    proto::BlockScopedData,
    schema,
};
use aptos_logger::info;
use async_trait::async_trait;
use diesel::{
    sql_query,
    sql_types::{BigInt, Text},
    RunQueryDsl,
};
use schema::indexer_states;
use std::fmt::Debug;

diesel_migrations::embed_migrations!();

/// The `SubstreamProcessor` processes the output from a substream specific to the server instance
#[async_trait]
pub trait SubstreamProcessor: Send + Sync + Debug {
    /// name of the substream module, for status logging
    /// This will get stored in the database for each (substream_module, block_height) pair
    fn substream_module_name(&self) -> &'static str;

    /// Accepts a block, and processes it. This method will be called from `process_substream_with_status`
    /// In case a block cannot be processed, returns an error: the processor will mark it as failed in the database,
    /// and it will be retried next time the indexer is started.
    async fn process_substream(
        &self,
        stream_data: BlockScopedData,
        block_height: u64,
    ) -> Result<ProcessingResult, BlockProcessingError>;

    /// Gets a reference to the connection pool
    /// This is used by the `get_conn()` helper below
    fn connection_pool(&self) -> &PgDbPool;

    //* Below are helper methods that don't need to be implemented *//

    /// Gets the connection.
    /// If it was unable to do so (default timeout: 30s), it will keep retrying until it can.
    /// It's a static method because we need the connection before the processor is initialized
    fn get_conn(pool: &PgDbPool) -> PgPoolConnection {
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
    async fn process_substream_with_status(
        &mut self,
        current_substream_name: String,
        stream_data: BlockScopedData,
        block_height: u64,
    ) -> Result<ProcessingResult, BlockProcessingError> {
        if current_substream_name != self.substream_module_name() {
            panic!("Wrong processor detected: this processor can only process module {},  module {} detected.", self.substream_module_name(), current_substream_name);
        }
        PROCESSOR_INVOCATIONS
            .with_label_values(&[self.substream_module_name()])
            .inc();

        aptos_logger::debug!("Marking block started {}", block_height);
        self.mark_block_started(block_height);
        aptos_logger::debug!("Starting to process stream for block {}", block_height);
        let res = self.process_substream(stream_data, block_height).await;
        // Handle block success/failure
        match res.as_ref() {
            Ok(processing_result) => self.update_status_success(processing_result),
            Err(bpe) => self.update_status_err(bpe),
        };
        res
    }

    /// Writes that a block has been started for this `SubstreamProcessor` to the DB
    fn mark_block_started(&self, block_height: u64) {
        aptos_logger::debug!(
            "[{}] Marking processing block started: {}",
            self.substream_module_name(),
            block_height
        );
        let psm = IndexerState::for_mark_started(
            self.substream_module_name().to_string(),
            block_height as i64,
        );
        self.apply_processor_status(&psm);
    }

    /// Writes that a block has been completed successfully for this `SubstreamProcessor` to the DB
    fn update_status_success(&self, processing_result: &ProcessingResult) {
        aptos_logger::debug!(
            "[{}] Marking processing block OK: block_height {}",
            self.substream_module_name(),
            processing_result.block_height
        );
        PROCESSOR_SUCCESSES
            .with_label_values(&[self.substream_module_name()])
            .inc();
        let psm = IndexerState::from_processing_result_ok(processing_result);
        self.apply_processor_status(&psm);
    }

    /// Writes that a block has errored for this `SubstreamProcessor` to the DB
    fn update_status_err(&self, bpe: &BlockProcessingError) {
        aptos_logger::debug!(
            "[{}] Marking processing block Err: {:?}",
            self.substream_module_name(),
            bpe
        );
        PROCESSOR_ERRORS
            .with_label_values(&[self.substream_module_name()])
            .inc();
        let psm = IndexerState::from_block_processing_err(bpe);
        self.apply_processor_status(&psm);
    }

    /// Actually performs the write for a `IndexerState` changeset
    fn apply_processor_status(&self, psm: &IndexerState) {
        let conn = Self::get_conn(self.connection_pool());
        execute_with_better_error(
            &conn,
            diesel::insert_into(indexer_states::table)
                .values(psm)
                .on_conflict((
                    indexer_states::dsl::substream_module,
                    indexer_states::dsl::block_height,
                ))
                .do_update()
                .set(psm),
        )
        .expect("Error updating Processor Status!");
    }
}

pub fn run_migrations(pool: &PgDbPool) {
    info!("Running migrations...");
    embedded_migrations::run_with_output(
        &pool.get().expect("Could not get connection for migrations"),
        &mut std::io::stdout(),
    )
    .expect("migrations failed!");
    info!("Migrations complete!");
}

/// Gets the highest block for this `SubstreamProcessor` from the DB
/// This is so we know where to resume from on restarts.
/// If a block has any unprocessed transactions, we will restart processing the entire block.
pub fn get_start_block(pool: &PgDbPool, substream_module_name: &String) -> Option<i64> {
    let conn = pool
        .get()
        .expect("Could not get connection for checking starting block");
    let sql = "
        WITH boundaries AS 
        (
            SELECT
                MAX(block_height) AS MAX_V,
                MIN(block_height) AS MIN_V 
            FROM
                indexer_states 
            WHERE
                substream_module = $1 
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
                        indexer_states,
                        boundaries 
                    WHERE
                        substream_module = $1 
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
    let res: Vec<Option<Gap>> = sql_query(sql)
        .bind::<Text, _>(substream_module_name)
        .get_results(&conn)
        .unwrap();
    match res.first() {
        Some(Some(gap)) => Some(gap.block_height),
        _ => None,
    }
}

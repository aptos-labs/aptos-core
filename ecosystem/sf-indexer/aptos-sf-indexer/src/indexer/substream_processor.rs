// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    counters::{
        GOT_CONNECTION, LATEST_PROCESSED_BLOCK, PROCESSOR_ERRORS, PROCESSOR_INVOCATIONS,
        PROCESSOR_SUCCESSES, UNABLE_TO_GET_CONNECTION,
    },
    database::{execute_with_better_error, PgDbPool, PgPoolConnection},
    indexer::{errors::BlockProcessingError, processing_result::ProcessingResult},
    models::{indexer_states::IndexerState, ledger_infos::LedgerInfo},
    proto::BlockScopedData,
    schema,
};
use aptos_logger::info;
use async_trait::async_trait;
use diesel::{
    prelude::*,
    sql_query,
    sql_types::{BigInt, Text},
    RunQueryDsl,
};
use schema::{indexer_states, ledger_infos};
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
        &mut self,
        stream_data: BlockScopedData,
        block_height: u64,
    ) -> Result<ProcessingResult, BlockProcessingError>;

    /// Gets a reference to the connection pool
    /// This is used by the `get_conn()` helper below
    fn connection_pool(&self) -> &PgDbPool;

    /// If not verified, verify that chain id is correct, else panic. These functions
    /// will be called inside of process_substream since chain id can only be accessed inside the protobuf
    fn is_chain_id_verified(&self) -> bool;

    fn set_is_chain_id_verified(&mut self);
    //* Below are helper methods that don't need to be implemented *//

    /// This is a helper method, tying together the other helper methods to allow tracking status in the DB
    async fn process_substream_with_status(
        &mut self,
        stream_data: BlockScopedData,
        block_height: u64,
    ) -> Result<ProcessingResult, BlockProcessingError> {
        PROCESSOR_INVOCATIONS
            .with_label_values(&[self.substream_module_name()])
            .inc();

        aptos_logger::debug!(block_height = block_height, "Marking block started");
        self.mark_block_started(block_height);
        aptos_logger::debug!(block_height = block_height, "Starting to process stream");
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
            substream_module_name = self.substream_module_name(),
            block_height = block_height,
            "Marking processing block started",
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
            substream_module_name = self.substream_module_name(),
            block_height = processing_result.block_height,
            "Marking processing block OK",
        );
        PROCESSOR_SUCCESSES
            .with_label_values(&[self.substream_module_name()])
            .inc();
        LATEST_PROCESSED_BLOCK
            .with_label_values(&[self.substream_module_name()])
            .set(processing_result.block_height as i64);
        let psm = IndexerState::from_processing_result_ok(processing_result);
        self.apply_processor_status(&psm);
    }

    /// Writes that a block has errored for this `SubstreamProcessor` to the DB
    fn update_status_err(&self, bpe: &BlockProcessingError) {
        aptos_logger::debug!(
            substream_module_name = self.substream_module_name(),
            "Marking processing block ERROR. {:?}",
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
        let mut conn = get_conn(self.connection_pool());
        execute_with_better_error(
            &mut conn,
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

    /// If chain id doesn't exist, save it. Otherwise, make sure that we're indexing the same chain
    /// If check is successful, we will call set_is_chain_id_verified to attempt to persist the result
    /// of the check. Make sure to implement this function and call is_chain_id_verified to read the flag.
    fn check_or_update_chain_id(&mut self, input_chain_id: i64) {
        info!("Checking if chain id is correct");
        let mut conn = self
            .connection_pool()
            .get()
            .expect("DB connection is not available to query chain id");

        let chain_in_db = ledger_infos::dsl::ledger_infos
            .select(ledger_infos::dsl::chain_id)
            .load::<i64>(&conn)
            .expect("Error loading chain id from db");
        let chain_in_db = chain_in_db.first();
        match chain_in_db {
            Some(chain_id) => {
                if *chain_id != input_chain_id {
                    panic!("Wrong chain detected! Trying to index chain {} now but existing data is for chain {}", input_chain_id, chain_id);
                }
                info!(chain_id = chain_id, "Chain id matches! Continuing to index");
            }
            None => {
                info!(
                    input_chain_id = input_chain_id,
                    "Adding chain id to db, continue to index",
                );
                execute_with_better_error(
                    &mut conn,
                    diesel::insert_into(ledger_infos::table).values(LedgerInfo {
                        chain_id: input_chain_id,
                    }),
                )
                .unwrap();
            }
        }
        self.set_is_chain_id_verified();
    }
}

/// Gets the connection.
/// If it was unable to do so (default timeout: 30s), it will keep retrying until it can.
/// It's a static method because we need the connection before the processor is initialized
pub(crate) fn get_conn(pool: &PgDbPool) -> PgPoolConnection {
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
    // This query gets the first block where the block height (aka block id) isn't equal to the next
    // block in the list (where block ids are sorted). There's also special handling if the gap happens in the beginning.
    let sql = "
        WITH raw_boundaries AS
        (
            SELECT
                MAX(block_height) AS MAX_BLOCK,
                MIN(block_height) AS MIN_BLOCK
            FROM
                indexer_states
            WHERE
                substream_module = $1
                AND success = TRUE
        ),
        boundaries AS
        (
            SELECT
                MAX(block_height) AS MAX_BLOCK,
                MIN(block_height) AS MIN_BLOCK
            FROM
                indexer_states, raw_boundaries
            WHERE
                substream_module = $1
                AND success = true
                and block_height >= GREATEST(MAX_BLOCK - $2, 0)

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
                        AND block_height >= GREATEST(MAX_BLOCK - $2, 0)
                ) a
            WHERE
                block_height + 1 <> next_block_height
        )
        SELECT
            CASE
                WHEN
                    MIN_BLOCK <> GREATEST(MAX_BLOCK - $2, 0)
                THEN
                    GREATEST(MAX_BLOCK - $2, 0)
                ELSE
                    COALESCE(maybe_gap, MAX_BLOCK + 1)
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
        .bind::<Text, _>(substream_module_name)
        // This is the number used to determine how far we look back for gaps. Increasing it may result in slower startup
        .bind::<BigInt, _>(1500000)
        .get_results(&conn)
        .unwrap();
    res.pop().unwrap().map(|g| g.block_height)
}

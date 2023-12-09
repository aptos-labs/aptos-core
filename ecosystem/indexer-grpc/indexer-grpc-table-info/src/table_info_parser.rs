// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Error;
use aptos_api::context::Context;
use aptos_api_types::TransactionOnChainData;
use aptos_db_indexer_async_v2::counters::{
    IndexerTableInfoStep, DURATION_IN_SECS, SERVICE_TYPE, TOTAL_SIZE_IN_BYTES,
};
use aptos_logger::{debug, error, info, sample, sample::SampleRate};
use aptos_sdk::bcs;
use aptos_storage_interface::DbWriter;
use aptos_types::write_set::WriteSet;
use std::{sync::Arc, time::Duration};
use tonic::Status;

type EndVersion = u64;
pub const LEDGER_VERSION_RETRY_TIME_MILLIS: u64 = 10;
const DEFAULT_NUM_RETRIES: usize = 3;

pub struct TableInfoParser {
    pub current_version: u64,
    pub parser_task_count: u16,
    pub parser_batch_size: u16,
    pub highest_known_version: u64,
    pub context: Arc<Context>,
}

pub struct TransactionBatchInfo {
    pub start_version: u64,
    pub num_transactions_to_fetch: u16,
}

impl TableInfoParser {
    pub fn new(
        context: Arc<Context>,
        request_start_version: u64,
        parser_task_count: u16,
        parser_batch_size: u16,
    ) -> Self {
        Self {
            current_version: request_start_version,
            parser_task_count,
            parser_batch_size,
            highest_known_version: 0,
            context,
        }
    }

    /// Fans out a bunch of threads and processes write sets from transactions in parallel.
    /// Pushes results in parallel to the stream, but only return that the batch is
    /// fully completed if every job in the batch is successful
    /// Processing transactions in 2 stages:
    /// 1. Fetch transactions from storage
    /// 2. Get write sets from transactions and parse write sets to get handle -> key,value type mapping, write the mapping to the rocksdb
    pub async fn process_next_batch(
        &mut self,
        db_writer: Arc<dyn DbWriter>,
    ) -> Vec<Result<EndVersion, Status>> {
        let mut tasks = vec![];
        let batches = self.get_batches().await;
        let db_writer = db_writer.clone();

        for batch in batches {
            let start_time = std::time::Instant::now();
            let db_writer = db_writer.clone();
            let context = self.context.clone();
            let ledger_version = self.highest_known_version;
            let task = tokio::spawn(async move {
                let raw_txns =
                    Self::fetch_raw_txns_with_retries(context.clone(), ledger_version, batch).await;
                Self::parse_table_info(context.clone(), raw_txns.clone(), db_writer.clone())
                    .await
                    .expect("Failed to parse table info");

                DURATION_IN_SECS
                    .with_label_values(&[
                        SERVICE_TYPE,
                        IndexerTableInfoStep::TableInfoProcessedBatch.get_step(),
                        IndexerTableInfoStep::TableInfoProcessedBatch.get_label(),
                    ])
                    .set(start_time.elapsed().as_secs_f64());
                TOTAL_SIZE_IN_BYTES
                    .with_label_values(&[
                        SERVICE_TYPE,
                        IndexerTableInfoStep::TableInfoProcessedBatch.get_step(),
                        IndexerTableInfoStep::TableInfoProcessedBatch.get_label(),
                    ])
                    .set(bcs::serialized_size(&raw_txns).unwrap() as i64);

                Ok(raw_txns.last().unwrap().version)
            });
            tasks.push(task);
        }
        match futures::future::try_join_all(tasks).await {
            Ok(res) => res,
            Err(err) => panic!("Error processing table info batches: {:?}", err),
        }
    }

    pub fn get_max_batch_version(
        results: Vec<Result<EndVersion, Status>>,
    ) -> Result<EndVersion, Status> {
        let mut max_version = 0;
        for result in results {
            match result {
                Ok(version) => {
                    max_version = std::cmp::max(max_version, version);
                },
                Err(err) => {
                    return Err(err);
                },
            }
        }
        Ok(max_version)
    }

    async fn get_batches(&mut self) -> Vec<TransactionBatchInfo> {
        self.ensure_highest_known_version().await;

        info!(
            current_version = self.current_version,
            highest_known_version = self.highest_known_version,
            parser_batch_size = self.parser_batch_size,
            parser_task_count = self.parser_task_count,
            "Preparing to fetch transactions"
        );

        let mut starting_version = self.current_version;
        let mut num_fetches = 0;
        let mut batches = vec![];

        while num_fetches < self.parser_task_count && starting_version <= self.highest_known_version
        {
            let num_transactions_to_fetch = std::cmp::min(
                self.parser_batch_size as u64,
                self.highest_known_version - starting_version + 1,
            ) as u16;

            batches.push(TransactionBatchInfo {
                start_version: starting_version,
                num_transactions_to_fetch,
            });
            starting_version += num_transactions_to_fetch as u64;
            num_fetches += 1;
        }
        batches
    }

    async fn fetch_raw_txns_with_retries(
        context: Arc<Context>,
        ledger_version: u64,
        batch: TransactionBatchInfo,
    ) -> Vec<TransactionOnChainData> {
        let mut retries = 0;
        loop {
            match context.get_transactions(
                batch.start_version,
                batch.num_transactions_to_fetch,
                ledger_version,
            ) {
                Ok(raw_txns) => return raw_txns,
                Err(err) => {
                    retries += 1;

                    if retries >= DEFAULT_NUM_RETRIES {
                        error!(
                            starting_version = batch.start_version,
                            num_transactions = batch.num_transactions_to_fetch,
                            error = format!("{:?}", err),
                            "Could not fetch transactions: retries exhausted",
                        );
                        panic!(
                            "Could not fetch {} transactions after {} retries, starting at {}: {:?}",
                            batch.num_transactions_to_fetch, retries, batch.start_version, err
                        );
                    } else {
                        error!(
                            starting_version = batch.start_version,
                            num_transactions = batch.num_transactions_to_fetch,
                            error = format!("{:?}", err),
                            "Could not fetch transactions: will retry",
                        );
                    }
                    tokio::time::sleep(Duration::from_millis(300)).await;
                },
            }
        }
    }

    async fn parse_table_info(
        context: Arc<Context>,
        raw_txns: Vec<TransactionOnChainData>,
        db_writer: Arc<dyn DbWriter>,
    ) -> Result<(), Error> {
        if raw_txns.is_empty() {
            return Ok(());
        }

        let start_millis = chrono::Utc::now().naive_utc();
        let first_version = raw_txns.first().map(|txn| txn.version).unwrap();
        let ledger_chain_id = context.chain_id().id();

        let write_sets: Vec<WriteSet> = raw_txns.iter().map(|txn| txn.changes.clone()).collect();
        let write_sets_slice: Vec<&WriteSet> = write_sets.iter().collect();
        db_writer
            .clone()
            .index_table_info(context.db.clone(), first_version, &write_sets_slice)
            .expect("Failed to process write sets and index to the table info rocksdb");
        let fetch_millis = (chrono::Utc::now().naive_utc() - start_millis).num_milliseconds();

        info!(
            table_info_first_version = first_version,
            ledger_chain_id = ledger_chain_id,
            write_sets_bcs_size = bcs::serialized_size(&write_sets).unwrap(),
            table_info_parsing_millis_per_batch = fetch_millis,
            num_transactions = raw_txns.len(),
            "Table info parsed successfully"
        );

        Ok(())
    }

    pub fn set_highest_known_version(&mut self) -> anyhow::Result<()> {
        let info = self.context.get_latest_ledger_info_wrapped()?;
        self.highest_known_version = info.ledger_version.0;
        Ok(())
    }

    /// Will keep looping and checking the latest ledger info to see if there are new transactions
    /// If there are, it will set the highest known version
    async fn ensure_highest_known_version(&mut self) {
        let mut empty_loops = 0;
        while self.highest_known_version == 0 || self.current_version > self.highest_known_version {
            if empty_loops > 0 {
                tokio::time::sleep(Duration::from_millis(LEDGER_VERSION_RETRY_TIME_MILLIS)).await;
            }
            empty_loops += 1;
            if let Err(err) = self.set_highest_known_version() {
                error!(
                    error = format!("{:?}", err),
                    "Failed to set highest known version"
                );
                continue;
            } else {
                sample!(
                    SampleRate::Frequency(100),
                    debug!(
                        highest_known_version = self.highest_known_version,
                        "Found new highest known version",
                    )
                );
            }
        }
    }
}

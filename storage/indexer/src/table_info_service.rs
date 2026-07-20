// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::db_v2::IndexerAsyncV2;
use aptos_logger::{debug, error, info, sample, sample::SampleRate};
use aptos_storage_interface::DbReader;
use aptos_types::{transaction::Version, write_set::WriteSet};
use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

const LEDGER_VERSION_RETRY_TIME_MILLIS: u64 = 10;
const PARSE_RETRY_TIME_SECS: u64 = 5;

/// A batch of contiguous transaction versions to parse.
#[derive(Clone, Copy, Debug)]
struct TransactionBatchInfo {
    start_version: Version,
    num_transactions_to_parse: u16,
}

/// TableInfoService is responsible for parsing table info from the write sets of committed
/// transactions and writing them to the table info rocksdb, so that the API layer can translate
/// table handles into typed keys/values. Not thread safe.
pub struct TableInfoService {
    pub parser_task_count: u16,
    pub parser_batch_size: u16,
    pub main_db_reader: Arc<dyn DbReader>,
    pub indexer_async_v2: Arc<IndexerAsyncV2>,

    current_version: AtomicU64,
    aborted: AtomicBool,
}

impl TableInfoService {
    pub fn new(
        main_db_reader: Arc<dyn DbReader>,
        request_start_version: Version,
        parser_task_count: u16,
        parser_batch_size: u16,
        indexer_async_v2: Arc<IndexerAsyncV2>,
    ) -> Self {
        Self {
            current_version: AtomicU64::new(request_start_version),
            parser_task_count,
            parser_batch_size,
            main_db_reader,
            indexer_async_v2,
            aborted: AtomicBool::new(false),
        }
    }

    pub fn abort(&self) {
        self.aborted.store(true, Ordering::SeqCst);
    }

    pub fn next_version(&self) -> Version {
        self.current_version.load(Ordering::SeqCst)
    }

    /// Main loop: wait for new transactions in the main DB, parse their write sets in parallel
    /// batches and persist the extracted table info.
    pub async fn run(&self) {
        loop {
            let start_time = std::time::Instant::now();
            let ledger_version = self.get_highest_known_version().await;
            if self.aborted.load(Ordering::SeqCst) {
                info!("[Table Info] Service aborted");
                break;
            }
            let batches = self.get_batches(ledger_version);
            let last_version = match self.process_batches_in_parallel(&batches).await {
                Some(last_version) => last_version,
                None => continue,
            };

            let start_version = self.current_version.load(Ordering::SeqCst);
            info!(
                start_version = start_version,
                end_version = last_version,
                num_versions = last_version + 1 - start_version,
                duration_secs = start_time.elapsed().as_secs_f64(),
                "[Table Info] Processed transactions",
            );
            self.current_version
                .store(last_version + 1, Ordering::SeqCst);
        }
    }

    /// Fans out parsing tasks, one per batch, and waits for all of them. Returns the last
    /// version processed, or None if there was nothing to process.
    ///
    /// Parsing table info from a write op may fail transiently when the value type of a nested
    /// table is not known yet (the parent table's write op may be processed by a later batch in
    /// the same round); such items are parked in the `pending_on` map of [`IndexerAsyncV2`] and
    /// re-parsed sequentially at the end of the round.
    async fn process_batches_in_parallel(
        &self,
        batches: &[TransactionBatchInfo],
    ) -> Option<Version> {
        let last_batch = batches.last()?;
        let end_version =
            last_batch.start_version + last_batch.num_transactions_to_parse as u64 - 1;

        let mut tasks = vec![];
        for batch in batches.iter().copied() {
            let db_reader = self.main_db_reader.clone();
            let indexer_async_v2 = self.indexer_async_v2.clone();
            tasks.push(tokio::spawn(async move {
                Self::process_batch(
                    db_reader,
                    indexer_async_v2,
                    batch.start_version,
                    batch.num_transactions_to_parse as u64,
                )
                .await
            }));
        }
        for task in tasks {
            task.await.expect("[Table Info] Parsing task panicked");
        }

        // If pending on items are not empty, meaning the current round hasn't fully parsed all
        // table infos due to nested tables spanning batches, retry sequentially to ensure
        // parsing is complete.
        if !self.indexer_async_v2.is_indexer_async_v2_pending_on_empty() {
            self.indexer_async_v2.clear_pending_on();
            let start_version = batches.first().expect("batches is non-empty").start_version;
            Self::process_batch(
                self.main_db_reader.clone(),
                self.indexer_async_v2.clone(),
                start_version,
                end_version + 1 - start_version,
            )
            .await;
        }

        assert!(
            self.indexer_async_v2.is_indexer_async_v2_pending_on_empty(),
            "Missing data in table info parsing after sequential retry"
        );

        // Update rocksdb's next version to be processed after verifying all txns in this round
        // are successfully parsed.
        self.indexer_async_v2
            .update_next_version(end_version + 1)
            .expect("Failed to update next version of the table info db");

        Some(end_version)
    }

    /// Reads the write sets of a range of transactions from the main DB and parses them for
    /// table info. Retries forever on failure to avoid taking down the rest of the node's
    /// functionality.
    async fn process_batch(
        db_reader: Arc<dyn DbReader>,
        indexer_async_v2: Arc<IndexerAsyncV2>,
        start_version: Version,
        num_transactions: u64,
    ) {
        loop {
            match Self::parse_table_info(
                db_reader.clone(),
                indexer_async_v2.clone(),
                start_version,
                num_transactions,
            ) {
                Ok(()) => return,
                Err(e) => {
                    error!(
                        start_version = start_version,
                        num_transactions = num_transactions,
                        error = ?e,
                        "[Table Info] Error during parse_table_info, retrying",
                    );
                    tokio::time::sleep(Duration::from_secs(PARSE_RETRY_TIME_SECS)).await;
                },
            }
        }
    }

    fn parse_table_info(
        db_reader: Arc<dyn DbReader>,
        indexer_async_v2: Arc<IndexerAsyncV2>,
        start_version: Version,
        num_transactions: u64,
    ) -> anyhow::Result<()> {
        let write_sets: Vec<WriteSet> = db_reader
            .get_write_set_iterator(start_version, num_transactions)?
            .collect::<Result<Vec<_>, _>>()?;
        let write_set_refs = write_sets.iter().collect::<Vec<_>>();
        indexer_async_v2.index_table_info(db_reader, start_version, &write_set_refs)?;
        Ok(())
    }

    /// Chunks the transactions between the current version and the given ledger version into
    /// up to `parser_task_count` batches of at most `parser_batch_size` transactions.
    fn get_batches(&self, ledger_version: Version) -> Vec<TransactionBatchInfo> {
        let mut start_version = self.current_version.load(Ordering::SeqCst);
        info!(
            current_version = start_version,
            highest_known_version = ledger_version,
            parser_batch_size = self.parser_batch_size,
            parser_task_count = self.parser_task_count,
            "[Table Info] Preparing to parse transactions"
        );

        let mut num_batches = 0;
        let mut batches = vec![];
        while num_batches < self.parser_task_count && start_version <= ledger_version {
            let num_transactions_to_parse = std::cmp::min(
                self.parser_batch_size as u64,
                ledger_version + 1 - start_version,
            ) as u16;
            batches.push(TransactionBatchInfo {
                start_version,
                num_transactions_to_parse,
            });
            start_version += num_transactions_to_parse as u64;
            num_batches += 1;
        }
        batches
    }

    /// Polls the main DB until its synced version reaches the current version, i.e. there is
    /// something new to parse.
    async fn get_highest_known_version(&self) -> Version {
        let mut empty_loops = 0;
        loop {
            if self.aborted.load(Ordering::SeqCst) {
                return 0;
            }
            if empty_loops > 0 {
                tokio::time::sleep(Duration::from_millis(LEDGER_VERSION_RETRY_TIME_MILLIS)).await;
            }
            empty_loops += 1;

            match self.main_db_reader.get_synced_version() {
                Ok(Some(synced_version))
                    if synced_version > 0
                        && synced_version >= self.current_version.load(Ordering::SeqCst) =>
                {
                    sample!(
                        SampleRate::Frequency(100),
                        debug!(
                            ledger_version = synced_version,
                            "[Table Info] Found new highest known ledger version",
                        )
                    );
                    return synced_version;
                },
                Ok(_) => (),
                Err(err) => {
                    error!(
                        error = format!("{:?}", err),
                        "[Table Info] Failed to get synced version"
                    );
                },
            }
        }
    }
}

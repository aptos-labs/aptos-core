// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Error;
use aptos_api::context::Context;
use aptos_api_types::TransactionOnChainData;
use aptos_db_indexer_async_v2::backup_restore_operator::gcs::GcsBackupRestoreOperator;
use aptos_logger::{error, info, sample, sample::SampleRate};
use aptos_storage_interface::DbWriter;
use aptos_types::write_set::WriteSet;
use std::sync::Arc;
use tonic::Status;

type EndVersion = u64;

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

    pub async fn process_next_batch(
        &mut self,
        db_writer: Arc<dyn DbWriter>,
        backup_restore_operator: Arc<GcsBackupRestoreOperator>,
    ) -> Vec<Result<EndVersion, Status>> {
        let mut tasks = vec![];
        let batches = self.get_batches().await;
        let db_writer = db_writer.clone();
        let backup_restore_operator = backup_restore_operator.clone();

        for batch in batches {
            let db_writer = db_writer.clone();
            let backup_restore_operator = backup_restore_operator.clone();
            let context = self.context.clone();
            let ledger_version = self.highest_known_version;
            let task = tokio::spawn(async move {
                let raw_txns =
                    Self::fetch_raw_txns_with_retries(context.clone(), ledger_version, batch).await;
                Self::parse_table_info(
                    context.clone(),
                    raw_txns.clone(),
                    db_writer.clone(),
                    backup_restore_operator.clone(),
                )
                .await
                .expect("Failed to parse table info");

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
        loop {
            match context.get_transactions(
                batch.start_version,
                batch.num_transactions_to_fetch,
                ledger_version,
            ) {
                Ok(raw_txns) => return raw_txns,
                Err(_err) => {
                    continue;
                },
            }
        }
    }

    /// Parses the table information from the raw transactions before converting to the api transactions,
    /// optionally backup the rocksdb to gcs depending on epoch advancement or not.
    async fn parse_table_info(
        context: Arc<Context>,
        raw_txns: Vec<TransactionOnChainData>,
        db_writer: Arc<dyn DbWriter>,
        backup_restore_operator: Arc<GcsBackupRestoreOperator>,
    ) -> Result<(), Error> {
        if raw_txns.is_empty() {
            return Ok(());
        }

        let start_millis = chrono::Utc::now().naive_utc();
        let first_version = raw_txns.first().map(|txn| txn.version).unwrap();
        let ledger_chain_id = context.chain_id().id();
        let (_, _, block_event) = context
            .db
            .get_block_info_by_version(first_version)
            .unwrap_or_else(|_| {
                panic!(
                    "Could not get block_info for start version {}",
                    first_version,
                )
            });
        let block_event_epoch = block_event.epoch();

        let write_sets: Vec<WriteSet> = raw_txns.iter().map(|txn| txn.changes.clone()).collect();
        let write_sets_slice: Vec<&WriteSet> = write_sets.iter().collect();
        db_writer
            .clone()
            .index(
                context.db.clone(),
                first_version,
                &write_sets_slice,
                block_event_epoch,
            )
            .expect("Failed to process write sets and index to the table info rocksdb");

        let metadata_epoch = backup_restore_operator.clone().get_metadata_epoch().await;
        if metadata_epoch < block_event_epoch {
            let checkpoint_path = context
                .node_config
                .storage
                .get_dir_paths()
                .default_root_path()
                .join(block_event_epoch.to_string());
            backup_restore_operator
                .try_upload_snapshot(
                    ledger_chain_id as u64,
                    block_event_epoch,
                    first_version,
                    db_writer.clone(),
                    checkpoint_path.clone(),
                )
                .await
                .expect("Failed to upload snapshot");
        }
        let fetch_millis = (chrono::Utc::now().naive_utc() - start_millis).num_milliseconds();

        info!(
            table_info_first_version = first_version,
            block_event_epoch = block_event_epoch,
            metadata_epoch = metadata_epoch,
            ledger_chain_id = ledger_chain_id,
            table_info_parsing_millis = fetch_millis,
            num_transactions = raw_txns.len(),
        );

        Ok(())
    }

    pub fn set_highest_known_version(&mut self) -> anyhow::Result<()> {
        let info = self.context.get_latest_ledger_info_wrapped()?;
        self.highest_known_version = info.ledger_version.0;
        Ok(())
    }

    async fn ensure_highest_known_version(&mut self) {
        while self.highest_known_version == 0 || self.current_version > self.highest_known_version {
            if let Err(err) = self.set_highest_known_version() {
                error!(
                    error = format!("{:?}", err),
                    "Failed to set highest known version"
                );
                continue;
            } else {
                sample!(
                    SampleRate::Frequency(10),
                    info!(
                        highest_known_version = self.highest_known_version,
                        "Found new highest known version",
                    )
                );
            }
        }
    }
}

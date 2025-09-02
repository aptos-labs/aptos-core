// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    backup_restore::gcs::GcsBackupRestoreOperator, snapshot_folder_name, snapshot_folder_prefix,
};
use anyhow::{anyhow, Context, Error};
use aptos_api::context::Context as ApiContext;
use aptos_api_types::TransactionOnChainData;
use aptos_db_indexer::db_v2::IndexerAsyncV2;
use aptos_indexer_grpc_fullnode::stream_coordinator::{
    IndexerStreamCoordinator, TransactionBatchInfo,
};
use aptos_indexer_grpc_utils::counters::{log_grpc_step, IndexerGrpcStep};
use aptos_logger::{debug, error, info, sample, sample::SampleRate};
use aptos_types::write_set::WriteSet;
use itertools::Itertools;
use std::{cmp::Ordering, sync::Arc, time::Duration};

type EndVersion = u64;
const LEDGER_VERSION_RETRY_TIME_MILLIS: u64 = 10;
const TABLE_INFO_SNAPSHOT_CHECK_INTERVAL_IN_SECS: u64 = 5;
const SERVICE_TYPE: &str = "table_info_service";

/// TableInfoService is responsible for parsing table info from transactions and writing them to rocksdb.
/// Not thread safe.
pub struct TableInfoService {
    pub current_version: u64,
    pub parser_task_count: u16,
    pub parser_batch_size: u16,
    pub context: Arc<ApiContext>,
    pub indexer_async_v2: Arc<IndexerAsyncV2>,

    // Backup and restore service. If not enabled, this will be None.
    pub backup_restore_operator: Option<Arc<GcsBackupRestoreOperator>>,
}

impl TableInfoService {
    pub fn new(
        context: Arc<ApiContext>,
        request_start_version: u64,
        parser_task_count: u16,
        parser_batch_size: u16,
        backup_restore_operator: Option<Arc<GcsBackupRestoreOperator>>,
        indexer_async_v2: Arc<IndexerAsyncV2>,
    ) -> Self {
        Self {
            current_version: request_start_version,
            parser_task_count,
            parser_batch_size,
            context,
            backup_restore_operator,
            indexer_async_v2,
        }
    }

    /// Start table info service and backup service is optional.
    /// It contains two main loops:
    /// 1. Table info parsing loop: fetching raw txns from db, processing, and writing to rocksdb.
    ///        If backup service is enabled, it will also snapshot the rocksdb at the end of each epoch.
    /// 2. Optional backup service loop: it monitors if new snapshots are available to backup and uploads them to GCS.
    pub async fn run(&mut self) {
        // TODO: fix the restore logic.
        let backup_is_enabled = match self.backup_restore_operator.clone() {
            Some(backup_restore_operator) => {
                let context = self.context.clone();
                let _task = tokio::spawn(async move {
                    loop {
                        aptos_logger::info!("[Table Info] Checking for snapshots to backup.");
                        Self::backup_snapshot_if_present(
                            context.clone(),
                            backup_restore_operator.clone(),
                        )
                        .await;
                        tokio::time::sleep(Duration::from_secs(
                            TABLE_INFO_SNAPSHOT_CHECK_INTERVAL_IN_SECS,
                        ))
                        .await;
                    }
                });
                true
            },
            None => false,
        };

        let mut current_epoch: Option<u64> = None;
        loop {
            let start_time = std::time::Instant::now();
            let ledger_version = self.get_highest_known_version().await.unwrap_or_default();
            let batches = self.get_batches(ledger_version).await;
            let transactions = self.fetch_batches(batches, ledger_version).await.unwrap();
            let num_transactions = transactions.len();
            let last_version = transactions
                .last()
                .map(|txn| txn.version)
                .unwrap_or_default();
            let (transactions_in_previous_epoch, transactions_in_current_epoch, epoch) =
                transactions_in_epochs(&self.context, current_epoch, transactions);

            // At the end of the epoch, snapshot the database.
            if !transactions_in_previous_epoch.is_empty() {
                self.process_transactions_in_parallel(
                    self.indexer_async_v2.clone(),
                    transactions_in_previous_epoch,
                )
                .await;
                let previous_epoch = epoch - 1;
                if backup_is_enabled {
                    aptos_logger::info!(
                        epoch = previous_epoch,
                        "[Table Info] Snapshot taken at the end of the epoch"
                    );
                    Self::snapshot_indexer_async_v2(
                        self.context.clone(),
                        self.indexer_async_v2.clone(),
                        previous_epoch,
                    )
                    .await
                    .expect("Failed to snapshot indexer async v2");
                }
            } else {
                // If there are no transactions in the previous epoch, it means we have caught up to the latest epoch.
                // We still need to figure out if we're at the start of the epoch or in the middle of the epoch.
                if let Some(current_epoch) = current_epoch {
                    if current_epoch != epoch {
                        // We're at the start of the epoch.
                        // We need to snapshot the database.
                        if backup_is_enabled {
                            aptos_logger::info!(
                                epoch = current_epoch,
                                "[Table Info] Snapshot taken at the start of the epoch"
                            );
                            Self::snapshot_indexer_async_v2(
                                self.context.clone(),
                                self.indexer_async_v2.clone(),
                                current_epoch,
                            )
                            .await
                            .expect("Failed to snapshot indexer async v2");
                        }
                    }
                }
            }

            self.process_transactions_in_parallel(
                self.indexer_async_v2.clone(),
                transactions_in_current_epoch,
            )
            .await;

            let versions_processed = num_transactions as i64;
            let start_version = self.current_version;
            log_grpc_step(
                SERVICE_TYPE,
                IndexerGrpcStep::TableInfoProcessed,
                Some(start_version as i64),
                Some(last_version as i64),
                None,
                None,
                Some(start_time.elapsed().as_secs_f64()),
                None,
                Some(versions_processed),
                None,
            );

            self.current_version = last_version + 1;
            current_epoch = Some(epoch);
        }
    }

    async fn fetch_batches(
        &self,
        batches: Vec<TransactionBatchInfo>,
        ledger_version: u64,
    ) -> anyhow::Result<Vec<TransactionOnChainData>> {
        // Spawn a bunch of threads to fetch transactions in parallel.
        let mut tasks = vec![];
        for batch in batches.iter().cloned() {
            let task = tokio::spawn(IndexerStreamCoordinator::fetch_raw_txns_with_retries(
                self.context.clone(),
                ledger_version,
                batch,
            ));
            tasks.push(task);
        }
        // Wait for all the threads to finish.
        let mut raw_txns = vec![];
        for task in tasks {
            raw_txns.push(task.await?);
        }
        // Flatten the results and sort them.
        let result: Vec<TransactionOnChainData> = raw_txns
            .into_iter()
            .flatten()
            .sorted_by_key(|txn| txn.version)
            .collect();

        // Verify that the transactions are sorted with no gap.
        if result.windows(2).any(|w| w[0].version + 1 != w[1].version) {
            // get all the versions

            let versions: Vec<u64> = result.iter().map(|txn| txn.version).collect();
            return Err(anyhow::anyhow!(format!(
                "Transactions are not sorted {:?}",
                versions
            )));
        }
        Ok(result)
    }

    /// Fans out a bunch of threads and processes write sets from transactions in parallel.
    /// Pushes results in parallel to the stream, but only return that the batch is
    /// fully completed if every job in the batch is successful and no pending on items
    /// Processing transactions in 2 stages:
    /// 1. Fetch transactions from ledger db
    /// 2. Get write sets from transactions and parse write sets to get handle -> key,value type mapping, write the mapping to the rocksdb
    async fn process_transactions_in_parallel(
        &self,
        indexer_async_v2: Arc<IndexerAsyncV2>,
        transactions: Vec<TransactionOnChainData>,
    ) -> Vec<EndVersion> {
        let mut tasks = vec![];
        let context = self.context.clone();
        let last_version = transactions
            .last()
            .map(|txn| txn.version)
            .unwrap_or_default();

        // We copy the transactions here in case we need to retry the parsing.
        let batches: Vec<Vec<TransactionOnChainData>> = transactions
            .chunks(self.parser_batch_size as usize)
            .map(|chunk| chunk.to_vec())
            .collect();

        for batch in batches {
            let task = tokio::spawn(Self::process_transactions(
                context.clone(),
                indexer_async_v2.clone(),
                batch,
            ));
            tasks.push(task);
        }

        match futures::future::try_join_all(tasks).await {
            Ok(res) => {
                let end_version = last_version;

                // If pending on items are not empty, meaning the current loop hasn't fully parsed all table infos
                // due to the nature of multithreading where instructions used to parse table info might come later,
                // retry sequentially to ensure parsing is complete
                //
                // Risk of this sequential approach is that it could be slow when the txns to process contain extremely
                // nested table items, but the risk is bounded by the configuration of the number of txns to process and number of threads
                if !self.indexer_async_v2.is_indexer_async_v2_pending_on_empty() {
                    self.indexer_async_v2.clear_pending_on();
                    Self::process_transactions(
                        context.clone(),
                        indexer_async_v2.clone(),
                        transactions,
                    )
                    .await;
                }

                assert!(
                    self.indexer_async_v2.is_indexer_async_v2_pending_on_empty(),
                    "Missing data in table info parsing after sequential retry"
                );

                // Update rocksdb's to be processed next version after verifying all txns are successfully parsed
                self.indexer_async_v2
                    .update_next_version(end_version + 1)
                    .unwrap();

                res
            },
            Err(err) => panic!(
                "[Table Info] Error processing table info batches: {:?}",
                err
            ),
        }
    }

    /// Process a single batch of transactions for table info parsing.
    /// It's used in the first loop to process batches in parallel,
    /// and it's used in the second loop to process transactions sequentially
    /// if pending on items are not empty
    async fn process_transactions(
        context: Arc<ApiContext>,
        indexer_async_v2: Arc<IndexerAsyncV2>,
        raw_txns: Vec<TransactionOnChainData>,
    ) -> EndVersion {
        let start_time = std::time::Instant::now();
        let start_version = raw_txns[0].version;
        let end_version = raw_txns.last().unwrap().version;
        let num_transactions = raw_txns.len();

        loop {
            // NOTE: The retry is unlikely to be helpful. Put a loop here just to avoid panic and
            // allow the rest of FN functionality continue to work.
            match Self::parse_table_info(
                context.clone(),
                raw_txns.clone(),
                indexer_async_v2.clone(),
            ) {
                Ok(_) => break,
                Err(e) => {
                    error!(error = ?e, "Error during parse_table_info.");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                },
            }
        }

        log_grpc_step(
            SERVICE_TYPE,
            IndexerGrpcStep::TableInfoProcessedBatch,
            Some(start_version as i64),
            Some(end_version as i64),
            None,
            None,
            Some(start_time.elapsed().as_secs_f64()),
            None,
            Some(num_transactions as i64),
            None,
        );

        raw_txns.last().unwrap().version
    }

    /// Retrieves transaction batches based on the provided ledger version.
    /// The function prepares to fetch transactions by determining the start version,
    /// the number of fetches, and the size of each batch.
    async fn get_batches(&mut self, ledger_version: u64) -> Vec<TransactionBatchInfo> {
        info!(
            current_version = self.current_version,
            highest_known_version = ledger_version,
            parser_batch_size = self.parser_batch_size,
            parser_task_count = self.parser_task_count,
            "[Table Info] Preparing to fetch transactions"
        );

        let mut start_version = self.current_version;
        let mut num_fetches = 0;
        let mut batches = vec![];

        while num_fetches < self.parser_task_count && start_version <= ledger_version {
            let num_transactions_to_fetch = std::cmp::min(
                self.parser_batch_size as u64,
                ledger_version + 1 - start_version,
            ) as u16;

            batches.push(TransactionBatchInfo {
                start_version,
                num_transactions_to_fetch,
                head_version: ledger_version,
            });

            start_version += num_transactions_to_fetch as u64;
            num_fetches += 1;
        }

        batches
    }

    /// Parse table info from write sets,
    fn parse_table_info(
        context: Arc<ApiContext>,
        raw_txns: Vec<TransactionOnChainData>,
        indexer_async_v2: Arc<IndexerAsyncV2>,
    ) -> Result<(), Error> {
        if raw_txns.is_empty() {
            return Ok(());
        }

        let start_time = std::time::Instant::now();
        let first_version = raw_txns.first().map(|txn| txn.version).unwrap();
        let write_sets: Vec<WriteSet> = raw_txns.iter().map(|txn| txn.changes.clone()).collect();
        let write_sets_slice: Vec<&WriteSet> = write_sets.iter().collect();
        indexer_async_v2
            .index_table_info(context.db.clone(), first_version, &write_sets_slice)
            .map_err(|err| anyhow!("[Table Info] Failed to process write sets and index to the table info rocksdb: {}", err))?;

        info!(
            table_info_first_version = first_version,
            table_info_parsing_millis_per_batch = start_time.elapsed().as_millis(),
            num_transactions = raw_txns.len(),
            "[Table Info] Table info parsed successfully"
        );

        Ok(())
    }

    async fn snapshot_indexer_async_v2(
        context: Arc<ApiContext>,
        indexer_async_v2: Arc<IndexerAsyncV2>,
        epoch: u64,
    ) -> anyhow::Result<()> {
        let chain_id = context.chain_id().id();
        // temporary path to store the snapshot
        let snapshot_dir = context
            .node_config
            .get_data_dir()
            .join(snapshot_folder_name(chain_id as u64, epoch));
        // rocksdb will create a checkpoint to take a snapshot of full db and then save it to snapshot_path
        indexer_async_v2
            .create_checkpoint(&snapshot_dir)
            .context(format!("DB checkpoint failed at epoch {}", epoch))?;

        Ok(())
    }

    /// Uploads the snapshot to GCS if found.
    /// 1. If current epoch is backuped, it will skip the backup.
    /// 2. If the chain id in the backup metadata does not match with the current network, it will panic.
    /// Not thread safe.
    /// TODO(larry): improve the error handling.
    async fn backup_snapshot_if_present(
        context: Arc<ApiContext>,
        backup_restore_operator: Arc<GcsBackupRestoreOperator>,
    ) {
        let target_snapshot_directory_prefix =
            snapshot_folder_prefix(context.chain_id().id() as u64);
        // Scan the data directory to find the latest epoch to upload.
        let mut epochs_to_backup = vec![];
        for entry in std::fs::read_dir(context.node_config.get_data_dir()).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let file_name = path.file_name().unwrap().to_string_lossy();
            if path.is_dir()
                && file_name.starts_with(&target_snapshot_directory_prefix)
                && !file_name.ends_with(".tmp")
            {
                let epoch = file_name.replace(&target_snapshot_directory_prefix, "");
                let epoch = epoch.parse::<u64>().unwrap();
                epochs_to_backup.push(epoch);
            }
        }
        // If nothing to backup, return.
        if epochs_to_backup.is_empty() {
            // No snapshot to backup.
            aptos_logger::info!("[Table Info] No snapshot to backup. Skipping the backup.");
            return;
        }
        aptos_logger::info!(
            epochs_to_backup = format!("{:?}", epochs_to_backup),
            "[Table Info] Found snapshots to backup."
        );
        // Sort the epochs to backup.
        epochs_to_backup.sort();
        aptos_logger::info!(
            epochs_to_backup = format!("{:?}", epochs_to_backup),
            "[Table Info] Sorted snapshots to backup."
        );
        // Backup the existing snapshots and cleanup.
        for epoch in epochs_to_backup {
            backup_the_snapshot_and_cleanup(
                context.clone(),
                backup_restore_operator.clone(),
                epoch,
            )
            .await;
        }
    }

    /// TODO(jill): consolidate it with `ensure_highest_known_version`
    /// Will keep looping and checking the latest ledger info to see if there are new transactions
    /// If there are, it will update the ledger version version
    async fn get_highest_known_version(&self) -> Result<u64, Error> {
        let mut info = self.context.get_latest_ledger_info_wrapped();
        let mut ledger_version = info.unwrap().ledger_version.0;
        let mut empty_loops = 0;

        while ledger_version == 0 || self.current_version > ledger_version {
            if empty_loops > 0 {
                tokio::time::sleep(Duration::from_millis(LEDGER_VERSION_RETRY_TIME_MILLIS)).await;
            }
            empty_loops += 1;
            if let Err(err) = {
                info = self.context.get_latest_ledger_info_wrapped();
                ledger_version = info.unwrap().ledger_version.0;
                Ok::<(), Error>(())
            } {
                error!(
                    error = format!("{:?}", err),
                    "[Table Info] Failed to set highest known version"
                );
                continue;
            } else {
                sample!(
                    SampleRate::Frequency(100),
                    debug!(
                        ledger_version = ledger_version,
                        "[Table Info] Found new highest known ledger version",
                    )
                );
            }
        }
        Ok(ledger_version)
    }
}

async fn backup_the_snapshot_and_cleanup(
    context: Arc<ApiContext>,
    backup_restore_operator: Arc<GcsBackupRestoreOperator>,
    epoch: u64,
) {
    let snapshot_folder_name = snapshot_folder_name(context.chain_id().id() as u64, epoch);
    aptos_logger::info!(
        epoch = epoch,
        snapshot_folder_name = snapshot_folder_name,
        "[Table Info] Backing up the snapshot and cleaning up the old snapshot."
    );
    let ledger_chain_id = context.chain_id().id();
    // Validate the runtime.
    let backup_metadata = backup_restore_operator.get_metadata().await;
    if let Some(metadata) = backup_metadata {
        if metadata.chain_id != (ledger_chain_id as u64) {
            panic!(
                "Table Info backup chain id does not match with current network. Expected: {}, found in backup: {}",
                context.chain_id().id(),
                metadata.chain_id
            );
        }
    } else {
        aptos_logger::warn!(
            epoch = epoch,
            snapshot_folder_name = snapshot_folder_name,
            "[Table Info] No backup metadata found. Skipping the backup."
        );
    }

    let start_time = std::time::Instant::now();
    // temporary path to store the snapshot
    let snapshot_dir = context
        .node_config
        .get_data_dir()
        .join(snapshot_folder_name.clone());
    // If the backup is for old epoch, clean up and return.
    if let Some(metadata) = backup_metadata {
        aptos_logger::info!(
            epoch = epoch,
            metadata_epoch = metadata.epoch,
            snapshot_folder_name = snapshot_folder_name,
            snapshot_dir = snapshot_dir.to_str(),
            "[Table Info] Checking the metadata before backup."
        );
        if metadata.epoch >= epoch {
            aptos_logger::info!(
                epoch = epoch,
                snapshot_folder_name = snapshot_folder_name,
                "[Table Info] Snapshot already backed up. Skipping the backup."
            );
            // Remove the snapshot directory.
            std::fs::remove_dir_all(snapshot_dir).unwrap();
            return;
        }
    } else {
        aptos_logger::warn!(
            epoch = epoch,
            snapshot_folder_name = snapshot_folder_name,
            "[Table Info] No backup metadata found."
        );
    }
    aptos_logger::info!(
        epoch = epoch,
        snapshot_folder_name = snapshot_folder_name,
        snapshot_dir = snapshot_dir.to_str(),
        "[Table Info] Backing up the snapshot."
    );
    // TODO: add checks to handle concurrent backup jobs.
    backup_restore_operator
        .backup_db_snapshot_and_update_metadata(ledger_chain_id as u64, epoch, snapshot_dir.clone())
        .await
        .expect("Failed to upload snapshot in table info service");

    // TODO: use log_grpc_step to log the backup step.
    info!(
        backup_epoch = epoch,
        backup_millis = start_time.elapsed().as_millis(),
        "[Table Info] Table info db backed up successfully"
    );
}

/// Split transactions into two epochs based on the first version in **this** epoch.
/// If the first version of the transaction is less than the epoch first version, it will be in the previous epoch.
/// Otherwise, it will be in the current epoch.
fn transactions_in_epochs(
    context: &ApiContext,
    current_epoch: Option<u64>,
    mut transactions: Vec<TransactionOnChainData>,
) -> (
    Vec<TransactionOnChainData>,
    Vec<TransactionOnChainData>,
    u64,
) {
    let last_version = transactions
        .last()
        .map(|txn| txn.version)
        .unwrap_or_default();
    let first_version = transactions
        .first()
        .map(|txn| txn.version)
        .unwrap_or_default();
    // Get epoch information.
    let (epoch_first_version, _, block_epoch) = context
        .db
        .get_block_info_by_version(last_version)
        .unwrap_or_else(|_| panic!("Could not get block_info for last version {}", last_version));

    if current_epoch.is_none() {
        // Current epoch is not tracked yet, assume that all transactions are in the current epoch.
        return (vec![], transactions, block_epoch.epoch());
    }
    let current_epoch = current_epoch.unwrap();

    let split_off_index = match current_epoch.cmp(&block_epoch.epoch()) {
        Ordering::Equal => {
            // All transactions are in the this epoch.
            // Previous epoch is empty, i.e., [0, 0), and this epoch is [first_version, last_version].
            0
        },
        Ordering::Less => {
            // Try the best to split the transactions into two epochs.
            epoch_first_version - first_version
        },
        _ => unreachable!("Epochs are not sorted."),
    };

    // Log the split of the transactions.
    aptos_logger::info!(
        split_off_index = split_off_index,
        last_version = last_version,
        first_version = first_version,
        epoch_first_version = epoch_first_version,
        block_epoch = block_epoch.epoch(),
        current_epoch = current_epoch,
        "[Table Info] Split transactions into two epochs."
    );

    let transactions_in_this_epoch = transactions.split_off(split_off_index as usize);
    // The rest of the transactions are in the previous epoch.
    let transactions_in_previous_epoch = transactions;
    (
        transactions_in_previous_epoch,
        transactions_in_this_epoch,
        block_epoch.epoch(),
    )
}

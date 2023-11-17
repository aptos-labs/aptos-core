// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    convert::convert_transaction,
    counters::{FETCHED_LATENCY_IN_SECS, FETCHED_TRANSACTION, UNABLE_TO_FETCH_TRANSACTION},
    runtime::{DEFAULT_NUM_RETRIES, RETRY_TIME_MILLIS},
};
use aptos_api::context::Context;
use aptos_api_types::{AsConverter, Transaction as APITransaction, TransactionOnChainData};
use aptos_indexer_grpc_utils::{chunk_transactions, constants::MESSAGE_SIZE_LIMIT};
use aptos_logger::{error, info, sample, sample::SampleRate};
use aptos_protos::{
    internal::fullnode::v1::{
        transactions_from_node_response, TransactionsFromNodeResponse, TransactionsOutput,
    },
    transaction::v1::Transaction as TransactionPB,
};
use aptos_vm::data_cache::AsMoveResolver;
use std::{
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::mpsc;
use tonic::Status;

type EndVersion = u64;

// Basically a handler for a single GRPC stream request
pub struct IndexerStreamCoordinator {
    pub current_version: u64,
    pub processor_task_count: u16,
    pub processor_batch_size: u16,
    pub output_batch_size: u16,
    pub highest_known_version: u64,
    pub context: Arc<Context>,
    pub transactions_sender: mpsc::Sender<Result<TransactionsFromNodeResponse, tonic::Status>>,
}

// Single batch of transactions to fetch, convert, and stream
pub struct TransactionBatchInfo {
    pub start_version: u64,
    pub num_transactions_to_fetch: u16,
}

impl IndexerStreamCoordinator {
    /// Coordinates the fetching, processing, and streaming of transactions
    pub fn new(
        context: Arc<Context>,
        request_start_version: u64,
        processor_task_count: u16,
        processor_batch_size: u16,
        output_batch_size: u16,
        transactions_sender: mpsc::Sender<Result<TransactionsFromNodeResponse, tonic::Status>>,
    ) -> Self {
        Self {
            current_version: request_start_version,
            processor_task_count,
            processor_batch_size,
            output_batch_size,
            highest_known_version: 0,
            context,
            transactions_sender,
        }
    }

    /// Fans out a bunch of threads and processes transactions in parallel.
    /// Pushes results in parallel to the stream, but only return that the batch is
    /// fully completed if every job in the batch is successful
    /// Processing transactions in 4 stages:
    /// 1. Fetch transactions from storage
    /// 2. Convert transactions to rust objects (for example stringifying move structs into json)
    /// 3. Convert into protobuf objects
    /// 4. Encode protobuf objects (base64)
    pub async fn process_next_batch(&mut self) -> Vec<Result<EndVersion, Status>> {
        let ledger_chain_id = self.context.chain_id().id();
        let mut tasks = vec![];
        let batches = self.get_batches().await;
        let output_batch_size = self.output_batch_size;

        for batch in batches {
            let context = self.context.clone();
            let ledger_version = self.highest_known_version;
            let transaction_sender = self.transactions_sender.clone();

            let task = tokio::spawn(async move {
                // Fetch and convert transactions from API
                let raw_txns =
                    Self::fetch_raw_txns_with_retries(context.clone(), ledger_version, batch).await;
                let api_txns = Self::convert_to_api_txns(context, raw_txns).await;
                api_txns.last().map(record_fetched_transaction_latency);
                let pb_txns = Self::convert_to_pb_txns(api_txns);
                // Wrap in stream response object and send to channel
                for chunk in pb_txns.chunks(output_batch_size as usize) {
                    for chunk in chunk_transactions(chunk.to_vec(), MESSAGE_SIZE_LIMIT) {
                        let item = TransactionsFromNodeResponse {
                            response: Some(transactions_from_node_response::Response::Data(
                                TransactionsOutput {
                                    transactions: chunk,
                                },
                            )),
                            chain_id: ledger_chain_id as u32,
                        };
                        match transaction_sender.send(Result::<_, Status>::Ok(item)).await {
                            Ok(_) => {},
                            Err(_) => {
                                // Client disconnects.
                                return Err(Status::aborted("Client disconnected"));
                            },
                        }
                    }
                }
                Ok(pb_txns.last().unwrap().version)
            });
            tasks.push(task);
        }
        match futures::future::try_join_all(tasks).await {
            Ok(res) => res,
            Err(err) => panic!("Error processing transaction batches: {:?}", err),
        }
    }

    /// Gets the last version of the batch if the entire batch is successful, otherwise return error
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

    /// This will create batches based on the configuration of the request
    async fn get_batches(&mut self) -> Vec<TransactionBatchInfo> {
        self.ensure_highest_known_version().await;

        info!(
            current_version = self.current_version,
            highest_known_version = self.highest_known_version,
            processor_batch_size = self.processor_batch_size,
            processor_task_count = self.processor_task_count,
            "Preparing to fetch transactions"
        );

        let mut starting_version = self.current_version;
        let mut num_fetches = 0;
        let mut batches = vec![];

        while num_fetches < self.processor_task_count
            && starting_version <= self.highest_known_version
        {
            let num_transactions_to_fetch = std::cmp::min(
                self.processor_batch_size as u64,
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
                    UNABLE_TO_FETCH_TRANSACTION.inc();
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

    async fn convert_to_api_txns(
        context: Arc<Context>,
        raw_txns: Vec<TransactionOnChainData>,
    ) -> Vec<APITransaction> {
        if raw_txns.is_empty() {
            return vec![];
        }
        let start_millis = chrono::Utc::now().naive_utc();

        let first_version = raw_txns.first().map(|txn| txn.version).unwrap();
        let state_view = context.latest_state_view().unwrap();
        let resolver = state_view.as_move_resolver();
        let converter = resolver.as_converter(context.db.clone());

        // Enrich data with block metadata
        let (_, _, block_event) = context
            .db
            .get_block_info_by_version(first_version)
            .unwrap_or_else(|_| {
                panic!(
                    "Could not get block_info for start version {}",
                    first_version,
                )
            });
        let mut timestamp = block_event.proposed_time();
        let mut epoch = block_event.epoch();
        let mut epoch_bcs = aptos_api_types::U64::from(epoch);
        let mut block_height = block_event.height();
        let mut block_height_bcs = aptos_api_types::U64::from(block_height);

        let mut transactions = vec![];
        for (ind, raw_txn) in raw_txns.into_iter().enumerate() {
            let txn_version = raw_txn.version;
            // Do not update block_height if first block is block metadata
            if ind > 0 {
                // Update the timestamp if the next block occurs
                if let Some(txn) = raw_txn.transaction.try_as_block_metadata() {
                    timestamp = txn.timestamp_usecs();
                    epoch = txn.epoch();
                    epoch_bcs = aptos_api_types::U64::from(epoch);
                    block_height += 1;
                    block_height_bcs = aptos_api_types::U64::from(block_height);
                }
            }
            match converter
                .try_into_onchain_transaction(timestamp, raw_txn)
                .map(|mut txn| {
                    match txn {
                        APITransaction::PendingTransaction(_) => {
                            unreachable!("Indexer should never see pending transactions")
                        },
                        APITransaction::UserTransaction(ref mut ut) => {
                            ut.info.block_height = Some(block_height_bcs);
                            ut.info.epoch = Some(epoch_bcs);
                        },
                        APITransaction::GenesisTransaction(ref mut gt) => {
                            gt.info.block_height = Some(block_height_bcs);
                            gt.info.epoch = Some(epoch_bcs);
                        },
                        APITransaction::BlockMetadataTransaction(ref mut bmt) => {
                            bmt.info.block_height = Some(block_height_bcs);
                            bmt.info.epoch = Some(epoch_bcs);
                        },
                        APITransaction::StateCheckpointTransaction(ref mut sct) => {
                            sct.info.block_height = Some(block_height_bcs);
                            sct.info.epoch = Some(epoch_bcs);
                        },
                    };
                    txn
                }) {
                Ok(transaction) => transactions.push(transaction),
                Err(err) => {
                    UNABLE_TO_FETCH_TRANSACTION.inc();
                    error!(
                        version = txn_version,
                        error = format!("{:?}", err),
                        "Could not convert from OnChainTransactions",
                    );
                    // IN CASE WE NEED TO SKIP BAD TXNS
                    // continue;
                    panic!(
                        "Could not convert txn {} from OnChainTransactions: {:?}",
                        txn_version, err
                    );
                },
            }
        }

        if transactions.is_empty() {
            panic!("No transactions!");
        }

        let fetch_millis = (chrono::Utc::now().naive_utc() - start_millis).num_milliseconds();

        info!(
            first_version = first_version,
            num_transactions = transactions.len(),
            time_millis = fetch_millis,
            actual_last_version = transactions
                .last()
                .map(|txn| txn.version().unwrap())
                .unwrap_or(0),
            "Fetched transactions",
        );

        FETCHED_TRANSACTION.inc();
        transactions
    }

    fn convert_to_pb_txns(api_txns: Vec<APITransaction>) -> Vec<TransactionPB> {
        api_txns
            .iter()
            .map(|txn| {
                let info = txn.transaction_info().unwrap();
                convert_transaction(txn, info.block_height.unwrap().0, info.epoch.unwrap().0)
            })
            .collect()
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
                tokio::time::sleep(Duration::from_millis(RETRY_TIME_MILLIS)).await;
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

/// Record the transaction fetched from the storage latency.
fn record_fetched_transaction_latency(txn: &aptos_api_types::Transaction) {
    let current_time_in_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Current time is before UNIX_EPOCH")
        .as_secs_f64();
    let txn_timestamp = txn.timestamp();

    if txn_timestamp > 0 {
        let txn_timestemp_in_secs = txn_timestamp as f64 / 1_000_000.0;
        FETCHED_LATENCY_IN_SECS.set(current_time_in_secs - txn_timestemp_in_secs);
    }
}

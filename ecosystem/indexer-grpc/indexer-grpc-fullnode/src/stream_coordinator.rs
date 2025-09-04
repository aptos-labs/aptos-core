// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    convert::convert_transaction,
    counters::UNABLE_TO_FETCH_TRANSACTION,
    runtime::{DEFAULT_NUM_RETRIES, RETRY_TIME_MILLIS},
};
use velor_api::context::Context;
use velor_api_types::{AsConverter, Transaction as APITransaction, TransactionOnChainData};
use velor_indexer_grpc_utils::{
    chunk_transactions,
    constants::MESSAGE_SIZE_LIMIT,
    counters::{log_grpc_step_fullnode, IndexerGrpcStep},
};
use velor_logger::{error, info, sample, sample::SampleRate};
use velor_protos::{
    internal::fullnode::v1::{
        transactions_from_node_response, TransactionsFromNodeResponse, TransactionsOutput,
    },
    transaction::v1::{
        EventSizeInfo, Transaction as TransactionPB, TransactionSizeInfo, WriteOpSizeInfo,
    },
    util::timestamp::Timestamp,
};
use itertools::Itertools;
use serde::Serialize;
use std::{sync::Arc, time::Duration};
use tokio::sync::mpsc;
use tonic::Status;

type EndVersion = u64;

const SERVICE_TYPE: &str = "indexer_fullnode";
const MINIMUM_TASK_LOAD_SIZE_IN_BYTES: usize = 100_000;

// Basically a handler for a single GRPC stream request
pub struct IndexerStreamCoordinator {
    pub current_version: u64,
    pub end_version: u64,
    pub processor_task_count: u16,
    pub processor_batch_size: u16,
    pub output_batch_size: u16,
    pub highest_known_version: u64,
    pub context: Arc<Context>,
    pub transactions_sender: mpsc::Sender<Result<TransactionsFromNodeResponse, tonic::Status>>,
}

// Single batch of transactions to fetch, convert, and stream
#[derive(Clone, Copy)]
pub struct TransactionBatchInfo {
    pub start_version: u64,
    pub head_version: u64,
    pub num_transactions_to_fetch: u16,
}

impl IndexerStreamCoordinator {
    /// Coordinates the fetching, processing, and streaming of transactions
    pub fn new(
        context: Arc<Context>,
        request_start_version: u64,
        end_version: u64,
        processor_task_count: u16,
        processor_batch_size: u16,
        output_batch_size: u16,
        transactions_sender: mpsc::Sender<Result<TransactionsFromNodeResponse, tonic::Status>>,
    ) -> Self {
        Self {
            current_version: request_start_version,
            end_version,
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
        let fetching_start_time = std::time::Instant::now();
        // Stage 1: fetch transactions from storage.
        let sorted_transactions_from_storage_with_size =
            self.fetch_transactions_from_storage().await;
        let first_version = sorted_transactions_from_storage_with_size
            .first()
            .map(|(txn, _)| txn.version)
            .unwrap() as i64;
        let end_version = sorted_transactions_from_storage_with_size
            .last()
            .map(|(txn, _)| txn.version)
            .unwrap() as i64;
        let num_transactions = sorted_transactions_from_storage_with_size.len();
        let highest_known_version = self.highest_known_version as i64;
        let (_, _, block_event) = self
            .context
            .db
            .get_block_info_by_version(end_version as u64)
            .unwrap_or_else(|_| {
                panic!(
                    "[Indexer Fullnode] Could not get block_info for version {}",
                    end_version,
                )
            });
        let last_transaction_timestamp_in_microseconds = block_event.proposed_time();
        let last_transaction_timestamp = Some(Timestamp {
            seconds: (last_transaction_timestamp_in_microseconds / 1_000_000) as i64,
            nanos: ((last_transaction_timestamp_in_microseconds % 1_000_000) * 1000) as i32,
        });

        log_grpc_step_fullnode(
            IndexerGrpcStep::FullnodeFetchedBatch,
            Some(first_version),
            Some(end_version),
            last_transaction_timestamp.as_ref(),
            Some(highest_known_version),
            None,
            Some(fetching_start_time.elapsed().as_secs_f64()),
            Some(num_transactions as i64),
        );
        // Stage 2: convert transactions to rust objects. CPU-bound load.
        let decoding_start_time = std::time::Instant::now();
        let mut task_batches = vec![];
        let mut current_batch = vec![];
        let mut current_batch_size = 0;
        for (txn, size) in sorted_transactions_from_storage_with_size {
            current_batch.push(txn);
            current_batch_size += size;
            if current_batch_size > MINIMUM_TASK_LOAD_SIZE_IN_BYTES {
                task_batches.push(current_batch);
                current_batch = vec![];
                current_batch_size = 0;
            }
        }
        if !current_batch.is_empty() {
            task_batches.push(current_batch);
        }

        let output_batch_size = self.output_batch_size;
        let ledger_chain_id = self.context.chain_id().id();
        let mut tasks = vec![];
        for batch in task_batches {
            let context = self.context.clone();
            let task = tokio::task::spawn_blocking(move || {
                let raw_txns = batch;
                let api_txns = Self::convert_to_api_txns(context, raw_txns);
                let pb_txns = Self::convert_to_pb_txns(api_txns);
                let mut responses = vec![];
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
                        responses.push(item);
                    }
                }
                responses
            });
            tasks.push(task);
        }
        let responses = match futures::future::try_join_all(tasks).await {
            Ok(res) => res.into_iter().flatten().collect::<Vec<_>>(),
            Err(err) => panic!(
                "[Indexer Fullnode] Error processing transaction batches: {:?}",
                err
            ),
        };
        log_grpc_step_fullnode(
            IndexerGrpcStep::FullnodeDecodedBatch,
            Some(first_version),
            Some(end_version),
            last_transaction_timestamp.as_ref(),
            Some(highest_known_version),
            None,
            Some(decoding_start_time.elapsed().as_secs_f64()),
            Some(num_transactions as i64),
        );
        // Stage 3: send responses to stream
        let sending_start_time = std::time::Instant::now();
        for response in responses {
            if self.transactions_sender.send(Ok(response)).await.is_err() {
                // Error from closed channel. This means the client has disconnected.
                return vec![];
            }
        }
        log_grpc_step_fullnode(
            IndexerGrpcStep::FullnodeSentBatch,
            Some(first_version),
            Some(end_version),
            last_transaction_timestamp.as_ref(),
            Some(highest_known_version),
            None,
            Some(sending_start_time.elapsed().as_secs_f64()),
            Some(num_transactions as i64),
        );
        vec![Ok(end_version as u64)]
    }

    /// Fetches transactions from storage with each transaction's size.
    /// Results are transactions sorted by version.
    async fn fetch_transactions_from_storage(&mut self) -> Vec<(TransactionOnChainData, usize)> {
        let batches = self.get_batches().await;
        let mut storage_fetch_tasks = vec![];
        let ledger_version = self.highest_known_version;
        for batch in batches {
            let context = self.context.clone();
            let task = tokio::spawn(async move {
                Self::fetch_raw_txns_with_retries(context.clone(), ledger_version, batch).await
            });
            storage_fetch_tasks.push(task);
        }

        let transactions_from_storage =
            match futures::future::try_join_all(storage_fetch_tasks).await {
                Ok(res) => res,
                Err(err) => panic!(
                    "[Indexer Fullnode] Error fetching transaction batches: {:?}",
                    err
                ),
            };

        transactions_from_storage
            .into_iter()
            .flatten()
            .sorted_by(|a, b| a.version.cmp(&b.version))
            .map(|txn| {
                let size = bcs::serialized_size(&txn).expect("Unable to serialize txn");
                (txn, size)
            })
            .collect::<Vec<_>>()
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

        let mut starting_version = self.current_version;
        let mut num_fetches = 0;
        let mut batches = vec![];
        let end_version = std::cmp::min(self.end_version, self.highest_known_version + 1);

        while num_fetches < self.processor_task_count && starting_version < end_version {
            let num_transactions_to_fetch = std::cmp::min(
                self.processor_batch_size as u64,
                end_version - starting_version,
            ) as u16;

            batches.push(TransactionBatchInfo {
                start_version: starting_version,
                head_version: self.highest_known_version,
                num_transactions_to_fetch,
            });
            starting_version += num_transactions_to_fetch as u64;
            num_fetches += 1;
        }
        batches
    }

    pub async fn fetch_raw_txns_with_retries(
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

    fn convert_to_api_txns(
        context: Arc<Context>,
        raw_txns: Vec<TransactionOnChainData>,
    ) -> Vec<(APITransaction, TransactionSizeInfo)> {
        if raw_txns.is_empty() {
            return vec![];
        }
        let start_millis = chrono::Utc::now().naive_utc();

        let first_version = raw_txns.first().map(|txn| txn.version).unwrap();
        let state_view = context.latest_state_view().unwrap();
        let converter = state_view.as_converter(context.db.clone(), context.indexer_reader.clone());

        // Enrich data with block metadata
        let (_, _, block_event) = context
            .db
            .get_block_info_by_version(first_version)
            .unwrap_or_else(|_| {
                panic!(
                    "[Indexer Fullnode] Could not get block_info for start version {}",
                    first_version,
                )
            });
        let mut timestamp = block_event.proposed_time();
        let mut epoch = block_event.epoch();
        let mut epoch_bcs = velor_api_types::U64::from(epoch);
        let mut block_height = block_event.height();
        let mut block_height_bcs = velor_api_types::U64::from(block_height);

        let mut transactions = vec![];
        for (ind, raw_txn) in raw_txns.into_iter().enumerate() {
            let txn_version = raw_txn.version;
            // Do not update block_height if first block is block metadata
            if ind > 0 {
                // Update the timestamp if the next block occurs
                if let Some(txn) = raw_txn.transaction.try_as_block_metadata_ext() {
                    timestamp = txn.timestamp_usecs();
                    epoch = txn.epoch();
                    epoch_bcs = velor_api_types::U64::from(epoch);
                    block_height += 1;
                    block_height_bcs = velor_api_types::U64::from(block_height);
                } else if let Some(txn) = raw_txn.transaction.try_as_block_metadata() {
                    timestamp = txn.timestamp_usecs();
                    epoch = txn.epoch();
                    epoch_bcs = velor_api_types::U64::from(epoch);
                    block_height += 1;
                    block_height_bcs = velor_api_types::U64::from(block_height);
                }
            }
            let size_info = Self::get_size_info(&raw_txn);
            let res = converter
                .try_into_onchain_transaction(timestamp, raw_txn)
                .map(|mut txn| {
                    match txn {
                        APITransaction::PendingTransaction(_) => {
                            unreachable!(
                                "[Indexer Fullnode] Indexer should never see pending transactions"
                            )
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
                        APITransaction::BlockEpilogueTransaction(ref mut bet) => {
                            bet.info.block_height = Some(block_height_bcs);
                            bet.info.epoch = Some(epoch_bcs);
                        },
                        APITransaction::ValidatorTransaction(ref mut vt) => {
                            let info = vt.transaction_info_mut();
                            info.block_height = Some(block_height_bcs);
                            info.epoch = Some(epoch_bcs);
                        },
                    };
                    txn
                });
            match res {
                Ok(transaction) => transactions.push((transaction, size_info)),
                Err(err) => {
                    UNABLE_TO_FETCH_TRANSACTION.inc();
                    error!(
                        version = txn_version,
                        error = format!("{:?}", err),
                        "[Indexer Fullnode] Could not convert from OnChainTransactions",
                    );
                    // IN CASE WE NEED TO SKIP BAD TXNS
                    // continue;
                    panic!(
                        "[Indexer Fullnode] Could not convert txn {} from OnChainTransactions: {:?}",
                        txn_version, err
                    );
                },
            }
        }

        if transactions.is_empty() {
            panic!("[Indexer Fullnode] No transactions!");
        }

        let fetch_millis = (chrono::Utc::now().naive_utc() - start_millis).num_milliseconds();

        info!(
            start_version = first_version,
            end_version = transactions
                .last()
                .map(|(txn, _size_info)| txn.version().unwrap())
                .unwrap_or(0),
            num_of_transactions = transactions.len(),
            fetch_duration_in_ms = fetch_millis,
            service_type = SERVICE_TYPE,
            "[Indexer Fullnode] Successfully converted transactions",
        );

        transactions
    }

    fn ser_size_u32<T: Serialize>(t: &T) -> u32 {
        bcs::serialized_size(t).expect("serialized_size() failed") as u32
    }

    fn get_size_info(raw_txn: &TransactionOnChainData) -> TransactionSizeInfo {
        TransactionSizeInfo {
            transaction_bytes: Self::ser_size_u32(&raw_txn.transaction),
            event_size_info: raw_txn
                .events
                .iter()
                .map(|event| EventSizeInfo {
                    type_tag_bytes: Self::ser_size_u32(event.type_tag()),
                    total_bytes: event.size() as u32,
                })
                .collect(),
            write_op_size_info: raw_txn
                .changes
                .write_op_iter()
                .map(|(state_key, write_op)| WriteOpSizeInfo {
                    key_bytes: Self::ser_size_u32(state_key),
                    value_bytes: write_op.bytes_size() as u32,
                })
                .collect(),
        }
    }

    fn convert_to_pb_txns(
        api_txns: Vec<(APITransaction, TransactionSizeInfo)>,
    ) -> Vec<TransactionPB> {
        api_txns
            .into_iter()
            .map(|(txn, size_info)| {
                let info = txn.transaction_info().unwrap();
                convert_transaction(
                    &txn,
                    info.block_height.unwrap().0,
                    info.epoch.unwrap().0,
                    size_info,
                )
            })
            .collect()
    }

    pub fn set_highest_known_version(&mut self) -> anyhow::Result<()> {
        let info = self.context.get_latest_ledger_info_wrapped()?;
        let latest_table_info_version = self
            .context
            .indexer_reader
            .as_ref()
            .expect("Table info reader not set")
            .get_latest_table_info_ledger_version()?
            .expect("Table info ledger version not set");

        self.highest_known_version =
            std::cmp::min(info.ledger_version.0, latest_table_info_version);

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
                    "[Indexer Fullnode] Failed to set highest known version"
                );
                continue;
            } else {
                sample!(
                    SampleRate::Frequency(10),
                    info!(
                        highest_known_version = self.highest_known_version,
                        "[Indexer Fullnode] Found new highest known version",
                    )
                );
            }
        }
    }
}

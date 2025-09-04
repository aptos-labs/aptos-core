// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{constants::IndexerGrpcRequestMetadata, timestamp_to_iso, timestamp_to_unixtime};
use velor_metrics_core::{register_gauge_vec, register_int_gauge_vec, GaugeVec, IntGaugeVec};
use velor_protos::util::timestamp::Timestamp;
use once_cell::sync::Lazy;
use prometheus::{register_int_counter_vec, IntCounterVec};

pub enum IndexerGrpcStep {
    // [Data Service] New request received.
    DataServiceNewRequestReceived,
    // [Data Service] Fetching data from in-memory cache.
    DataServiceFetchingDataFromInMemoryCache,
    // [Data Service] Waiting for data from cache.
    DataServiceWaitingForCacheData,
    // [Data Service] Fetched data from Redis cache.
    DataServiceDataFetchedCache,
    // [Data Service] Fetched data from Filestore.
    DataServiceDataFetchedFilestore,
    // [Data Service] Decoded transactions.
    DataServiceTxnsDecoded,
    // [Data Service] One chunk of transactions sent to GRPC response channel.
    DataServiceChunkSent,
    // [Data Service] All chunks of transactions sent to GRPC response channel. Current batch finished.
    DataServiceAllChunksSent,

    // [Indexer Cache] Received transactions from fullnode.
    CacheWorkerReceivedTxns,
    // [Indexer Cache] Encoded transactions.
    CacheWorkerTxnEncoded,
    // [Indexer Cache] Processed transactions in a batch.
    CacheWorkerTxnsProcessed,
    // [Indexer Cache] Successfully process current batch.
    CacheWorkerBatchProcessed,

    // [File worker] Fetch transactions from cache.
    FilestoreFetchTxns,
    // [File worker] Upload transactions to filestore.
    FilestoreUploadTxns,
    // [File worker] Update metadata to filestore.
    FilestoreUpdateMetadata,
    // [File worker] Successfully process current batch.
    FilestoreProcessedBatch,
    // [File worker] Encoded transactions.
    FileStoreEncodedTxns,

    // [Indexer Fullnode] Fetched batch of transactions from fullnode
    FullnodeFetchedBatch,
    // [Indexer Fullnode] Decoded batch of transactions from fullnode
    FullnodeDecodedBatch,
    // [Indexer Fullnode] Processed batch of transactions from fullnode
    FullnodeProcessedBatch,
    // [Indexer Fullnode] Sent batch successfully
    FullnodeSentBatch,

    // [Indexer Table Info] Processed batch of transactions from fullnode
    TableInfoProcessedBatch,
    // [Indexer Table Info] Processed transactions from fullnode
    TableInfoProcessed,
    // [Indexer Indices] Processed transactions from VelorDB
    InternalIndexerDBProcessed,
}

impl IndexerGrpcStep {
    pub fn get_step(&self) -> &'static str {
        match self {
            // Data service steps
            IndexerGrpcStep::DataServiceNewRequestReceived => "1",
            IndexerGrpcStep::DataServiceFetchingDataFromInMemoryCache => "2.0.0",
            IndexerGrpcStep::DataServiceWaitingForCacheData => "2.0",
            IndexerGrpcStep::DataServiceDataFetchedCache => "2.1",
            IndexerGrpcStep::DataServiceDataFetchedFilestore => "2.2",
            IndexerGrpcStep::DataServiceTxnsDecoded => "3.1",
            IndexerGrpcStep::DataServiceChunkSent => "3.2",
            IndexerGrpcStep::DataServiceAllChunksSent => "4",
            // Cache worker steps
            IndexerGrpcStep::CacheWorkerReceivedTxns => "1",
            IndexerGrpcStep::CacheWorkerTxnEncoded => "2",
            IndexerGrpcStep::CacheWorkerTxnsProcessed => "3",
            IndexerGrpcStep::CacheWorkerBatchProcessed => "4",
            // Filestore worker steps
            IndexerGrpcStep::FilestoreProcessedBatch => "1",
            IndexerGrpcStep::FilestoreFetchTxns => "1.0",
            IndexerGrpcStep::FileStoreEncodedTxns => "1.0.1",
            IndexerGrpcStep::FilestoreUploadTxns => "1.1",
            IndexerGrpcStep::FilestoreUpdateMetadata => "1.2",
            // Fullnode steps
            IndexerGrpcStep::FullnodeFetchedBatch => "1",
            IndexerGrpcStep::FullnodeDecodedBatch => "2",
            IndexerGrpcStep::FullnodeSentBatch => "3",
            IndexerGrpcStep::FullnodeProcessedBatch => "4",
            // Table info service steps
            IndexerGrpcStep::TableInfoProcessedBatch => "1",
            IndexerGrpcStep::TableInfoProcessed => "2",
            IndexerGrpcStep::InternalIndexerDBProcessed => "1",
        }
    }

    pub fn get_label(&self) -> &'static str {
        match self {
            // Data service steps
            IndexerGrpcStep::DataServiceNewRequestReceived => {
                "[Data Service] New request received."
            }
            IndexerGrpcStep::DataServiceFetchingDataFromInMemoryCache => {
                "[Data Service] Fetching data from in-memory cache."
            }
            IndexerGrpcStep::DataServiceWaitingForCacheData => {
                "[Data Service] Waiting for data from cache."
            }
            IndexerGrpcStep::DataServiceDataFetchedCache => "[Data Service] Data fetched from redis cache.",
            IndexerGrpcStep::DataServiceDataFetchedFilestore => {
                "[Data Service] Data fetched from file store."
            }
            IndexerGrpcStep::DataServiceTxnsDecoded => "[Data Service] Transactions decoded.",
            IndexerGrpcStep::DataServiceChunkSent => "[Data Service] One chunk of transactions sent to GRPC response channel.",
            IndexerGrpcStep::DataServiceAllChunksSent => "[Data Service] All chunks of transactions sent to GRPC response channel. Current batch finished.",
            // Cache worker steps
            IndexerGrpcStep::CacheWorkerReceivedTxns => "[Indexer Cache] Received transactions from fullnode.",
            IndexerGrpcStep::CacheWorkerTxnEncoded => "[Indexer Cache] Encoded transactions.",
            IndexerGrpcStep::CacheWorkerTxnsProcessed => "[Indexer Cache] Processed transactions in a batch.",
            IndexerGrpcStep::CacheWorkerBatchProcessed => "[Indexer Cache] Successfully process current batch.",
            // Filestore worker steps
            IndexerGrpcStep::FilestoreProcessedBatch => "[File worker] Successfully process current batch.",
            IndexerGrpcStep::FilestoreFetchTxns => "[File worker] Fetch transactions from cache.",
            IndexerGrpcStep::FilestoreUploadTxns => "[File worker] Finished uploading batch of transactions to filestore.",
            IndexerGrpcStep::FilestoreUpdateMetadata => "[File worker] Update filestore metadata.",
            IndexerGrpcStep::FileStoreEncodedTxns => "[File worker] Encoded transactions.",
            // Fullnode steps
            IndexerGrpcStep::FullnodeFetchedBatch => "[Indexer Fullnode] Fetched batch of transactions from fullnode",
            IndexerGrpcStep::FullnodeDecodedBatch => "[Indexer Fullnode] Decoded batch of transactions from fullnode",
            IndexerGrpcStep::FullnodeProcessedBatch => "[Indexer Fullnode] Processed batch of transactions from fullnode",
            IndexerGrpcStep::FullnodeSentBatch => "[Indexer Fullnode] Sent batch successfully",
            // Table info service steps
            IndexerGrpcStep::TableInfoProcessedBatch => {
                "[Indexer Table Info] Processed batch successfully"
            }
            IndexerGrpcStep::TableInfoProcessed => {
                "[Indexer Table Info] Processed successfully"
            }
            IndexerGrpcStep::InternalIndexerDBProcessed => {
                "[Indexer DB indices] Processed successfully"
            }
        }
    }
}

/// Latest processed transaction version.
pub static LATEST_PROCESSED_VERSION: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "indexer_grpc_latest_processed_version",
        "Latest processed transaction version",
        &["service_type", "step", "message"],
    )
    .unwrap()
});

/// Transactions' total size in bytes at each step
pub static TOTAL_SIZE_IN_BYTES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "indexer_grpc_total_size_in_bytes_v2",
        "Total size in bytes at this step",
        &["service_type", "step", "message"],
    )
    .unwrap()
});

/// Number of transactions at each step
pub static NUM_TRANSACTIONS_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "indexer_grpc_num_transactions_count_v2",
        "Total count of transactions at this step",
        &["service_type", "step", "message"],
    )
    .unwrap()
});

/// Number of versions that were overlapped in a multi-task fetch pull
pub static NUM_MULTI_FETCH_OVERLAPPED_VERSIONS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "indexer_grpc_num_multi_thread_fetch_overlapped_versions",
        "Number of versions that were overlapped in a multi-task fetch pull",
        &["service_type", "overlap_type"],
    )
    .unwrap()
});

/// Number of times we internally retry fetching a transaction/block
pub static TRANSACTION_STORE_FETCH_RETRIES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "indexer_grpc_num_transaction_store_fetch_retries",
        "Number of times we internally retry fetching a transaction/block",
        &["store"],
    )
    .unwrap()
});

/// Generic duration metric
pub static DURATION_IN_SECS: Lazy<GaugeVec> = Lazy::new(|| {
    register_gauge_vec!("indexer_grpc_duration_in_secs", "Duration in seconds", &[
        "service_type",
        "step",
        "message"
    ])
    .unwrap()
});

/// Transaction timestamp in unixtime
pub static TRANSACTION_UNIX_TIMESTAMP: Lazy<GaugeVec> = Lazy::new(|| {
    register_gauge_vec!(
        "indexer_grpc_transaction_unix_timestamp",
        "Transaction timestamp in unixtime",
        &["service_type", "step", "message"]
    )
    .unwrap()
});

pub fn log_grpc_step(
    service_type: &str,
    step: IndexerGrpcStep,
    start_version: Option<i64>,
    end_version: Option<i64>,
    start_version_timestamp: Option<&Timestamp>,
    end_version_timestamp: Option<&Timestamp>,
    // Duration from the start of the batch to completing the IndexerGrpcStep.
    // I chose to log this instead of individual step durations so that the whole processing duration is captured.
    duration_in_secs: Option<f64>,
    size_in_bytes: Option<usize>,
    num_transactions: Option<i64>,
    request_metadata: Option<&IndexerGrpcRequestMetadata>,
) {
    if let Some(duration_in_secs) = duration_in_secs {
        DURATION_IN_SECS
            .with_label_values(&[service_type, step.get_step(), step.get_label()])
            .set(duration_in_secs);
    }
    if let Some(num_transactions) = num_transactions {
        NUM_TRANSACTIONS_COUNT
            .with_label_values(&[service_type, step.get_step(), step.get_label()])
            .inc_by(num_transactions as u64);
    }
    if let Some(end_version) = end_version {
        LATEST_PROCESSED_VERSION
            .with_label_values(&[service_type, step.get_step(), step.get_label()])
            .set(end_version);
    }
    if let Some(end_version_timestamp) = end_version_timestamp {
        let end_txn_timestamp_unixtime = timestamp_to_unixtime(end_version_timestamp);
        TRANSACTION_UNIX_TIMESTAMP
            .with_label_values(&[service_type, step.get_step(), step.get_label()])
            .set(end_txn_timestamp_unixtime);
    }
    if let Some(size_in_bytes) = size_in_bytes {
        TOTAL_SIZE_IN_BYTES
            .with_label_values(&[service_type, step.get_step(), step.get_label()])
            .inc_by(size_in_bytes as u64);
    }

    let start_txn_timestamp_iso = start_version_timestamp.map(timestamp_to_iso);
    let end_txn_timestamp_iso = end_version_timestamp.map(timestamp_to_iso);
    if request_metadata.is_none() {
        tracing::info!(
            start_version,
            end_version,
            start_txn_timestamp_iso,
            end_txn_timestamp_iso,
            num_transactions,
            duration_in_secs,
            size_in_bytes,
            service_type,
            step = step.get_step(),
            "{}",
            step.get_label(),
        );
    } else {
        let request_metadata = request_metadata.unwrap();
        tracing::info!(
            start_version,
            end_version,
            start_txn_timestamp_iso,
            end_txn_timestamp_iso,
            num_transactions,
            duration_in_secs,
            size_in_bytes,
            // Request metadata variables
            processor_name = &request_metadata.processor_name,
            request_identifier_type = &request_metadata.request_identifier_type,
            request_identifier = &request_metadata.request_identifier,
            request_email = &request_metadata.request_email,
            request_application_name = &request_metadata.request_application_name,
            connection_id = &request_metadata.request_connection_id,
            service_type,
            step = step.get_step(),
            "{}",
            step.get_label(),
        );
    }
}

pub fn log_grpc_step_fullnode(
    step: IndexerGrpcStep,
    start_version: Option<i64>,
    end_version: Option<i64>,
    end_version_timestamp: Option<&Timestamp>,
    highest_known_version: Option<i64>,
    tps: Option<f64>,
    duration_in_secs: Option<f64>,
    num_transactions: Option<i64>,
) {
    let service_type = "indexer_fullnode";

    if let Some(duration_in_secs) = duration_in_secs {
        DURATION_IN_SECS
            .with_label_values(&[service_type, step.get_step(), step.get_label()])
            .set(duration_in_secs);
    }
    if let Some(num_transactions) = num_transactions {
        NUM_TRANSACTIONS_COUNT
            .with_label_values(&[service_type, step.get_step(), step.get_label()])
            .inc_by(num_transactions as u64);
    }
    if let Some(end_version) = end_version {
        LATEST_PROCESSED_VERSION
            .with_label_values(&[service_type, step.get_step(), step.get_label()])
            .set(end_version);
    }
    if let Some(end_version_timestamp) = end_version_timestamp {
        let end_txn_timestamp_unixtime = timestamp_to_unixtime(end_version_timestamp);
        TRANSACTION_UNIX_TIMESTAMP
            .with_label_values(&[service_type, step.get_step(), step.get_label()])
            .set(end_txn_timestamp_unixtime);
    }

    tracing::info!(
        start_version,
        end_version,
        num_transactions,
        duration_in_secs,
        highest_known_version,
        tps,
        service_type,
        step = step.get_step(),
        "{}",
        step.get_label(),
    );
}

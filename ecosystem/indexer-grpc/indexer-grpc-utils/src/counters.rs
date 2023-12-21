// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{constants::IndexerGrpcRequestMetadata, timestamp_to_iso, timestamp_to_unixtime};
use aptos_metrics_core::{register_gauge_vec, register_int_gauge_vec, GaugeVec, IntGaugeVec};
use aptos_protos::util::timestamp::Timestamp;
use once_cell::sync::Lazy;

pub enum IndexerGrpcStep {
    DataServiceNewRequestReceived,   // [Data Service] New request received.
    DataServiceWaitingForCacheData,  // [Data Service] Waiting for data from cache.
    DataServiceDataFetchedCache,     // [Data Service] Fetched data from Redis cache.
    DataServiceDataFetchedFilestore, // [Data Service] Fetched data from Filestore.
    DataServiceTxnsDecoded,          // [Data Service] Decoded transactions.
    DataServiceChunkSent, // [Data Service] One chunk of transactions sent to GRPC response channel.
    DataServiceAllChunksSent, // [Data Service] All chunks of transactions sent to GRPC response channel. Current batch finished.

    CacheWorkerTxnsProcessed, // [Indexer Cache] Processed transactions in a batch.
    CacheWorkerBatchProcessed, // [Indexer Cache] Successfully process current batch.

    FilestoreUploadTxns, // [File worker] Upload transactions to filestore.

    FullnodeFetchedBatch, // [Indexer Fullnode] Fetched batch of transactions from fullnode
    FullnodeDecodedBatch, // [Indexer Fullnode] Decoded batch of transactions from fullnode
    FullnodeProcessedBatch, // [Indexer Fullnode] Processed batch of transactions from fullnode
    FullnodeSentBatch,    // [Indexer Fullnode] Sent batch successfully

    TableInfoProcessedBatch, // [Indexer Table Info] Processed batch of transactions from fullnode
    TableInfoProcessed,      // [Indexer Table Info] Processed transactions from fullnode
}

impl IndexerGrpcStep {
    pub fn get_step(&self) -> &'static str {
        match self {
            // Data service steps
            IndexerGrpcStep::DataServiceNewRequestReceived => "1",
            IndexerGrpcStep::DataServiceWaitingForCacheData => "2.0",
            IndexerGrpcStep::DataServiceDataFetchedCache => "2.1",
            IndexerGrpcStep::DataServiceDataFetchedFilestore => "2.2",
            IndexerGrpcStep::DataServiceTxnsDecoded => "3.1",
            IndexerGrpcStep::DataServiceChunkSent => "3.2",
            IndexerGrpcStep::DataServiceAllChunksSent => "4",
            // Cache worker steps
            IndexerGrpcStep::CacheWorkerTxnsProcessed => "1",
            IndexerGrpcStep::CacheWorkerBatchProcessed => "2",
            // Filestore worker steps
            IndexerGrpcStep::FilestoreUploadTxns => "1",
            // Fullnode steps
            IndexerGrpcStep::FullnodeFetchedBatch => "1",
            IndexerGrpcStep::FullnodeDecodedBatch => "2",
            IndexerGrpcStep::FullnodeSentBatch => "3",
            IndexerGrpcStep::FullnodeProcessedBatch => "4",
            // Table info service steps
            IndexerGrpcStep::TableInfoProcessedBatch => "1",
            IndexerGrpcStep::TableInfoProcessed => "2",
        }
    }

    pub fn get_label(&self) -> &'static str {
        match self {
            // Data service steps
            IndexerGrpcStep::DataServiceNewRequestReceived => {
                "[Data Service] New request received."
            },
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
            IndexerGrpcStep::CacheWorkerTxnsProcessed => "[Indexer Cache] Processed transactions in a batch.",
            IndexerGrpcStep::CacheWorkerBatchProcessed => "[Indexer Cache] Successfully process current batch.",
            // Filestore worker steps
            IndexerGrpcStep::FilestoreUploadTxns => "[File worker] Upload transactions to filestore.",
            // Fullnode steps
            IndexerGrpcStep::FullnodeFetchedBatch => "[Indexer Fullnode] Fetched batch of transactions from fullnode",
            IndexerGrpcStep::FullnodeDecodedBatch => "[Indexer Fullnode] Decoded batch of transactions from fullnode",
            IndexerGrpcStep::FullnodeProcessedBatch => "[Indexer Fullnode] Processed batch of transactions from fullnode",
            IndexerGrpcStep::FullnodeSentBatch => "[Indexer Fullnode] Sent batch successfully",
            // Table info service steps
            IndexerGrpcStep::TableInfoProcessedBatch => {
                "[Indexer Table Info] Processed batch successfully"
            },
            IndexerGrpcStep::TableInfoProcessed => {
                "[Indexer Table Info] Processed successfully"
            },
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
pub static TOTAL_SIZE_IN_BYTES: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "indexer_grpc_total_size_in_bytes",
        "Total size in bytes at this step",
        &["service_type", "step", "message"],
    )
    .unwrap()
});

/// Number of transactions at each step
pub static NUM_TRANSACTIONS_COUNT: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "indexer_grpc_num_transactions_count",
        "Total count of transactions at this step",
        &["service_type", "step", "message"],
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
    enable_logging: bool,
    start_version: Option<i64>,
    end_version: Option<i64>,
    start_version_timestamp: Option<&Timestamp>,
    end_version_timestamp: Option<&Timestamp>,
    // Duration from the start of the batch to completing the IndexerGrpcStep.
    // I chose to log this instead of individual step durations so that the whole processing duration is captured.
    duration_in_secs: Option<f64>,
    size_in_bytes: Option<usize>,
    num_transactions: Option<i64>,
    request_metadata: Option<IndexerGrpcRequestMetadata>,
) {
    if let Some(duration_in_secs) = duration_in_secs {
        DURATION_IN_SECS
            .with_label_values(&[service_type, step.get_step(), step.get_label()])
            .set(duration_in_secs);
    }
    if let Some(num_transactions) = num_transactions {
        NUM_TRANSACTIONS_COUNT
            .with_label_values(&[service_type, step.get_step(), step.get_label()])
            .set(num_transactions);
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
            .set(size_in_bytes as i64);
    }

    if enable_logging {
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
            tracing::info!(
                start_version,
                end_version,
                start_txn_timestamp_iso,
                end_txn_timestamp_iso,
                num_transactions,
                duration_in_secs,
                size_in_bytes,
                // Request metadata variables
                request_name = request_metadata.clone().unwrap().processor_name.as_str(),
                request_email = request_metadata.clone().unwrap().request_email.as_str(),
                request_api_key_name = request_metadata
                    .clone()
                    .unwrap()
                    .request_api_key_name
                    .as_str(),
                processor_name = request_metadata.clone().unwrap().processor_name.as_str(),
                connection_id = request_metadata
                    .clone()
                    .unwrap()
                    .request_connection_id
                    .as_str(),
                request_user_classification = request_metadata
                    .unwrap()
                    .request_user_classification
                    .as_str(),
                service_type,
                step = step.get_step(),
                "{}",
                step.get_label(),
            );
        }
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
            .set(num_transactions);
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

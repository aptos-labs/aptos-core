// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{register_gauge_vec, register_int_gauge_vec, GaugeVec, IntGaugeVec};
use once_cell::sync::Lazy;

pub enum IndexerGrpcStep {
    DataServiceNewRequestReceived, // [Data Service] New request received.

    DataServiceDataFetchedInMemory, // [Data Service] Fetched data from in-memory cache.
    InMemoryCacheFetchedTxns,       // [In-memory Cache] In-memory cache fetched transactions.
    InMemoryCacheDecodedTxns,       // [In-memory Cache] In-memory cache decoded transactions.

    DataServiceDataFetchedCache, // [Data Service] Fetched data from Redis cache.
    RedisCacheFetchedTxns,       // [Redis Cache] Redis cache fetched transactions.
    RedisCacheDecodedTxns,       // [Redis Cache] Redis cache decoded transactions.

    DataServiceDataFetchedFilestore, // [Data Service] Fetched data from Filestore.
    FilestoreFetchedTxns,            // [Filestore] Filestore fetched transactions.
    FilestoreDecodedTxns,            // [Filestore] Filestore decoded transactions.

    DataServiceDataFetchedUnknown, // [Data Service] Fetched data from unknown source.
    DataServiceTxnsDecoded,        // [Data Service] Decoded transactions.
    DataServiceChunkSent, // [Data Service] One chunk of transactions sent to GRPC response channel.
    DataServiceAllChunksSent, // [Data Service] All chunks of transactions sent to GRPC response channel. Current batch finished.

    CacheWorkerTxnsProcessed, // [Indexer Cache] Processed transactions in a batch.
    CacheWorkerBatchProcessed, // [Indexer Cache] Successfully process current batch.

    FilestoreUploadTxns, // [File worker] Upload transactions to filestore.

    FullnodeFetchedBatch, // [Indexer Fullnode] Fetched batch of transactions from fullnode
    FullnodeDecodedBatch, // [Indexer Fullnode] Decoded batch of transactions from fullnode
    FullnodeProcessedBatch, // [Indexer Fullnode] Processed batch of transactions from fullnode
    FullnodeSentBatch,    // [Indexer Fullnode] Sent batch successfully
}

impl IndexerGrpcStep {
    pub fn get_step(&self) -> &'static str {
        match self {
            // Data service steps
            IndexerGrpcStep::DataServiceNewRequestReceived => "1",
            IndexerGrpcStep::DataServiceDataFetchedInMemory => "2.0",
            IndexerGrpcStep::InMemoryCacheFetchedTxns => "2.0.1",
            IndexerGrpcStep::InMemoryCacheDecodedTxns => "2.0.2",
            IndexerGrpcStep::DataServiceDataFetchedCache => "2.1",
            IndexerGrpcStep::RedisCacheFetchedTxns => "2.1.1",
            IndexerGrpcStep::RedisCacheDecodedTxns => "2.1.2",
            IndexerGrpcStep::DataServiceDataFetchedFilestore => "2.2",
            IndexerGrpcStep::FilestoreFetchedTxns => "2.2.1",
            IndexerGrpcStep::FilestoreDecodedTxns => "2.2.2",
            IndexerGrpcStep::DataServiceDataFetchedUnknown => "2.x",
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
            IndexerGrpcStep::FullnodeProcessedBatch => "3",
            IndexerGrpcStep::FullnodeSentBatch => "4",
        }
    }

    pub fn get_label(&self) -> &'static str {
        match self {
            // Data service steps
            IndexerGrpcStep::DataServiceNewRequestReceived => {
                "[Data Service] New request received."
            }
            IndexerGrpcStep::DataServiceDataFetchedInMemory => {
                "[Data Service] Data fetched from in-memory cache."
            }
            IndexerGrpcStep::InMemoryCacheFetchedTxns => "[In-memory Cache] In-memory cache fetched transactions.",
            IndexerGrpcStep::InMemoryCacheDecodedTxns => "[In-memory Cache] In-memory cache decoded transactions.",
            IndexerGrpcStep::DataServiceDataFetchedCache => "[Data Service] Data fetched from redis cache.",
            IndexerGrpcStep ::RedisCacheFetchedTxns => "[Redis Cache] Redis cache fetched transactions.",
            IndexerGrpcStep::RedisCacheDecodedTxns => "[Redis Cache] Redis cache decoded transactions.",
            IndexerGrpcStep::DataServiceDataFetchedFilestore => {
                "[Data Service] Data fetched from file store."
            }
            IndexerGrpcStep::FilestoreFetchedTxns => "[Filestore] Filestore fetched transactions.",
            IndexerGrpcStep::FilestoreDecodedTxns => "[Filestore] Filestore decoded transactions.",
            IndexerGrpcStep::DataServiceDataFetchedUnknown => {
                "[Data Service] Data fetched from unknown source."
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

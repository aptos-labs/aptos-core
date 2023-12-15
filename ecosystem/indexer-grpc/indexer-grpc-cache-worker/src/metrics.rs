// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{
    register_gauge, register_int_counter, register_int_counter_vec, register_int_gauge, Gauge,
    IntCounter, IntCounterVec, IntGauge,
};
use once_cell::sync::Lazy;

/// Latest processed transaction version.
pub static LATEST_PROCESSED_VERSION: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "indexer_grpc_cache_worker_latest_processed_version",
        "Latest processed transaction version",
    )
    .unwrap()
});

/// Number of transactions that saved into cache.
pub static PROCESSED_VERSIONS_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "indexer_grpc_cache_worker_processed_versions",
        "Number of transactions that have been processed by cache worker",
    )
    .unwrap()
});

/// Number of errors that cache worker has encountered.
pub static ERROR_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "indexer_grpc_cache_worker_errors",
        "Number of errors that cache worker has encountered",
        &["error_type"]
    )
    .unwrap()
});

/// Data latency for cache worker based on latest processed transaction.
pub static PROCESSED_LATENCY_IN_SECS: Lazy<Gauge> = Lazy::new(|| {
    register_gauge!(
        "indexer_grpc_cache_worker_data_latency_in_secs",
        "Latency of cache worker based on latest processed transaction",
    )
    .unwrap()
});

/// Number of transactions in each batch that cache worker has processed.
pub static PROCESSED_BATCH_SIZE: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "indexer_grpc_cache_worker_processed_batch_size",
        "Size of latest processed batch by cache worker",
    )
    .unwrap()
});

/// Number of waits that cache worker has encountered for file store.
pub static WAIT_FOR_FILE_STORE_COUNTER: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "indexer_grpc_cache_worker_wait_for_file_store_counter",
        "Number of waits that cache worker has encountered for file store",
    )
    .unwrap()
});

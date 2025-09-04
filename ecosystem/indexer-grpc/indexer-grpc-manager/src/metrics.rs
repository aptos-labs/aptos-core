// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_metrics_core::{
    exponential_buckets, register_histogram_vec, register_int_counter, register_int_counter_vec,
    register_int_gauge, register_int_gauge_vec, HistogramVec, IntCounter, IntCounterVec, IntGauge,
    IntGaugeVec,
};
use once_cell::sync::Lazy;

pub static FILE_STORE_VERSION: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "velor_indexer_grpc_v2_file_store_version",
        "File store version (next_version)."
    )
    .unwrap()
});

pub static FILE_STORE_VERSION_IN_CACHE: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "velor_indexer_grpc_v2_file_store_version_in_cache",
        "File store version in cache."
    )
    .unwrap()
});

pub static FILE_STORE_UPLOADED_BYTES: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "velor_indexer_grpc_v2_file_store_uploaded_bytes",
        "# of bytes uploaded to file store."
    )
    .unwrap()
});

pub static IS_MASTER: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "velor_indexer_grpc_v2_is_master",
        "Is this instance the master instance?"
    )
    .unwrap()
});

pub static IS_FILE_STORE_LAGGING: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "velor_indexer_grpc_v2_is_file_store_lagging",
        "Is file store lagging?"
    )
    .unwrap()
});

pub static CACHE_SIZE: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "velor_indexer_grpc_v2_cache_size",
        "The cache_size of the grpc manager instance."
    )
    .unwrap()
});

pub static MAX_CACHE_SIZE: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "velor_indexer_grpc_v2_max_cache_size",
        "The max_cache_size of the grpc manager instance."
    )
    .unwrap()
});

pub static TARGET_CACHE_SIZE: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "velor_indexer_grpc_v2_target_cache_size",
        "The target_cache_size of the grpc manager instance."
    )
    .unwrap()
});

pub static CACHE_START_VERSION: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "velor_indexer_grpc_v2_grpc_manager_cache_start_version",
        "The cache_start_version of the grpc manager instance."
    )
    .unwrap()
});

pub static CACHE_END_VERSION: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "velor_indexer_grpc_v2_grpc_manager_cache_end_version",
        "The cache_end_version (exclusive) of the grpc manager instance."
    )
    .unwrap()
});

pub static KNOWN_LATEST_VERSION: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "velor_indexer_grpc_v2_grpc_manager_known_latest_version",
        "The known_latest_version of the grpc manager instance."
    )
    .unwrap()
});

pub static CONNECTED_INSTANCES: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "velor_indexer_grpc_v2_grpc_manager_connected_instances",
        "The # of connected instances of each service type.",
        &["service_type"],
    )
    .unwrap()
});

pub static COUNTER: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_indexer_grpc_v2_grpc_manager_counter",
        "Generic counter for various things.",
        &["name"],
    )
    .unwrap()
});

pub static TIMER: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "velor_indexer_grpc_v2_grpc_manager_timer",
        "Generic timer for various things.",
        &["name"],
        exponential_buckets(/*start=*/ 1e-6, /*factor=*/ 2.0, /*count=*/ 30).unwrap(),
    )
    .unwrap()
});

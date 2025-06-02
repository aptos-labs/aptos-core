// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{
    exponential_buckets, register_histogram_vec, register_int_counter_vec, register_int_gauge,
    register_int_gauge_vec, HistogramVec, IntCounterVec, IntGauge, IntGaugeVec,
};
use once_cell::sync::Lazy;

pub static NUM_CONNECTED_STREAMS: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "aptos_indexer_grpc_v2_num_connected_streams",
        "# of connected streams.",
        &["data_service_type"]
    )
    .unwrap()
});

pub static CACHE_START_VERSION: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_indexer_grpc_v2_live_data_service_cache_start_version",
        "The cache_start_version of the live data service instance."
    )
    .unwrap()
});

pub static CACHE_END_VERSION: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_indexer_grpc_v2_live_data_service_cache_end_version",
        "The cache_end_version (exclusive) of the live data service instance."
    )
    .unwrap()
});

pub static CACHE_SIZE_BYTES: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_indexer_grpc_v2_live_data_service_cache_size_bytes",
        "Cache size in bytes."
    )
    .unwrap()
});

pub static CACHE_SIZE_LIMIT_BYTES: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_indexer_grpc_v2_live_data_service_cache_size_limit_bytes",
        "Limit of cache size in bytes."
    )
    .unwrap()
});

pub static LATENCY_MS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_indexer_grpc_v2_live_data_service_latency_ms",
        "The latency of live data service (comparing with txn timestamp)."
    )
    .unwrap()
});

pub static COUNTER: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_indexer_grpc_v2_data_service_counter",
        "Generic counter for various things.",
        &["name"],
    )
    .unwrap()
});

pub static TIMER: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_indexer_grpc_v2_data_service_timer",
        "Generic timer for various things.",
        &["name"],
        exponential_buckets(/*start=*/ 1e-6, /*factor=*/ 2.0, /*count=*/ 30).unwrap(),
    )
    .unwrap()
});

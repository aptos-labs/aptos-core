// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{
    register_gauge_vec, register_histogram_vec, register_int_counter_vec, register_int_gauge_vec,
    GaugeVec, HistogramVec, IntCounterVec, IntGaugeVec,
};
use once_cell::sync::Lazy;

/// Latest processed transaction version.
pub static LATEST_PROCESSED_VERSION: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "indexer_grpc_data_service_latest_processed_version",
        "Latest processed transaction version",
        &["request_token", "processor_name"],
    )
    .unwrap()
});

/// Observed latest processed transaction version.
pub static OBSERVED_LATEST_PROCESSED_VERSION: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "indexer_grpc_data_service_observed_latest_processed_version",
        "Latest processed transaction version",
        &["request_token", "processor_name"],
    )
    .unwrap()
});

/// Number of transactions that served by data service.
pub static PROCESSED_VERSIONS_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "indexer_grpc_data_service_processed_versions",
        "Number of transactions that have been processed by data service",
        &["request_token", "processor_name"],
    )
    .unwrap()
});

/// Number of errors that data service has encountered.
pub static ERROR_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "indexer_grpc_data_service_error",
        "Number of errors that data service has encountered",
        &["error_type"]
    )
    .unwrap()
});

/// Data latency for data service based on latest processed transaction based on selected processor.
pub static PROCESSED_LATENCY_IN_SECS: Lazy<GaugeVec> = Lazy::new(|| {
    register_gauge_vec!(
        "indexer_grpc_data_service_latest_data_latency_in_secs",
        "Latency of data service based on latest processed transaction",
        &["request_token", "processor_name"],
    )
    .unwrap()
});

/// Data latency for data service based on latest processed transaction for all processors.
pub static PROCESSED_LATENCY_IN_SECS_ALL: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "indexer_grpc_data_service_latest_data_latency_in_secs_all",
        "Latency of data service based on latest processed transaction",
        &["request_token"]
    )
    .unwrap()
});

/// Number of transactions in each batch that data service has processed.
pub static PROCESSED_BATCH_SIZE: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "indexer_grpc_data_service_processed_batch_size",
        "Size of latest processed batch by data service",
        &["request_token", "processor_name"],
    )
    .unwrap()
});

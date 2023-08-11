// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{
    register_counter, register_gauge_vec, register_gauge,register_int_counter, register_int_counter_vec, Counter,
    GaugeVec, IntCounter, IntCounterVec, Gauge,
};
use once_cell::sync::Lazy;

/// Indexer GRPC latency against PFN in seconds.
pub static INDEXER_GRPC_LATENCY_AGAINST_PFN_LATENCY_IN_SECS: Lazy<GaugeVec> = Lazy::new(|| {
    register_gauge_vec!(
        "indexer_grpc_latency_against_pfn_in_secs",
        "Indexer GRPC latency against PFN in seconds.",
        &["pfn_address"],
    )
    .unwrap()
});

/// Verification error count.
pub static VERIFICATION_ERROR_COUNT: Lazy<Counter> = Lazy::new(|| {
    register_counter!(
        "indexer_grpc_post_processor_verification_error_count",
        "Verification error count.",
    )
    .unwrap()
});

/// Task failure count.
pub static TASK_FAILURE_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "indexer_grpc_post_processor_task_failure_count",
        "Task failure count.",
        &["task_name"],
    )
    .unwrap()
});

/// Data Service Checker transaction count.
pub static DATA_SERVICE_CHECKER_TRANSACTION_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "indexer_grpc_post_processor_data_service_checker_transaction_count",
        "Data Service Checker transaction count.",
    )
    .unwrap()
});

/// Data Service Checker transaction tps gauge.
pub static DATA_SERVICE_CHECKER_TRANSACTION_TPS: Lazy<Gauge> = Lazy::new(|| {
    register_gauge!(
        "indexer_grpc_post_processor_data_service_checker_transaction_tps",
        "Data Service Checker transaction tps.",
    )
    .unwrap()
});

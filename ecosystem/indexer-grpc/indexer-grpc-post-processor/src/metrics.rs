// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{
    register_counter, register_gauge_vec, register_int_counter_vec, Counter, GaugeVec,
    IntCounterVec,
};
use once_cell::sync::Lazy;

/// Latest observed transaction timestamp vs current timestamp.
/// Node type can be "pfn" or "indexer".
pub static OBSERVED_LATEST_TRANSACTION_LATENCY: Lazy<GaugeVec> = Lazy::new(|| {
    register_gauge_vec!(
        "indexer_grpc_post_processor_observed_transaction_latency_in_secs",
        "Latest observed transaction timestamp vs current timestamp.",
        &["node_type"],
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

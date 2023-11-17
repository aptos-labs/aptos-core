// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{register_gauge_vec, register_int_gauge_vec, GaugeVec, IntGaugeVec};
use once_cell::sync::Lazy;

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

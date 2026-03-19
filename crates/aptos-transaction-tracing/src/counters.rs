// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_metrics_core::{register_histogram_vec, HistogramVec};
use once_cell::sync::Lazy;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Histogram buckets for tracing latencies (in seconds).
const TRACING_BUCKETS: &[f64] = &[
    0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0,
];

/// Per-transaction latency from mempool insertion to each lifecycle stage,
/// grouped by sender address. Safe cardinality since only allowlisted addresses
/// are traced.
pub static TXN_TRACING: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_transaction_tracing",
        "Per-transaction latency from mempool insertion to each lifecycle stage",
        &["sender", "stage"],
        TRACING_BUCKETS.to_vec()
    )
    .unwrap()
});

/// Record latency from insertion to this stage, grouped by sender.
pub fn observe_stage_latency(insertion_time_usecs: u64, sender: &str, stage: &str) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO);
    if let Some(latency) = now.checked_sub(Duration::from_micros(insertion_time_usecs)) {
        TXN_TRACING
            .with_label_values(&[sender, stage])
            .observe(latency.as_secs_f64());
    }
}

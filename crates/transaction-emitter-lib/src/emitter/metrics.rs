// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Prometheus metrics for the transaction emitter.
//!
//! These metrics provide real-time visibility into transaction submission performance
//! and can be pushed to vmagent or any Prometheus-compatible endpoint.

use aptos_metrics_core::{
    exponential_buckets, register_histogram, register_int_counter, register_int_gauge, Histogram,
    IntCounter, IntGauge,
};
use once_cell::sync::Lazy;

/// Counter for total transactions submitted across all workers.
pub static TXN_EMITTER_SUBMITTED: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "txn_emitter_submitted_total",
        "Total number of transactions submitted by the emitter"
    )
    .unwrap()
});

/// Counter for total transactions committed.
pub static TXN_EMITTER_COMMITTED: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "txn_emitter_committed_total",
        "Total number of transactions committed on chain"
    )
    .unwrap()
});

/// Counter for total transactions that expired without being committed.
pub static TXN_EMITTER_EXPIRED: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "txn_emitter_expired_total",
        "Total number of transactions that expired"
    )
    .unwrap()
});

/// Counter for total failed transaction submissions.
pub static TXN_EMITTER_FAILED_SUBMISSION: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "txn_emitter_failed_submission_total",
        "Total number of failed transaction submissions"
    )
    .unwrap()
});

/// Histogram for transaction latency in seconds.
/// Buckets range from 100ms to ~26 seconds (exponential).
pub static TXN_EMITTER_LATENCY_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "txn_emitter_latency_seconds",
        "Transaction latency from submission to commit in seconds",
        // Buckets from 0.1s to ~26s
        exponential_buckets(0.1, 2.0, 9).unwrap()
    )
    .unwrap()
});

/// Gauge for current committed transactions per second.
pub static TXN_EMITTER_COMMITTED_TPS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "txn_emitter_committed_tps",
        "Current committed transactions per second"
    )
    .unwrap()
});

/// Gauge for current submitted transactions per second.
pub static TXN_EMITTER_SUBMITTED_TPS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "txn_emitter_submitted_tps",
        "Current submitted transactions per second"
    )
    .unwrap()
});

/// Records transaction submission stats.
pub fn record_submission_stats(submitted: u64, failed: u64) {
    TXN_EMITTER_SUBMITTED.inc_by(submitted);
    TXN_EMITTER_FAILED_SUBMISSION.inc_by(failed);
}

/// Records transaction commit stats.
pub fn record_commit_stats(committed: u64, expired: u64) {
    TXN_EMITTER_COMMITTED.inc_by(committed);
    TXN_EMITTER_EXPIRED.inc_by(expired);
}

/// Records transaction latency in milliseconds for a batch of transactions.
/// The latency is recorded once per transaction in the batch to ensure
/// histogram percentiles are accurate.
pub fn record_latency_ms(latency_ms: u64, count: u64) {
    // Convert ms to seconds for the histogram
    let latency_secs = latency_ms as f64 / 1000.0;
    // Record one observation per transaction to properly weight the histogram
    for _ in 0..count {
        TXN_EMITTER_LATENCY_SECONDS.observe(latency_secs);
    }
}

/// Updates TPS gauges with current rates.
pub fn update_tps_gauges(committed_tps: f64, submitted_tps: f64) {
    TXN_EMITTER_COMMITTED_TPS.set(committed_tps as i64);
    TXN_EMITTER_SUBMITTED_TPS.set(submitted_tps as i64);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_submission_stats() {
        // Just verify it doesn't panic
        record_submission_stats(100, 5);
    }

    #[test]
    fn test_record_commit_stats() {
        record_commit_stats(95, 5);
    }

    #[test]
    fn test_record_latency() {
        record_latency_ms(500, 10); // 500ms for 10 txns
        record_latency_ms(1000, 5); // 1s for 5 txns
    }

    #[test]
    fn test_update_tps_gauges() {
        update_tps_gauges(1000.5, 1100.0);
    }
}

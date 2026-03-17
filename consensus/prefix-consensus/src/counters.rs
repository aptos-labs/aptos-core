// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Prometheus metrics for the inner PC rounds within SPC.

use aptos_metrics_core::{register_histogram_vec, HistogramVec};
use once_cell::sync::Lazy;

/// Bucket boundaries (in seconds) covering sub-ms to 5s.
const ROUND_DURATION_BUCKETS: &[f64] = &[
    0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.2, 0.3, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 5.0,
];

/// Latency histogram for each round of the inner PC protocol within SPC.
///
/// Round labels:
///   "round1" — Vote1 broadcast → QC1 formation
///   "round2" — Vote2 broadcast → QC2 formation
///   "round3" — Vote3 broadcast → QC3 formation
pub static SPC_ROUND_DURATION: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "pc_spc_round_duration_s",
        "Latency histogram for each round of the inner PC protocol within SPC",
        &["round"],
        ROUND_DURATION_BUCKETS.to_vec()
    )
    .unwrap()
});

// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Prometheus metrics for the inner PC rounds within SPC.

use aptos_metrics_core::{
    register_histogram, register_histogram_vec, register_int_counter_vec, Histogram, HistogramVec,
    IntCounterVec,
};
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
///
pub static SPC_ROUND_DURATION: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "pc_spc_round_duration_s",
        "Latency histogram for each round of the inner PC protocol within SPC",
        &["round"],
        ROUND_DURATION_BUCKETS.to_vec()
    )
    .unwrap()
});

/// Bucket boundaries (in seconds) covering 1ms to 120s (to catch long delays).
const VIEW_DURATION_BUCKETS: &[f64] = &[
    0.001, 0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0, 60.0, 90.0, 120.0,
];

/// Time from "Entering View N" to "Starting inner PC for View N".
///
/// Labels:
///   "timer"                — VIEW_START_TIMEOUT fallback fired
///   "first_ranked_cert"    — rank-0 proposal arrived via try_start_pc
///   "enter_view_immediate" — rank-0 cert already available at enter_view
pub static SPC_VIEW_ENTER_TO_PC_START: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "pc_spc_view_enter_to_pc_start_s",
        "Time from entering a view to starting its inner PC",
        &["trigger"],
        VIEW_DURATION_BUCKETS.to_vec()
    )
    .unwrap()
});

/// Wall-clock time for View 1 inner PC to complete (start_view1 → handle_view1_complete).
pub static SPC_VIEW1_DURATION: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "pc_spc_view1_duration_s",
        "Wall-clock time for View 1 inner PC to complete",
        VIEW_DURATION_BUCKETS.to_vec()
    )
    .unwrap()
});

/// Counter: what triggered the inner PC start for views > 1.
///
/// Labels: same as SPC_VIEW_ENTER_TO_PC_START.
pub static SPC_PC_START_TRIGGER: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "pc_spc_pc_start_trigger",
        "What triggered the inner PC start: timer, first_ranked_cert, or enter_view_immediate",
        &["trigger"]
    )
    .unwrap()
});

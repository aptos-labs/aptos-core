// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Prometheus metrics for prefix consensus slot pipeline.

use aptos_metrics_core::{register_histogram_vec, HistogramVec};
use once_cell::sync::Lazy;

/// Bucket boundaries (in seconds) covering sub-ms to 5s.
const SLOT_DURATION_BUCKETS: &[f64] = &[
    0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.2, 0.3, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 5.0,
];

/// Latency histogram for each stage of a prefix consensus slot.
///
/// Stage labels:
///   "payload_pull"          — pull_payload() duration
///   "proposal_wait"         — proposal broadcast → SPC start
///   "spc_to_vlow"           — SPC start → v_low output
///   "vlow_to_vhigh"         — v_low → v_high output
///   "vlow_entry_resolution" — resolve missing v_low entries
///   "vlow_commit_wave"      — build + send v_low blocks
///   "vhigh_entry_resolution"— resolve missing v_high delta entries
///   "vhigh_commit_wave"     — build + send v_high blocks
///   "finalization"          — ranking update + cleanup
///   "total"                 — end-to-end slot duration
pub static SLOT_DURATION: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "pc_slot_duration_s",
        "Latency histogram for each stage of a prefix consensus slot",
        &["stage"],
        SLOT_DURATION_BUCKETS.to_vec()
    )
    .unwrap()
});

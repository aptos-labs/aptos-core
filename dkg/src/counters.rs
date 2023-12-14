// Copyright © Aptos Foundation

use aptos_metrics_core::{register_histogram_vec, register_int_gauge, HistogramVec, IntGauge};
use once_cell::sync::Lazy;

/// Traces latency throughout the DKG process
pub static DKG_TRACING: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_dkg_tracing",
        "Histogram for different stages of the DKG process",
        &["stage"]
    )
    .unwrap()
});

/// Monitor counters, used by monitor! macro
pub static OP_COUNTERS: Lazy<aptos_metrics_core::op_counters::OpMetrics> =
    Lazy::new(|| aptos_metrics_core::op_counters::OpMetrics::new_and_registered("dkg"));

/// Count of the pending messages sent to itself in the channel
pub static PENDING_SELF_MESSAGES: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_dkg_pending_self_messages",
        "Count of the pending messages sent to itself in the channel"
    )
    .unwrap()
});

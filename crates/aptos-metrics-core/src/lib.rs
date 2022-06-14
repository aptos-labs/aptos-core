// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

// Re-export counter types from prometheus crate
pub use prometheus::{
    gather, register_histogram, register_histogram_vec, register_int_counter,
    register_int_counter_vec, register_int_gauge, register_int_gauge_vec, Encoder, Histogram,
    HistogramTimer, HistogramVec, IntCounter, IntCounterVec, IntGauge, IntGaugeVec, TextEncoder,
};

pub mod op_counters;

/// Helper function to record metrics for external calls.
/// Include call counts, time, and whether it's inside or not (1 or 0).
/// It assumes a OpMetrics defined as OP_COUNTERS in crate::counters;
#[macro_export]
macro_rules! monitor {
    ( $name:literal, $fn:expr ) => {{
        use crate::counters::OP_COUNTERS;
        let _timer = OP_COUNTERS.timer($name);
        let gauge = OP_COUNTERS.gauge(concat!($name, "_running"));
        gauge.inc();
        let result = $fn;
        gauge.dec();
        result
    }};
}

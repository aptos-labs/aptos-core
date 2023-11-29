// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

// Re-export counter types from prometheus crate
pub use prometheus::{
    exponential_buckets, gather, histogram_opts, register_counter, register_gauge,
    register_gauge_vec, register_histogram, register_histogram_vec, register_int_counter,
    register_int_counter_vec, register_int_gauge, register_int_gauge_vec, Counter, Encoder, Gauge,
    GaugeVec, Histogram, HistogramTimer, HistogramVec, IntCounter, IntCounterVec, IntGauge,
    IntGaugeVec, TextEncoder,
};

mod avg_counter;
pub use avg_counter::register_avg_counter;
pub mod const_metric;
pub mod op_counters;

pub trait TimerHelper {
    fn timer_with(&self, labels: &[&str]) -> HistogramTimer;
}

impl TimerHelper for HistogramVec {
    fn timer_with(&self, vals: &[&str]) -> HistogramTimer {
        self.with_label_values(vals).start_timer()
    }
}

pub trait IntGaugeHelper {
    fn set_with(&self, labels: &[&str], val: i64);
}

impl IntGaugeHelper for IntGaugeVec {
    fn set_with(&self, labels: &[&str], val: i64) {
        self.with_label_values(labels).set(val)
    }
}

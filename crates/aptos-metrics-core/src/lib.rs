// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

// Re-export counter types from prometheus crate
pub use crate::{
    avg_counter::{register_avg_counter, register_avg_counter_vec},
    thread_local::{LocalHistogramVec, LocalIntCounter},
};
pub use prometheus::{
    exponential_buckets, gather, histogram_opts, register_counter, register_gauge,
    register_gauge_vec, register_histogram, register_histogram_vec, register_int_counter,
    register_int_counter_vec, register_int_gauge, register_int_gauge_vec, Counter, Encoder, Gauge,
    GaugeVec, Histogram, HistogramTimer, HistogramVec, IntCounter, IntCounterVec, IntGauge,
    IntGaugeVec, TextEncoder,
};

mod avg_counter;
pub mod const_metric;
pub mod op_counters;
pub mod thread_local;

pub trait TimerHelper {
    type TimerType;

    fn timer_with(&'static self, labels: &[&str]) -> Self::TimerType;

    fn observe_with(&'static self, labels: &[&str], val: f64);
}

impl TimerHelper for HistogramVec {
    type TimerType = HistogramTimer;

    fn timer_with(&'static self, labels: &[&str]) -> Self::TimerType {
        self.with_label_values(labels).start_timer()
    }

    fn observe_with(&'static self, labels: &[&str], val: f64) {
        self.with_label_values(labels).observe(val);
    }
}

pub struct ConcurrencyGauge {
    gauge: IntGauge,
}

impl ConcurrencyGauge {
    fn new(gauge: IntGauge) -> Self {
        gauge.inc();
        Self { gauge }
    }
}

impl Drop for ConcurrencyGauge {
    fn drop(&mut self) {
        self.gauge.dec();
    }
}

pub trait IntGaugeVecHelper {
    fn set_with(&self, labels: &[&str], val: i64);

    fn concurrency_with(&self, labels: &[&str]) -> ConcurrencyGauge;
}

impl IntGaugeVecHelper for IntGaugeVec {
    fn set_with(&self, labels: &[&str], val: i64) {
        self.with_label_values(labels).set(val)
    }

    fn concurrency_with(&self, labels: &[&str]) -> ConcurrencyGauge {
        ConcurrencyGauge::new(self.with_label_values(labels))
    }
}

pub trait IntCounterVecHelper {
    type IntType;

    fn inc_with(&'static self, labels: &[&str]);

    fn inc_with_by(&'static self, labels: &[&str], by: Self::IntType);
}

impl IntCounterVecHelper for IntCounterVec {
    type IntType = u64;

    fn inc_with(&'static self, labels: &[&str]) {
        self.with_label_values(labels).inc()
    }

    fn inc_with_by(&'static self, labels: &[&str], v: Self::IntType) {
        self.with_label_values(labels).inc_by(v)
    }
}

pub trait IntCounterHelper {
    type IntType;

    fn inc(&'static self);

    fn inc_by(&'static self, v: Self::IntType);
}

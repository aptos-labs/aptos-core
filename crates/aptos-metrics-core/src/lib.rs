// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

// Re-export counter types from prometheus crate
pub use prometheus::{
    exponential_buckets, gather, register_counter, register_gauge, register_histogram,
    register_histogram_vec, register_int_counter, register_int_counter_vec, register_int_gauge,
    register_int_gauge_vec, Counter, Encoder, Gauge, Histogram, HistogramTimer, HistogramVec,
    IntCounter, IntCounterVec, IntGauge, IntGaugeVec, TextEncoder,
};

pub mod const_metric;
pub mod op_counters;

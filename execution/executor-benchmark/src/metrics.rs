// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use velor_metrics_core::{
    exponential_buckets, register_histogram_vec, register_int_counter_vec, HistogramVec,
    IntCounterVec,
};
use once_cell::sync::Lazy;

pub static TIMER: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "velor_executor_benchmark_timer_seconds",
        "Various timers for performance analysis.",
        &["name"],
        exponential_buckets(/*start=*/ 1e-9, /*factor=*/ 2.0, /*count=*/ 32).unwrap(),
    )
    .unwrap()
});

pub static NUM_TXNS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_executor_benchmark_num_txns",
        "# of transactions received by each stage.",
        &["stage"]
    )
    .unwrap()
});

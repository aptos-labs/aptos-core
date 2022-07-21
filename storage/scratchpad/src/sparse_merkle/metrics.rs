// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use aptos_metrics_core::{
    exponential_buckets, register_histogram_vec, register_int_gauge, HistogramVec, IntGauge,
};
use once_cell::sync::Lazy;

pub static OLDEST_GENERATION: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_scratchpad_smt_oldest_generation",
        "Generation value on the oldest ancestor, after fetched."
    )
    .unwrap()
});

pub static LATEST_GENERATION: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_scratchpad_smt_latest_generation",
        "Generation value on newly spawned SMT."
    )
    .unwrap()
});

pub static TIMER: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_scratchpad_smt_timer_seconds",
        "Various timers for performance analysis.",
        &["name"],
        exponential_buckets(/*start=*/ 1e-6, /*factor=*/ 2.0, /*count=*/ 22).unwrap(),
    )
    .unwrap()
});

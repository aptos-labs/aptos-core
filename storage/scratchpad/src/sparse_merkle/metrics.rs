// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use diem_metrics::{register_histogram_vec, register_int_gauge, HistogramVec, IntGauge};
use once_cell::sync::Lazy;

pub static OLDEST_GENERATION: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "diem_scratchpad_smt_oldest_generation",
        "Generation value on the oldest ancestor, after fetched."
    )
    .unwrap()
});

pub static LATEST_GENERATION: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "diem_scratchpad_smt_latest_generation",
        "Generation value on newly spawned SMT."
    )
    .unwrap()
});

pub static TIMER: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "diem_scratchpad_smt_timer_seconds",
        "Various timers for performance analysis.",
        &["name"]
    )
    .unwrap()
});

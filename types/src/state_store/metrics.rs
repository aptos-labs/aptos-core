// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{
    exponential_buckets, register_histogram_vec, register_int_counter_vec, HistogramVec,
    IntCounterVec,
};
use once_cell::sync::Lazy;

pub static STATE_KEY_COUNTERS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_state_key_counters",
        "Aptos storage state key counters",
        &["key_type", "event"]
    )
    .unwrap()
});

pub static STATE_KEY_TIMER: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_state_key_timer",
        "Various timers for performance analysis.",
        &["key_type", "event"],
        exponential_buckets(/*start=*/ 1e-6, /*factor=*/ 2.0, /*count=*/ 22).unwrap(),
    )
    .unwrap()
});

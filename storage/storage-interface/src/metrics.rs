// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use aptos_metrics_core::{
    HistogramVec, IntCounterVec, exponential_buckets, register_histogram_vec,
    register_int_counter_vec,
};
use once_cell::sync::Lazy;

pub(crate) static TIMER: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_storage_interface_timer_seconds",
        "Various timers for performance analysis.",
        &["name"],
        exponential_buckets(
            /*start=*/ 1e-6, /*factor=*/ 2.0, /*count=*/ 22
        )
        .unwrap(),
    )
    .unwrap()
});

pub static COUNTER: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        // metric name
        "aptos_storage_interface_counter",
        // metric description
        "Various counters for storage-interface.",
        // metric labels (dimensions)
        &["name"],
    )
    .unwrap()
});

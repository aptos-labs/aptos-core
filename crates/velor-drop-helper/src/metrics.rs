// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_metrics_core::{
    exponential_buckets, register_histogram_vec, register_int_gauge_vec, HistogramVec, IntGaugeVec,
};
use once_cell::sync::Lazy;

pub static TIMER: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "velor_drop_helper_timer_seconds",
        "Various timers for performance analysis.",
        &["helper_name", "name"],
        exponential_buckets(/*start=*/ 1e-9, /*factor=*/ 2.0, /*count=*/ 32).unwrap(),
    )
    .unwrap()
});

pub static GAUGE: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "velor_drop_helper_gauges",
        "Various gauges to help debugging.",
        &["helper_name", "name"],
    )
    .unwrap()
});

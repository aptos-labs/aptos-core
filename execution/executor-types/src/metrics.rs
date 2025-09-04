// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_metrics_core::{exponential_buckets, register_histogram_vec, HistogramVec};
use once_cell::sync::Lazy;

pub static TIMER: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        // metric name
        "velor_executor_types_timer",
        // metric description
        "The time spent in seconds.",
        &["name"],
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
    )
    .unwrap()
});

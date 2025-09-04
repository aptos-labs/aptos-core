// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{HistogramVec, exponential_buckets, register_histogram_vec};
use once_cell::sync::Lazy;

pub static TIMER: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        // metric name
        "aptos_executor_types_timer",
        // metric description
        "The time spent in seconds.",
        &["name"],
        exponential_buckets(
            /*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20
        )
        .unwrap(),
    )
    .unwrap()
});

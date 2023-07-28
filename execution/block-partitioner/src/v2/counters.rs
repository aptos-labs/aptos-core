// Copyright © Aptos Foundation

use once_cell::sync::Lazy;
use aptos_metrics_core::{HistogramVec, register_histogram_vec, exponential_buckets};

pub static MISC_TIMERS_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        // metric name
        "aptos_block_partitioner_v2_misc_timers_seconds",
        // metric description
        "The time spent in seconds of miscellaneous phases of block partitioner v2.",
        &["name"],
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
    )
        .unwrap()
});

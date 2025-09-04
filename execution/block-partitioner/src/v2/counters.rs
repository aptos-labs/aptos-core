// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_metrics_core::{
    exponential_buckets, register_histogram, register_histogram_vec, Histogram, HistogramVec,
};
use once_cell::sync::Lazy;

pub static BLOCK_PARTITIONING_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "velor_block_partitioning_seconds",
        // metric description
        "The total time spent in seconds of block partitioning in the sharded block partitioner.",
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
    )
    .unwrap()
});

pub static MISC_TIMERS_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        // metric name
        "velor_block_partitioner_v2_misc_timers_seconds",
        // metric description
        "The time spent in seconds of miscellaneous phases of block partitioner v2.",
        &["name"],
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
    )
    .unwrap()
});

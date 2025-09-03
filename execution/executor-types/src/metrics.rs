// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{exponential_buckets, make_thread_local_histogram_vec};

make_thread_local_histogram_vec!(
    pub,
    TIMER,
    // metric name
    "aptos_executor_types_timer",
    // metric description
    "The time spent in seconds.",
    &["name"],
    exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
);

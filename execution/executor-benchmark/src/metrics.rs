// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use aptos_metrics_core::{
    exponential_buckets, make_thread_local_histogram_vec, make_thread_local_int_counter_vec,
};

make_thread_local_histogram_vec!(
    pub,
    TIMER,
    "aptos_executor_benchmark_timer_seconds",
    "Various timers for performance analysis.",
    &["name"],
    exponential_buckets(/*start=*/ 1e-9, /*factor=*/ 2.0, /*count=*/ 32).unwrap(),
);

make_thread_local_int_counter_vec!(
    pub,
    NUM_TXNS,
    "aptos_executor_benchmark_num_txns",
    "# of transactions received by each stage.",
    &["stage"]
);

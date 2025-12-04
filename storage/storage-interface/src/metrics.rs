// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![forbid(unsafe_code)]

use aptos_metrics_core::{
    exponential_buckets, make_thread_local_histogram_vec, make_thread_local_int_counter_vec,
};

make_thread_local_histogram_vec!(
    pub(crate),
    TIMER,
    "aptos_storage_interface_timer_seconds",
    "Various timers for performance analysis.",
    &["name"],
    exponential_buckets(/*start=*/ 1e-6, /*factor=*/ 2.0, /*count=*/ 22).unwrap(),
);

make_thread_local_int_counter_vec!(
    pub(crate),
    COUNTER,
    "aptos_storage_interface_counter",
    "Various counters for storage-interface.",
    &["name"],
);

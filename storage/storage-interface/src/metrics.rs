#![forbid(unsafe_code)] // Copyright (c) Aptos Foundation
                        // Copyright (c) Aptos Foundation
                        // SPDX-License-Identifier: Innovation-Enabling Source Code License

// SPDX-License-Identifier: Innovation-Enabling Source Code License

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

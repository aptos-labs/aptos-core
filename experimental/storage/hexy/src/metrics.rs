#![forbid(unsafe_code)] // Copyright (c) Aptos Foundation
                        // Copyright (c) Aptos Foundation
                        // SPDX-License-Identifier: Innovation-Enabling Source Code License

// SPDX-License-Identifier: Innovation-Enabling Source Code License

use aptos_metrics_core::{exponential_buckets, register_histogram_vec, HistogramVec};
use once_cell::sync::Lazy;

pub static TIMER: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_hexy_timer_seconds",
        "Various timers for performance analysis.",
        &["name"],
        exponential_buckets(/*start=*/ 1e-6, /*factor=*/ 2.0, /*count=*/ 22).unwrap(),
    )
    .unwrap()
});

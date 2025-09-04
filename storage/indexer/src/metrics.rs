// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{HistogramVec, exponential_buckets, register_histogram_vec};
use once_cell::sync::Lazy;

pub static TIMER: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_internal_indexer_timer_seconds",
        "Various timers for performance analysis.",
        &["name"],
        exponential_buckets(
            /*start=*/ 1e-9, /*factor=*/ 2.0, /*count=*/ 32
        )
        .unwrap(),
    )
    .unwrap()
});

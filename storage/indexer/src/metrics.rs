// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_metrics_core::{
    exponential_buckets, register_histogram_vec, register_int_gauge, HistogramVec, IntGauge,
};
use once_cell::sync::Lazy;

pub static TIMER: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_internal_indexer_timer_seconds",
        "Various timers for performance analysis.",
        &["name"],
        exponential_buckets(/*start=*/ 1e-9, /*factor=*/ 2.0, /*count=*/ 32).unwrap(),
    )
    .unwrap()
});

pub static INDEXER_DB_LATENCY: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_internal_indexer_latency",
        "The latency between main db update and data written to indexer db"
    )
    .unwrap()
});

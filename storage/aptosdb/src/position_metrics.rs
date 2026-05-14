// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_metrics_core::{
    exponential_buckets, register_histogram, register_int_counter, register_int_counter_vec,
    Histogram, IntCounter, IntCounterVec,
};
use once_cell::sync::Lazy;

pub static POSITION_WRITES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_position_writes_total",
        "Position writes applied by NativeStateCommitter",
        &["kind"]
    )
    .unwrap()
});

pub static POSITION_COLD_LOAD_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "aptos_position_cold_load_seconds",
        "Wall-clock duration of the position_db startup scan",
        exponential_buckets(0.1, 2.0, 12).unwrap()
    )
    .unwrap()
});

pub static POSITION_PRUNE_ROWS: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_position_prune_rows_total",
        "Number of stale rows pruned from position_db"
    )
    .unwrap()
});

pub static POSITION_DECODE_ERRORS: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_position_decode_errors_total",
        "Number of position payloads that failed to decode"
    )
    .unwrap()
});

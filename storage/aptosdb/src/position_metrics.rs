// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Prometheus metrics for the native-position subsystem.
//!
//! Exposed under the `aptos_position_*` namespace. Callers increment /
//! observe these at the appropriate sites — `NativeStateCommitter` for
//! writes, the startup loader for scan duration, the pruner for prune
//! counts.

use aptos_metrics_core::{
    exponential_buckets, register_histogram, register_int_counter, register_int_counter_vec,
    register_int_gauge, Histogram, IntCounter, IntCounterVec, IntGauge,
};
use once_cell::sync::Lazy;

/// Position writes committed, labelled by kind (create / update /
/// delete).
pub static POSITION_WRITES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_position_writes_total",
        "Position writes applied by NativeStateCommitter",
        &["kind"]
    )
    .unwrap()
});

/// Current resident Position count.
pub static POSITION_IN_MEMORY_COUNT: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_position_in_memory_count",
        "Number of Positions currently resident in the in-memory store"
    )
    .unwrap()
});

/// Cold-load scan duration (seconds). One observation per node open.
pub static POSITION_COLD_LOAD_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "aptos_position_cold_load_seconds",
        "Wall-clock duration of the position_db startup scan",
        exponential_buckets(0.1, 2.0, 12).unwrap()
    )
    .unwrap()
});

/// Pruner work per cycle: rows deleted.
pub static POSITION_PRUNE_ROWS: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_position_prune_rows_total",
        "Number of stale rows pruned from position_db"
    )
    .unwrap()
});

/// Number of stored Position payloads that failed to decode through
/// `NativePosition::deserialize`. Bumped at every reader / scanner
/// site that today silently skips corrupt entries. A non-zero value
/// means at least one row has either been corrupted at rest or was
/// written with an out-of-band codec, and the affected entries are
/// invisible to off-Move consumers (validator-side scanner, RPC).
pub static POSITION_DECODE_ERRORS: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_position_decode_errors_total",
        "Number of position payloads that failed to decode"
    )
    .unwrap()
});

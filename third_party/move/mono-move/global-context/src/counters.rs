// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::version::BlockIndex;
use once_cell::sync::Lazy;
use prometheus::{register_int_counter, register_int_gauge, IntCounter, IntGauge};

static GLOBAL_ARENA_ALLOCATED_BYTES: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "global_arena_allocated_bytes",
        "Total bytes allocated in global arena used for interning"
    )
    .unwrap()
});

pub(crate) fn set_global_arena_allocated_bytes(bytes: usize) {
    GLOBAL_ARENA_ALLOCATED_BYTES.set(bytes as i64);
}

static GLOBAL_ARENA_RESET_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "global_arena_reset_count",
        "Total number of global arena resets"
    )
    .unwrap()
});

pub(crate) fn inc_global_arena_reset_count() {
    GLOBAL_ARENA_RESET_COUNT.inc();
}

static INTERNED_EXECUTABLE_ID_COUNT: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "global_arena_interned_executable_id_count",
        "Number of interned executable IDs"
    )
    .unwrap()
});

pub(crate) fn set_interned_executable_id_count(count: usize) {
    INTERNED_EXECUTABLE_ID_COUNT.set(count as i64);
}

static EXECUTABLE_ID_INTERNER_CACHE_MISSES: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "global_arena_executable_id_interner_cache_misses",
        "Total executable ID cache misses since last maintenance"
    )
    .unwrap()
});

pub(crate) fn log_executable_id_interner_cache_miss() {
    EXECUTABLE_ID_INTERNER_CACHE_MISSES.inc();
}

pub(crate) fn reset_executable_id_interner_cache_miss() {
    EXECUTABLE_ID_INTERNER_CACHE_MISSES.reset();
}

static INTERNED_IDENTIFIER_COUNT: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "global_arena_interned_identifier_count",
        "Number of interned identifiers (function or struct names)"
    )
    .unwrap()
});

pub(crate) fn set_interned_identifier_count(count: usize) {
    INTERNED_IDENTIFIER_COUNT.set(count as i64);
}

static IDENTIFIER_INTERNER_CACHE_MISSES: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "global_arena_identifier_interner_cache_misses",
        "Total identifier cache misses since last maintenance"
    )
    .unwrap()
});

pub(crate) fn log_identifier_interner_cache_miss() {
    IDENTIFIER_INTERNER_CACHE_MISSES.inc();
}

pub(crate) fn reset_identifier_interner_cache_miss() {
    IDENTIFIER_INTERNER_CACHE_MISSES.reset();
}

static INTERNED_TYPE_COUNT: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "global_arena_interned_type_count",
        "Number of interned types"
    )
    .unwrap()
});

pub(crate) fn set_interned_type_count(count: usize) {
    INTERNED_TYPE_COUNT.set(count as i64);
}

static TYPE_INTERNER_CACHE_MISSES: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "global_arena_type_interner_cache_misses",
        "Total type cache misses since last maintenance"
    )
    .unwrap()
});

pub(crate) fn log_type_interner_cache_miss() {
    TYPE_INTERNER_CACHE_MISSES.inc();
}

pub(crate) fn reset_type_interner_cache_miss() {
    TYPE_INTERNER_CACHE_MISSES.reset();
}

static INTERNED_TYPE_LIST_COUNT: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "global_arena_interned_type_list_count",
        "Number of interned type lists"
    )
    .unwrap()
});

pub(crate) fn set_interned_type_list_count(count: usize) {
    INTERNED_TYPE_LIST_COUNT.set(count as i64);
}

static TYPE_LIST_INTERNER_CACHE_MISSES: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "global_arena_type_list_interner_cache_misses",
        "Total type list cache misses since last maintenance"
    )
    .unwrap()
});

pub(crate) fn log_type_list_interner_cache_miss() {
    TYPE_LIST_INTERNER_CACHE_MISSES.inc();
}

pub(crate) fn reset_type_list_interner_cache_miss() {
    TYPE_LIST_INTERNER_CACHE_MISSES.reset();
}

static EXECUTABLES_PROMOTED: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "executable_cache_executables_promoted",
        "Number of executables promoted from cold to hot in last maintenance"
    )
    .unwrap()
});

pub(crate) fn set_executables_promoted(count: usize) {
    EXECUTABLES_PROMOTED.set(count as i64);
}

static EXECUTABLES_FREED: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "executable_cache_executables_freed",
        "Number of executables freed in last maintenance"
    )
    .unwrap()
});

pub(crate) fn set_executables_freed(count: usize) {
    EXECUTABLES_FREED.set(count as i64);
}

static CURRENT_EPOCH: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "executable_cache_current_epoch",
        "Current epoch of the executable cache"
    )
    .unwrap()
});

pub(crate) fn set_block_idx(idx: BlockIndex) {
    CURRENT_EPOCH.set(idx as i64);
}

static MONOMORPHIZED_FUNCTION_COUNT: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "executable_cache_monomorphized_function_count",
        "Total cached monomorphized functions across all live executables"
    )
    .unwrap()
});

pub(crate) fn set_monomorphized_function_count(count: usize) {
    MONOMORPHIZED_FUNCTION_COUNT.set(count as i64);
}

static MONOMORPHIZED_FUNCTION_CACHE_MISSES: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "executable_cache_monomorphized_function_cache_misses",
        "Total monomorphizations performed (cache misses) since last full flush"
    )
    .unwrap()
});

pub(crate) fn log_monomorphized_function_cache_miss() {
    MONOMORPHIZED_FUNCTION_CACHE_MISSES.inc();
}

pub(crate) fn reset_monomorphized_function_cache_misses() {
    MONOMORPHIZED_FUNCTION_CACHE_MISSES.reset();
}

static MONOMORPHIZED_FUNCTIONS_EVICTED: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "executable_cache_monomorphized_functions_evicted",
        "Monomorphized functions evicted by LRU in last maintenance"
    )
    .unwrap()
});

pub(crate) fn set_monomorphized_functions_evicted(count: usize) {
    MONOMORPHIZED_FUNCTIONS_EVICTED.set(count as i64);
}

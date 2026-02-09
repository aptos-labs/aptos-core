// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use once_cell::sync::Lazy;
use prometheus::{register_int_counter, register_int_gauge, IntCounter, IntGauge};

static INTERNER_ARENA_ALLOCATED_BYTES: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "global_context_arena_allocated_bytes",
        "Total bytes allocated in arena used for interning"
    )
    .unwrap()
});

static INTERNED_EXECUTABLE_ID_COUNT: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "global_context_interned_executable_id_count",
        "Number of interned executable IDs"
    )
    .unwrap()
});

static INTERNED_IDENTIFIER_COUNT: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "global_context_interned_identifier_count",
        "Number of interned identifiers (function names, etc.)"
    )
    .unwrap()
});

static EXECUTABLE_ID_INTERNER_CACHE_MISSES: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "global_context_executable_id_interner_cache_misses",
        "Total executable ID cache misses since last maintenance"
    )
    .unwrap()
});

static IDENTIFIER_INTERNER_CACHE_MISSES: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "global_context_identifier_interner_cache_misses",
        "Total identifier cache misses since last maintenance"
    )
    .unwrap()
});

static INTERNER_FLUSH_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "global_context_interner_flush_count",
        "Total number of flush operations"
    )
    .unwrap()
});

pub(crate) fn log_executable_id_interner_cache_miss() {
    EXECUTABLE_ID_INTERNER_CACHE_MISSES.inc();
}

pub(crate) fn reset_executable_id_interner_cache_miss() {
    EXECUTABLE_ID_INTERNER_CACHE_MISSES.reset();
}

pub(crate) fn log_identifier_interner_cache_miss() {
    IDENTIFIER_INTERNER_CACHE_MISSES.inc();
}

pub(crate) fn reset_identifier_interner_cache_miss() {
    IDENTIFIER_INTERNER_CACHE_MISSES.reset();
}

pub(crate) fn inc_interner_flush_count() {
    INTERNER_FLUSH_COUNT.inc();
}

pub(crate) fn set_interner_arena_allocated_bytes(bytes: usize) {
    INTERNER_ARENA_ALLOCATED_BYTES.set(bytes as i64);
}

pub(crate) fn set_interned_executable_id_count(count: usize) {
    INTERNED_EXECUTABLE_ID_COUNT.set(count as i64);
}

pub(crate) fn set_interned_identifier_count(count: usize) {
    INTERNED_IDENTIFIER_COUNT.set(count as i64);
}

static TYPE_INTERNER_CACHE_MISSES: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "global_context_type_interner_cache_misses",
        "Total type cache misses since last maintenance"
    )
    .unwrap()
});

static TYPE_LIST_INTERNER_CACHE_MISSES: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "global_context_type_list_interner_cache_misses",
        "Total type list cache misses since last maintenance"
    )
    .unwrap()
});

static INTERNED_TYPE_COUNT: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "global_context_interned_type_count",
        "Number of interned types"
    )
    .unwrap()
});

static INTERNED_TYPE_LIST_COUNT: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "global_context_interned_type_list_count",
        "Number of interned type lists"
    )
    .unwrap()
});

pub(crate) fn log_type_interner_cache_miss() {
    TYPE_INTERNER_CACHE_MISSES.inc();
}

pub(crate) fn reset_type_interner_cache_miss() {
    TYPE_INTERNER_CACHE_MISSES.reset();
}

pub(crate) fn log_type_list_interner_cache_miss() {
    TYPE_LIST_INTERNER_CACHE_MISSES.inc();
}

pub(crate) fn reset_type_list_interner_cache_miss() {
    TYPE_LIST_INTERNER_CACHE_MISSES.reset();
}

pub(crate) fn set_interned_type_count(count: usize) {
    INTERNED_TYPE_COUNT.set(count as i64);
}

pub(crate) fn set_interned_type_list_count(count: usize) {
    INTERNED_TYPE_LIST_COUNT.set(count as i64);
}
